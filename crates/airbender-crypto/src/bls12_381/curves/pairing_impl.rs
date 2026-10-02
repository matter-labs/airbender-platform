use super::*;
use crate::bls12_381::{Fq12, Fq2};
use crate::extension_tower::*;
use ark_ec::bls12::g2::EllCoeff;
use ark_ec::pairing::Pairing;
use ark_ec::pairing::PairingOutput;
use ark_ec::short_weierstrass::SWCurveConfig;
use ark_ec::AffineRepr;
use ark_ec::CurveGroup;
use ark_ff::BitIteratorBE;
use ark_ff::Field;
use ark_ff::One;
use ark_serialize::CanonicalDeserialize;
use ark_serialize::CanonicalSerialize;
use core::borrow::Borrow;
use core::mem::MaybeUninit;

/// The pairs whose Miller loops run with one shared accumulator (one squaring per step for all of
/// them): up to this many `G1` points and `G2` items, as the caller passes them, are held on the
/// stack at a time (prepared points passed by value are ~26 KB each); every further chunk costs
/// one loop's squarings and one multiplication
const MILLER_LOOP_CHUNK: usize = 4;

/// The chunk of `Pairing::multi_miller_loop`, whose items are the prepared points themselves
/// (~26 KB each, all of a chunk on the stack at once): guests have 16-32 MB stacks, the host the
/// thread stacks of its embedding
#[cfg(target_arch = "riscv32")]
const OWNED_MILLER_LOOP_CHUNK: usize = 4;
#[cfg(not(target_arch = "riscv32"))]
const OWNED_MILLER_LOOP_CHUNK: usize = 2;

impl Bls12_381 {
    /// The product of the Miller functions `f_|u|` of the pairs, with one accumulator per chunk
    /// of pairs, the first one started at `initial` instead of `1` and multiplied by `initial` at
    /// the set bits of `|u|`, so that the result is `initial^|u| f`: the accumulator goes through
    /// the double-and-add of the exponent, and the residue witness pairing check
    /// (`crate::residue_witness`) gets `d^|u|` almost for free (the squarings are the loop's
    /// own). Unlike `multi_miller_loop` the result is not conjugated for the negative seed: the
    /// conjugation changes it by an `r`-th residue only, which the check is invariant to, and
    /// which the final exponentiation would remove.
    ///
    /// The prepared `G2` points are borrowed: precomputed lines are read in place (the items are
    /// held and borrowed as by `multi_miller_loop_prepared`).
    pub fn multi_miller_loop_with_initial(
        initial: &Fq12,
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        Self::miller_loop_impl::<MILLER_LOOP_CHUNK, _>(Some(initial), a, b)
    }

    /// The Miller loop over borrowed prepared `G2` points: the same as the `Pairing` trait's
    /// `multi_miller_loop` (conjugated for the negative seed), which converts its points to
    /// the prepared form by value, but with precomputed lines read in place (a chunk of items is
    /// held at once and each item is borrowed twice; items passed by value are held on the
    /// stack, `Pairing::multi_miller_loop` takes fewer of them at a time on the host).
    pub fn multi_miller_loop_prepared(
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        Self::miller_loop_conjugated::<MILLER_LOOP_CHUNK, _>(a, b)
    }

    /// The Miller loop of the pairs, `K` at a time, conjugated for the negative seed: the output
    /// of `multi_miller_loop` (the one place of the conjugation)
    fn miller_loop_conjugated<const K: usize, Q: Borrow<G2PreparedNoAlloc>>(
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = Q>,
    ) -> Fq12 {
        let mut result = Self::miller_loop_impl::<K, _>(None, a, b);
        if Config::X_IS_NEGATIVE {
            fp12_cyclotomic_inverse_in_place(&mut result);
        }
        result
    }

    /// The product of the Miller functions of the pairs without a point at infinity, `K` pairs
    /// at a time with one accumulator (`miller_loop_chunk`), the first chunk started at
    /// `initial`; not conjugated. Not inlined: the chunk buffer (the prepared points by value on
    /// the `Pairing::multi_miller_loop` path) then lives in this frame only while the loop runs,
    /// never in a caller's frame
    #[inline(never)]
    fn miller_loop_impl<const K: usize, Q: Borrow<G2PreparedNoAlloc>>(
        initial: Option<&Fq12>,
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = Q>,
    ) -> Fq12 {
        const { assert!(K > 0) };
        let mut a = a.into_iter();
        let mut b = b.into_iter();
        let mut result = Fq12::one();
        let mut initial = initial;
        let mut started = false;
        let mut exhausted = false;
        // the next active pairs, in input order: a pair with a point at infinity is a factor of
        // one and is skipped (the slots are initialized once: `K` prepared points by value on the
        // `Pairing::multi_miller_loop` path)
        let mut g1 = [G1Affine::identity(); K];
        let mut g2: [Option<Q>; K] = [const { None }; K];
        while !exhausted {
            let mut len = 0;
            while len < K {
                let (p, q) = match (a.next(), b.next()) {
                    (Some(p), Some(q)) => (p, q),
                    (None, None) => {
                        exhausted = true;
                        break;
                    }
                    _ => panic!("Caller must check input lengths"),
                };
                let p: <Self as Pairing>::G1Prepared = p.into();
                // the item is only looked at for a finite `p`, as the Miller loop of each pair
                // on its own did; a skipped item is dropped before the next pair is pulled
                if p.is_zero() || Borrow::<G2PreparedNoAlloc>::borrow(&q).is_zero() {
                    continue;
                }
                g1[len] = p.0;
                g2[len] = Some(q);
                len += 1;
            }
            if len == 0 {
                // only when the input is exhausted
                continue;
            }
            // the prepared points of the chunk (each item borrowed once more), the unused entries
            // padded with the last one
            fn prepared<Q: Borrow<G2PreparedNoAlloc>>(item: &Option<Q>) -> &G2PreparedNoAlloc {
                item.as_ref().expect("a gathered pair").borrow()
            }
            let mut lines = [prepared(&g2[len - 1]); K];
            for (line, item) in lines.iter_mut().zip(&g2[..len - 1]) {
                *line = prepared(item);
            }
            if started {
                let mut f = Fq12::one();
                Self::miller_loop_chunk(&mut f, None, &g1[..len], &lines[..len]);
                fp12_mul_assign(&mut result, &f);
            } else {
                Self::miller_loop_chunk(&mut result, initial.take(), &g1[..len], &lines[..len]);
                started = true;
            }
            // the items of the chunk are dropped before the next pair is pulled
            for item in &mut g2[..len] {
                *item = None;
            }
        }
        // no pair: the accumulator is the initial value to the power of the loop count all
        // the same
        match initial {
            Some(initial) => initial.pow(Config::X),
            None => result,
        }
    }

    /// The Miller loop of the pairs `(g1[j], g2[j])` with one accumulator `f`, which is
    /// overwritten: started at `initial` (at `1`), squared once per bit of `|u|` after the leading
    /// one (when started at `1`, the first squaring, of `1`, is skipped), multiplied by the line
    /// evaluations of every pair, and by `initial` at the set bits: the product of the `f_|u|` of
    /// the pairs on their own, `initial` counted once. Not conjugated. The lines of every
    /// prepared point follow the same schedule, so one index serves all pairs.
    #[inline(never)]
    fn miller_loop_chunk(
        f: &mut Fq12,
        initial: Option<&Fq12>,
        g1: &[G1Affine],
        g2: &[&G2PreparedNoAlloc],
    ) {
        debug_assert_eq!(g1.len(), g2.len());
        fp12_assign(f, initial.unwrap_or(&Fq12::ONE));
        let mut k = 0;
        for (step, bit) in BitIteratorBE::without_leading_zeros(Config::X)
            .skip(1)
            .enumerate()
        {
            // the first squaring is of `1` unless the accumulator was started elsewhere
            if step != 0 || initial.is_some() {
                fp12_square_in_place(f);
            }
            for (p, q) in g1.iter().zip(g2) {
                Self::ell_prepared(f, &q.ell_coeffs[k], p, q.normalized);
            }
            k += 1;
            if bit {
                for (p, q) in g1.iter().zip(g2) {
                    Self::ell_prepared(f, &q.ell_coeffs[k], p, q.normalized);
                }
                k += 1;
                if let Some(initial) = initial {
                    fp12_mul_assign(f, initial);
                }
            }
        }
        debug_assert_eq!(k, BLS12_381_NUM_ELL_COEFFS);
    }

    /// The Miller loop over all the pairs at once: the accumulator is squared once per bit of
    /// the seed, whatever the number of pairs. The result is the product of the Miller loop
    /// outputs of the pairs (not conjugated for the negative seed, as
    /// [`Self::multi_miller_loop_with_initial`]), with the lines of a normalized prepared
    /// point (`G2PreparedNoAlloc::normalized`) taken as they are: up to a factor in `Fq2`,
    /// which the pairing check is invariant to.
    ///
    /// With `initial`, it is the start of the accumulator, which is also multiplied by it at
    /// the set bits of the seed, as in [`Self::multi_miller_loop_with_initial`].
    ///
    /// A pair with a `G1` point at infinity contributes nothing (`e(O, Q) = 1`); the `G2`
    /// points must not be points at infinity.
    pub fn multi_miller_loop_shared<'a>(
        initial: Option<&Fq12>,
        pairs: impl Iterator<Item = (&'a G1Affine, &'a G2PreparedNoAlloc)> + Clone,
    ) -> Fq12 {
        let mut f = match initial {
            Some(initial) => *initial,
            None => Fq12::one(),
        };
        let lines = |f: &mut Fq12, index: usize| {
            for (p, q) in pairs.clone() {
                debug_assert!(!q.infinity);
                if p.infinity {
                    continue;
                }
                Self::ell_prepared(f, &q.ell_coeffs[index], p, q.normalized);
            }
        };
        let mut index = 0;
        for (i, bit) in BitIteratorBE::without_leading_zeros(Config::X)
            .skip(1)
            .enumerate()
        {
            // the first squaring is of `1` unless the accumulator was started elsewhere
            if i != 0 || initial.is_some() {
                fp12_square_in_place(&mut f);
            }
            lines(&mut f, index);
            index += 1;
            if bit {
                lines(&mut f, index);
                index += 1;
                if let Some(initial) = initial {
                    fp12_mul_assign(&mut f, initial);
                }
            }
        }
        debug_assert_eq!(index, BLS12_381_NUM_ELL_COEFFS);
        f
    }

    /// Evaluates a line of a prepared point at `p`, normalized or not
    fn ell_prepared(f: &mut Fq12, coeffs: &EllCoeff<Config>, p: &G1Affine, normalized: bool) {
        const { assert!(matches!(Config::TWIST_TYPE, TwistType::M)) };
        fp2_tmp!(c2 = &coeffs.2);
        fp2_mul_by_fp(c2, &p.y);
        fp2_tmp!(c1 = &coeffs.1);
        fp2_mul_by_fp(c1, &p.x);
        if normalized {
            fp12_mul_by_114(f, c1, c2);
        } else {
            fp12_mul_by_014(f, &coeffs.0, c1, c2);
        }
    }

    /// `f = f^X` for the (signed) curve parameter, `f` in the cyclotomic subgroup
    fn exp_by_x_in_place(f: &mut Fq12) {
        Self::spec_cyclotomic_exp_by_x_inplace(f);
        if Config::X_IS_NEGATIVE {
            fp12_cyclotomic_inverse_in_place(f);
        }
    }

    fn spec_cyclotomic_exp_by_x_inplace(f: &mut Fq12) {
        use ark_ff::Zero;
        if f.is_zero() {
            return;
        }
        Self::fast_exp_loop_with_naf(f, Self::X_NAF.iter().copied());
    }

    const X_NAF: [i8; 65] = [
        1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0,
    ];

    /// `exp_loop` taken from arkworks, each value updated in place
    fn fast_exp_loop_with_naf<I: Iterator<Item = i8>>(f: &mut Fq12, e: I) {
        fp12_tmp!(self_inverse = &*f);
        fp12_cyclotomic_inverse_in_place(self_inverse);
        let mut res = Fq12::one();
        let mut found_nonzero = false;
        for value in e {
            if found_nonzero {
                fp12_cyclotomic_square_in_place(&mut res);
            }
            if value != 0 {
                found_nonzero = true;
                if value > 0 {
                    fp12_mul_assign(&mut res, &*f);
                } else {
                    fp12_mul_assign(&mut res, self_inverse);
                }
            }
        }
        fp12_assign(f, &res);
    }
}

impl Pairing for Bls12_381 {
    type BaseField = Fq;
    type ScalarField = crate::bls12_381::Fr;
    type G1 = G1Projective;
    type G1Affine = G1Affine;
    type G1Prepared = ark_ec::bls12::G1Prepared<Config>;
    type G2 = G2Projective;
    type G2Affine = G2Affine;
    type G2Prepared = G2PreparedNoAlloc;
    type TargetField = Fq12;

    fn multi_miller_loop(
        a: impl IntoIterator<Item = impl Into<Self::G1Prepared>>,
        b: impl IntoIterator<Item = impl Into<Self::G2Prepared>>,
    ) -> ark_ec::pairing::MillerLoopOutput<Self> {
        ark_ec::pairing::MillerLoopOutput(
            Self::miller_loop_conjugated::<OWNED_MILLER_LOOP_CHUNK, _>(
                a,
                b.into_iter().map(Into::<G2PreparedNoAlloc>::into),
            ),
        )
    }

    fn final_exponentiation(
        f: ark_ec::pairing::MillerLoopOutput<Self>,
    ) -> Option<PairingOutput<Self>> {
        Self::final_exponentiation_with_inverse(&f.0, Field::inverse).map(PairingOutput)
    }
}

impl Bls12_381 {
    /// The final exponentiation of `f`, with the one field inversion it needs (`f^-1`, for the
    /// easy part) supplied by `inverse`: a prover can take it from a hint that it then checks
    /// with one multiplication. `inverse` returns `None` only for a zero `f`, and so does this.
    pub fn final_exponentiation_with_inverse(
        f: &Fq12,
        inverse: impl FnOnce(&Fq12) -> Option<Fq12>,
    ) -> Option<Fq12> {
        // Computing the final exponentiation following
        // https://eprint.iacr.org/2020/875
        // Adapted from the implementation in https://github.com/ConsenSys/gurvy/pull/29
        // The same sequence as the arkworks implementation, each value updated in place.
        // f1 = conj(f) = f^(p^6)
        fp12_tmp!(f1 = f);
        fp12_cyclotomic_inverse_in_place(f1);
        inverse(f).map(|f2| {
            // r = f^(p^6 - 1)
            fp12_tmp!(r = f1);
            fp12_mul_assign(r, &f2);
            // r = f^((p^6 - 1)(p^2 + 1)) = r^(p^2) * r
            fp12_tmp!(r_p2 = r);
            r_p2.frobenius_map_in_place(2);
            fp12_mul_assign(r, r_p2);
            // Hard part of the final exponentiation:
            // t[0].CyclotomicSquare(&result)
            fp12_tmp!(y0 = r);
            fp12_cyclotomic_square_in_place(y0);
            // t[1].Expt(&result)
            fp12_tmp!(y1 = r);
            Self::exp_by_x_in_place(y1);
            // t[2].InverseUnitary(&result)
            fp12_tmp!(y2 = r);
            fp12_cyclotomic_inverse_in_place(y2);
            // t[1].Mul(&t[1], &t[2])
            fp12_mul_assign(y1, y2);
            // t[2].Expt(&t[1])
            fp12_assign(y2, y1);
            Self::exp_by_x_in_place(y2);
            // t[1].InverseUnitary(&t[1])
            fp12_cyclotomic_inverse_in_place(y1);
            // t[1].Mul(&t[1], &t[2])
            fp12_mul_assign(y1, y2);
            // t[2].Expt(&t[1])
            fp12_assign(y2, y1);
            Self::exp_by_x_in_place(y2);
            // t[1].Frobenius(&t[1])
            y1.frobenius_map_in_place(1);
            // t[1].Mul(&t[1], &t[2])
            fp12_mul_assign(y1, y2);
            // result.Mul(&result, &t[0])
            fp12_mul_assign(r, y0);
            // t[0].Expt(&t[1])
            fp12_assign(y0, y1);
            Self::exp_by_x_in_place(y0);
            // t[2].Expt(&t[0])
            fp12_assign(y2, y0);
            Self::exp_by_x_in_place(y2);
            // t[0].FrobeniusSquare(&t[1])
            fp12_assign(y0, y1);
            y0.frobenius_map_in_place(2);
            // t[1].InverseUnitary(&t[1])
            fp12_cyclotomic_inverse_in_place(y1);
            // t[1].Mul(&t[1], &t[2])
            fp12_mul_assign(y1, y2);
            // t[1].Mul(&t[1], &t[0])
            fp12_mul_assign(y1, y0);
            // result.Mul(&result, &t[1])
            fp12_mul_assign(r, y1);
            *r
        })
    }
}

impl From<G2Affine> for G2PreparedNoAlloc {
    fn from(q: G2Affine) -> Self {
        if q.infinity {
            // coeffs should not be used
            Self {
                ell_coeffs: [Default::default(); BLS12_381_NUM_ELL_COEFFS],
                infinity: true,
                normalized: false,
            }
        } else {
            use ark_ff::AdditiveGroup;
            let two_inv = Fq::one().double().inverse().unwrap();
            let mut i = 0;
            let mut ell_coeffs: [MaybeUninit<EllCoeff<Config>>; BLS12_381_NUM_ELL_COEFFS] =
                [const { MaybeUninit::uninit() }; BLS12_381_NUM_ELL_COEFFS];
            let mut r = G2HomProjective {
                x: q.x,
                y: q.y,
                z: Fq2::one(),
            };
            for bit in BitIteratorBE::new(Config::X).skip(1) {
                r.double_in_place(&two_inv, &mut ell_coeffs[i]);
                i += 1;
                if bit {
                    r.add_in_place(&q, &mut ell_coeffs[i]);
                    i += 1;
                }
            }
            assert_eq!(i, ell_coeffs.len());
            Self {
                ell_coeffs: unsafe { ell_coeffs.map(|el| el.assume_init()) },
                infinity: false,
                normalized: false,
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct G2HomProjective {
    x: Fq2,
    y: Fq2,
    z: Fq2,
}

/// Writes a line coefficient triple into its slot, component by component
#[inline(always)]
fn write_coeff(out: &mut MaybeUninit<EllCoeff<Config>>, a: &Fq2, b: &Fq2, c: &Fq2) {
    // SAFETY: the three fields are written before the value is used
    unsafe {
        let p = out.as_mut_ptr();
        fp2_init(
            &mut *(core::ptr::addr_of_mut!((*p).0) as *mut MaybeUninit<Fq2>),
            a,
        );
        fp2_init(
            &mut *(core::ptr::addr_of_mut!((*p).1) as *mut MaybeUninit<Fq2>),
            b,
        );
        fp2_init(
            &mut *(core::ptr::addr_of_mut!((*p).2) as *mut MaybeUninit<Fq2>),
            c,
        );
    }
}

impl G2HomProjective {
    /// Doubles the point and writes the line coefficients; the formulas of the arkworks
    /// implementation for homogeneous projective coordinates, evaluated in place.
    fn double_in_place(&mut self, two_inv: &Fq, out: &mut MaybeUninit<EllCoeff<Config>>) {
        // a = x y / 2
        fp2_tmp!(a = &self.x);
        fp2_mul_assign(a, &self.y);
        fp2_mul_by_fp(a, two_inv);
        // b = y^2, c = z^2
        fp2_tmp!(b = &self.y);
        fp2_square_in_place(b);
        fp2_tmp!(c = &self.z);
        fp2_square_in_place(c);
        // e = 3 B c, f = 3 e
        fp2_tmp!(e = c);
        fp2_double_in_place(e);
        fp2_add_assign(e, c);
        fp2_mul_assign(e, &<Config as Bls12Config>::G2Config::COEFF_B);
        fp2_tmp!(f = e);
        fp2_double_in_place(f);
        fp2_add_assign(f, e);
        // g = (b + f) / 2
        fp2_tmp!(g = b);
        fp2_add_assign(g, f);
        fp2_mul_by_fp(g, two_inv);
        // h = (y + z)^2 - (b + c)
        fp2_tmp!(h = &self.y);
        fp2_add_assign(h, &self.z);
        fp2_square_in_place(h);
        fp2_sub_assign(h, b);
        fp2_sub_assign(h, c);
        // i = e - b, j = x^2
        fp2_tmp!(i = e);
        fp2_sub_assign(i, b);
        fp2_tmp!(j = &self.x);
        fp2_square_in_place(j);
        // z = b h
        fp2_assign(&mut self.z, b);
        fp2_mul_assign(&mut self.z, h);
        // x = a (b - f)
        fp2_sub_assign(b, f);
        fp2_assign(&mut self.x, a);
        fp2_mul_assign(&mut self.x, b);
        // y = g^2 - 3 e^2
        fp2_square_in_place(g);
        fp2_square_in_place(e);
        fp2_tmp!(e3 = e);
        fp2_double_in_place(e3);
        fp2_add_assign(e3, e);
        fp2_sub_assign(g, e3);
        fp2_assign(&mut self.y, g);
        // 3 j
        fp2_tmp!(j3 = j);
        fp2_double_in_place(j3);
        fp2_add_assign(j3, j);
        fp2_neg_in_place(h);
        match Config::TWIST_TYPE {
            TwistType::M => write_coeff(out, i, j3, h),
            TwistType::D => write_coeff(out, h, j3, i),
        }
    }

    /// Adds `q` and writes the line coefficients, in place as `double_in_place`
    fn add_in_place(&mut self, q: &G2Affine, out: &mut MaybeUninit<EllCoeff<Config>>) {
        // theta = y - q.y z, lambda = x - q.x z
        fp2_tmp!(theta = &q.y);
        fp2_mul_assign(theta, &self.z);
        fp2_neg_in_place(theta);
        fp2_add_assign(theta, &self.y);
        fp2_tmp!(lambda = &q.x);
        fp2_mul_assign(lambda, &self.z);
        fp2_neg_in_place(lambda);
        fp2_add_assign(lambda, &self.x);
        // c = theta^2, d = lambda^2, e = lambda d, f = z c, g = x d
        fp2_tmp!(c = theta);
        fp2_square_in_place(c);
        fp2_tmp!(d = lambda);
        fp2_square_in_place(d);
        fp2_tmp!(e = lambda);
        fp2_mul_assign(e, d);
        fp2_tmp!(f = &self.z);
        fp2_mul_assign(f, c);
        fp2_tmp!(g = &self.x);
        fp2_mul_assign(g, d);
        // h = e + f - 2 g
        fp2_tmp!(h = e);
        fp2_add_assign(h, f);
        fp2_sub_assign(h, g);
        fp2_sub_assign(h, g);
        // x = lambda h
        fp2_assign(&mut self.x, lambda);
        fp2_mul_assign(&mut self.x, h);
        // y = theta (g - h) - e y
        fp2_sub_assign(g, h);
        fp2_mul_assign(g, theta);
        fp2_mul_assign(&mut self.y, e);
        fp2_neg_in_place(&mut self.y);
        fp2_add_assign(&mut self.y, g);
        // z = z e
        fp2_mul_assign(&mut self.z, e);
        // j = theta q.x - lambda q.y
        fp2_tmp!(j = theta);
        fp2_mul_assign(j, &q.x);
        fp2_tmp!(lq = lambda);
        fp2_mul_assign(lq, &q.y);
        fp2_sub_assign(j, lq);
        fp2_neg_in_place(theta);
        match Config::TWIST_TYPE {
            TwistType::M => write_coeff(out, j, theta, lambda),
            TwistType::D => write_coeff(out, lambda, theta, j),
        }
    }
}

impl Default for G2PreparedNoAlloc {
    fn default() -> Self {
        Self::from(G2Affine::generator())
    }
}

impl From<G2Projective> for G2PreparedNoAlloc {
    fn from(q: G2Projective) -> Self {
        q.into_affine().into()
    }
}

impl<'a> From<&'a G2Affine> for G2PreparedNoAlloc {
    fn from(other: &'a G2Affine) -> Self {
        (*other).into()
    }
}

impl<'a> From<&'a G2Projective> for G2PreparedNoAlloc {
    fn from(q: &'a G2Projective) -> Self {
        q.into_affine().into()
    }
}

impl G2PreparedNoAlloc {
    pub fn is_zero(&self) -> bool {
        self.infinity
    }
}

pub const BLS12_381_NUM_ELL_COEFFS: usize = const {
    let num_bits_except_top_one = (u64::BITS - Config::X[0].leading_zeros() - 1) as usize;
    let mut result = num_bits_except_top_one;
    let num_non_zero_bits = Config::X[0].count_ones();
    // all non-zero bits except top one
    result += num_non_zero_bits as usize - 1;

    result
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct G2PreparedNoAlloc {
    pub ell_coeffs: [ark_ec::bls12::g2::EllCoeff<Config>; BLS12_381_NUM_ELL_COEFFS],
    pub infinity: bool,
    /// The lines are divided by their first coefficient, which is then one and is not stored
    /// (the coefficient of a line is a factor in `Fq2`, which the pairing check is invariant
    /// to). Only the lines of `multi_miller_loop_shared` may be normalized.
    pub normalized: bool,
}

impl CanonicalSerialize for G2PreparedNoAlloc {
    fn serialize_with_mode<W: ark_serialize::Write>(
        &self,
        _writer: W,
        _compress: ark_serialize::Compress,
    ) -> Result<(), ark_serialize::SerializationError> {
        unimplemented!("not supported");
    }

    fn serialized_size(&self, _compress: ark_serialize::Compress) -> usize {
        unimplemented!("not supported");
    }
}

impl ark_serialize::Valid for G2PreparedNoAlloc {
    fn check(&self) -> Result<(), ark_serialize::SerializationError> {
        unimplemented!("not supported");
    }
}

impl CanonicalDeserialize for G2PreparedNoAlloc {
    fn deserialize_with_mode<R: ark_serialize::Read>(
        _reader: R,
        _compress: ark_serialize::Compress,
        _validate: ark_serialize::Validate,
    ) -> Result<Self, ark_serialize::SerializationError> {
        unimplemented!("not supported");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bls12_381::{Fq6, Fr};
    use crate::extension_tower::tests::{rand_fp, rand_fp12, rand_fp2};
    use ark_ec::pairing::MillerLoopOutput;
    use ark_ec::PrimeGroup;
    use ark_ff::{BigInteger, PrimeField, UniformRand, Zero};
    use ark_std::rand::Rng;
    use ark_std::test_rng;
    use std::cell::RefCell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn compute_x_naf() {
        let t = ark_ff::biginteger::arithmetic::find_naf(Config::X);
        let t: Vec<_> = t.into_iter().rev().collect();
        dbg!(t);
    }

    impl Bls12_381 {
        /// The Miller loop of each pair on its own, the accumulators multiplied (the
        /// implementation before the pairs shared one accumulator, verbatim): the reference
        fn miller_loop_impl_per_pair(
            initial: Option<&Fq12>,
            a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
            b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
        ) -> Fq12 {
            let mut a = a.into_iter();
            let mut b = b.into_iter();
            let mut result = Fq12::one();
            let mut initial = initial;
            loop {
                match (a.next(), b.next()) {
                    (Some(p), Some(q)) => {
                        let p: <Self as Pairing>::G1Prepared = p.into();
                        if p.is_zero() {
                            continue;
                        }
                        let q: &G2PreparedNoAlloc = q.borrow();
                        if q.is_zero() {
                            continue;
                        }
                        let base = initial.take();
                        let mut f = match base {
                            Some(initial) => *initial,
                            None => Fq12::one(),
                        };
                        let mut ell_coeffs = q.ell_coeffs.iter();
                        for i in BitIteratorBE::without_leading_zeros(Config::X).skip(1) {
                            fp12_square_in_place(&mut f);
                            Self::ell_prepared(
                                &mut f,
                                ell_coeffs.next().unwrap(),
                                &p.0,
                                q.normalized,
                            );
                            if i {
                                Self::ell_prepared(
                                    &mut f,
                                    ell_coeffs.next().unwrap(),
                                    &p.0,
                                    q.normalized,
                                );
                                if let Some(initial) = base {
                                    fp12_mul_assign(&mut f, initial);
                                }
                            }
                        }
                        fp12_mul_assign(&mut result, &f);
                    }
                    (None, None) => break,
                    _ => {
                        panic!("Caller must check input lengths");
                    }
                }
            }
            // no pair: the accumulator is the initial value to the power of the loop count all
            // the same
            match initial {
                Some(initial) => initial.pow(Config::X),
                None => result,
            }
        }
    }

    fn conjugated(f: &Fq12) -> Fq12 {
        let mut f = *f;
        fp12_cyclotomic_inverse_in_place(&mut f);
        f
    }

    /// The other representative of `a` in the redundant representation `[0, 2p)`
    fn shifted(a: &Fq) -> Fq {
        let mut limbs = a.0;
        if limbs < Fq::MODULUS {
            limbs.add_with_carry(&Fq::MODULUS);
        } else {
            limbs.sub_with_borrow(&Fq::MODULUS);
        }
        Fq::new_unchecked(limbs)
    }

    fn shifted_fq2(a: &Fq2) -> Fq2 {
        Fq2::new(shifted(&a.c0), shifted(&a.c1))
    }

    fn shifted_fq12(a: &Fq12) -> Fq12 {
        let fq6 = |a: &Fq6| Fq6::new(shifted_fq2(&a.c0), shifted_fq2(&a.c1), shifted_fq2(&a.c2));
        Fq12::new(fq6(&a.c0), fq6(&a.c1))
    }

    /// The raw limbs of the twelve coefficients, as they are in memory
    fn limbs(f: &Fq12) -> Vec<[u64; 8]> {
        let fq6 = |a: &Fq6| [a.c0, a.c1, a.c2];
        [fq6(&f.c0), fq6(&f.c1)]
            .into_iter()
            .flatten()
            .flat_map(|a| [a.c0.0 .0, a.c1.0 .0])
            .collect()
    }

    /// How many results of the new implementation were also the same representative as the old
    /// one's: informational only (printed, not asserted), as only the field values must be equal
    #[derive(Default)]
    struct Stats {
        results: usize,
        same_limbs: usize,
    }

    impl Stats {
        fn check(&mut self, new: &Fq12, old: &Fq12, what: &str) {
            assert_eq!(new, old, "{what}");
            self.results += 1;
            self.same_limbs += (limbs(new) == limbs(old)) as usize;
        }
    }

    fn random_g1(rng: &mut impl Rng) -> G1Affine {
        (G1Projective::generator() * Fr::rand(rng)).into_affine()
    }

    fn random_g2(rng: &mut impl Rng) -> G2Affine {
        (G2Projective::generator() * Fr::rand(rng)).into_affine()
    }

    /// A finite prepared point with arbitrary line coefficients, normalized or not: the identity
    /// of the shared accumulator with the per-pair loops is ring algebra, it does not need actual
    /// lines
    fn synthetic_prepared(rng: &mut impl Rng) -> G2PreparedNoAlloc {
        G2PreparedNoAlloc {
            ell_coeffs: core::array::from_fn(|_| (rand_fp2(rng), rand_fp2(rng), rand_fp2(rng))),
            infinity: false,
            normalized: rng.gen(),
        }
    }

    /// The points the test inputs are drawn from: curve points, off-curve `G1` points, points
    /// in the other representative, prepared points with actual lines and with arbitrary
    /// coefficients
    struct Pool {
        g1: Vec<G1Affine>,
        g2: Vec<G2PreparedNoAlloc>,
    }

    impl Pool {
        fn new(rng: &mut impl Rng) -> Self {
            let mut g1: Vec<_> = (0..5).map(|_| random_g1(rng)).collect();
            g1.push(G1Affine::new_unchecked(rand_fp(rng), rand_fp(rng)));
            g1.push(G1Affine::new_unchecked(
                Fq::new_unchecked(Fq::MODULUS),
                rand_fp(rng),
            ));
            let p = random_g1(rng);
            g1.push(G1Affine::new_unchecked(shifted(&p.x), shifted(&p.y)));
            let mut g2: Vec<_> = (0..3)
                .map(|_| G2PreparedNoAlloc::from(random_g2(rng)))
                .collect();
            for _ in 0..2 {
                g2.push(synthetic_prepared(rng));
            }
            let mut other = G2PreparedNoAlloc::from(random_g2(rng));
            for (c0, c1, c2) in other.ell_coeffs.iter_mut() {
                *c0 = shifted_fq2(c0);
                *c1 = shifted_fq2(c1);
                *c2 = shifted_fq2(c2);
            }
            g2.push(other);
            Self { g1, g2 }
        }

        fn pairs(
            &self,
            slots: &[Slot],
            rng: &mut impl Rng,
        ) -> (Vec<G1Affine>, Vec<G2PreparedNoAlloc>) {
            let zero_g2 = G2PreparedNoAlloc::from(G2Affine::identity());
            slots
                .iter()
                .map(|slot| {
                    let p = self.g1[rng.gen_range(0..self.g1.len())];
                    let q = self.g2[rng.gen_range(0..self.g2.len())].clone();
                    match slot {
                        Slot::Active => (p, q),
                        Slot::ZeroP => (G1Affine::identity(), q),
                        Slot::ZeroQ => (p, zero_g2.clone()),
                        Slot::ZeroBoth => (G1Affine::identity(), zero_g2.clone()),
                    }
                })
                .unzip()
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Slot {
        Active,
        ZeroP,
        ZeroQ,
        ZeroBoth,
    }

    /// Where the pairs with a point at infinity are, for `n` pairs: none, one at the start, the
    /// middle or the end, all, every other one (straddling the chunk boundaries), a whole chunk
    fn patterns(n: usize) -> Vec<Vec<Slot>> {
        use Slot::*;
        let mut candidates = vec![vec![Active; n]];
        if n > 0 {
            for (index, slot) in [(0, ZeroP), (n / 2, ZeroQ), (n - 1, ZeroBoth)] {
                let mut slots = vec![Active; n];
                slots[index] = slot;
                candidates.push(slots);
            }
            candidates.push((0..n).map(|i| [ZeroP, ZeroQ, ZeroBoth][i % 3]).collect());
            candidates.push(
                (0..n)
                    .map(|i| {
                        if i % 2 == 0 {
                            Active
                        } else {
                            [ZeroP, ZeroQ][i / 2 % 2]
                        }
                    })
                    .collect(),
            );
            candidates.push((0..n).map(|i| if i < 4 { ZeroQ } else { Active }).collect());
        }
        let mut patterns = vec![];
        for slots in candidates {
            if !patterns.contains(&slots) {
                patterns.push(slots);
            }
        }
        patterns
    }

    const SIZES: [usize; 8] = [0, 1, 2, 3, 4, 5, 8, 9];

    /// New against old for every chunk size, borrowed and owned items, and through the entry
    /// points with the chunk sizes of the build
    fn check_case(
        initial: Option<&Fq12>,
        g1: &[G1Affine],
        g2: &[G2PreparedNoAlloc],
        stats: &mut Stats,
    ) {
        let old = Bls12_381::miller_loop_impl_per_pair(initial, g1.iter().copied(), g2.iter());
        let a = || g1.iter().copied();
        let mut check = |new: Fq12, what: &str| stats.check(&new, &old, what);
        check(
            Bls12_381::miller_loop_impl::<1, _>(initial, a(), g2.iter()),
            "K = 1",
        );
        check(
            Bls12_381::miller_loop_impl::<2, _>(initial, a(), g2.iter()),
            "K = 2",
        );
        check(
            Bls12_381::miller_loop_impl::<3, _>(initial, a(), g2.iter()),
            "K = 3",
        );
        check(
            Bls12_381::miller_loop_impl::<4, _>(initial, a(), g2.iter()),
            "K = 4",
        );
        let owned = || g2.iter().cloned();
        check(
            Bls12_381::miller_loop_impl::<1, _>(initial, a(), owned()),
            "owned, K = 1",
        );
        check(
            Bls12_381::miller_loop_impl::<2, _>(initial, a(), owned()),
            "owned, K = 2",
        );
        check(
            Bls12_381::miller_loop_impl::<3, _>(initial, a(), owned()),
            "owned, K = 3",
        );
        check(
            Bls12_381::miller_loop_impl::<4, _>(initial, a(), owned()),
            "owned, K = 4",
        );
        match initial {
            Some(initial) => check(
                Bls12_381::multi_miller_loop_with_initial(initial, a(), g2.iter()),
                "with initial",
            ),
            None => {
                let prepared = Bls12_381::multi_miller_loop_prepared(a(), g2.iter());
                check(conjugated(&prepared), "prepared");
                let f = Bls12_381::multi_miller_loop(a(), owned()).0;
                check(conjugated(&f), "Pairing");
            }
        }
    }

    /// Every size and pattern of points at infinity, with the initial value `initial` makes of
    /// a random element
    fn check_all_sizes(initial: impl Fn(Fq12) -> Option<Fq12>) {
        let mut rng = test_rng();
        let pool = Pool::new(&mut rng);
        let mut stats = Stats::default();
        for n in SIZES {
            for slots in patterns(n) {
                let (g1, g2) = pool.pairs(&slots, &mut rng);
                let initial = initial(rand_fp12(&mut rng));
                check_case(initial.as_ref(), &g1, &g2, &mut stats);
            }
        }
        println!(
            "{} results, {} with the old limbs",
            stats.results, stats.same_limbs
        );
    }

    #[test]
    fn shared_accumulator_matches_per_pair_loops() {
        check_all_sizes(|_| None);
    }

    #[test]
    fn shared_accumulator_matches_per_pair_loops_with_initial() {
        check_all_sizes(Some);
    }

    #[test]
    fn shared_accumulator_matches_per_pair_loops_with_special_initial_values() {
        let mut rng = test_rng();
        let pool = Pool::new(&mut rng);
        let mut stats = Stats::default();
        let d: Fq12 = rand_fp12(&mut rng);
        let initials = [Fq12::zero(), Fq12::one(), shifted_fq12(&d)];
        for n in [0, 1, 5] {
            for slots in patterns(n) {
                let (g1, g2) = pool.pairs(&slots, &mut rng);
                for initial in &initials {
                    check_case(Some(initial), &g1, &g2, &mut stats);
                }
            }
        }
        println!(
            "{} results, {} with the old limbs",
            stats.results, stats.same_limbs
        );
    }

    #[test]
    fn zero_lines_give_zero() {
        let mut rng = test_rng();
        let zero = G2PreparedNoAlloc {
            ell_coeffs: [Default::default(); BLS12_381_NUM_ELL_COEFFS],
            infinity: false,
            normalized: false,
        };
        let q = G2PreparedNoAlloc::from(random_g2(&mut rng));
        let g1 = [random_g1(&mut rng), random_g1(&mut rng)];
        for g2 in [[&zero, &q], [&q, &zero]] {
            let old = Bls12_381::miller_loop_impl_per_pair(None, g1, g2);
            let new = Bls12_381::multi_miller_loop_prepared(g1, g2);
            assert!(old.is_zero() && new.is_zero());
            assert!(Bls12_381::final_exponentiation(MillerLoopOutput(new)).is_none());
        }
    }

    #[test]
    fn initial_is_counted_once() {
        let mut rng = test_rng();
        let pool = Pool::new(&mut rng);
        for n in 0..=9 {
            let (g1, g2) = pool.pairs(&vec![Slot::Active; n], &mut rng);
            let d: Fq12 = rand_fp12(&mut rng);
            let l = Bls12_381::multi_miller_loop_with_initial(&d, g1.iter().copied(), g2.iter());
            let f = Bls12_381::multi_miller_loop_prepared(g1.iter().copied(), g2.iter());
            assert_eq!(l, d.pow(Config::X) * conjugated(&f));
        }
    }

    const LENGTHS_MESSAGE: &str = "Caller must check input lengths";

    fn panic_message(f: impl FnOnce() -> Fq12) -> Option<String> {
        let payload = catch_unwind(AssertUnwindSafe(f)).err()?;
        let message = match payload.downcast_ref::<&str>() {
            Some(message) => message.to_string(),
            None => payload.downcast_ref::<String>().unwrap().clone(),
        };
        Some(message)
    }

    #[test]
    fn length_mismatch_panics_as_before() {
        let mut rng = test_rng();
        let pool = Pool::new(&mut rng);
        let d: Fq12 = rand_fp12(&mut rng);
        let initial = Some(&d);
        for n in [0, 3, 4, 5] {
            for (len_a, len_b) in [(n, n + 1), (n + 1, n)] {
                let (g1, g2) = pool.pairs(&vec![Slot::Active; n + 1], &mut rng);
                let (g1, g2) = (&g1[..len_a], &g2[..len_b]);
                let a = || g1.iter().copied();
                let owned = || g2.iter().cloned();
                let old = panic_message(|| Bls12_381::miller_loop_impl_per_pair(initial, a(), g2));
                assert_eq!(old.as_deref(), Some(LENGTHS_MESSAGE));
                let calls: [Box<dyn Fn() -> Fq12>; 13] = [
                    Box::new(|| Bls12_381::miller_loop_impl::<1, _>(initial, a(), g2)),
                    Box::new(|| Bls12_381::miller_loop_impl::<2, _>(initial, a(), g2)),
                    Box::new(|| Bls12_381::miller_loop_impl::<3, _>(initial, a(), g2)),
                    Box::new(|| Bls12_381::miller_loop_impl::<4, _>(initial, a(), g2)),
                    Box::new(|| Bls12_381::miller_loop_impl::<1, _>(None, a(), owned())),
                    Box::new(|| Bls12_381::miller_loop_impl::<2, _>(None, a(), owned())),
                    Box::new(|| Bls12_381::miller_loop_impl::<3, _>(None, a(), owned())),
                    Box::new(|| Bls12_381::miller_loop_impl::<4, _>(None, a(), owned())),
                    Box::new(|| Bls12_381::miller_loop_impl::<4, _>(initial, a(), owned())),
                    Box::new(|| Bls12_381::multi_miller_loop_with_initial(&d, a(), g2)),
                    Box::new(|| Bls12_381::multi_miller_loop_prepared(a(), g2)),
                    Box::new(|| Bls12_381::multi_miller_loop_prepared(a(), owned())),
                    Box::new(|| Bls12_381::multi_miller_loop(a(), owned()).0),
                ];
                for call in calls {
                    assert_eq!(panic_message(call), old, "{len_a} and {len_b} items");
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "Caller must check input lengths")]
    fn more_g1_points_panic() {
        let mut rng = test_rng();
        let q = G2PreparedNoAlloc::from(random_g2(&mut rng));
        let g1 = [random_g1(&mut rng); 6];
        Bls12_381::multi_miller_loop_prepared(g1, [&q; 5]);
    }

    #[test]
    #[should_panic(expected = "Caller must check input lengths")]
    fn more_g2_points_panic() {
        let mut rng = test_rng();
        let q = random_g2(&mut rng);
        let g1 = [random_g1(&mut rng); 5];
        let _ = Bls12_381::multi_miller_loop(g1, [q; 6]);
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Event {
        NextG1(bool),
        NextG2(bool),
        Into,
        IntoG2,
    }

    type Log = Rc<RefCell<Vec<Event>>>;

    /// An iterator that logs its `next` calls
    struct Logged<I> {
        inner: I,
        log: Log,
        event: fn(bool) -> Event,
    }

    impl<I: Iterator> Iterator for Logged<I> {
        type Item = I::Item;

        fn next(&mut self) -> Option<I::Item> {
            let item = self.inner.next();
            self.log.borrow_mut().push((self.event)(item.is_some()));
            item
        }
    }

    /// A `G1` item that logs its conversion
    struct LoggedG1(G1Affine, Log);

    impl From<LoggedG1> for ark_ec::bls12::G1Prepared<Config> {
        fn from(p: LoggedG1) -> Self {
            p.1.borrow_mut().push(Event::Into);
            p.0.into()
        }
    }

    /// A `G2` item that logs its conversion to the prepared form
    struct LoggedG2(G2Affine, Log);

    impl From<LoggedG2> for G2PreparedNoAlloc {
        fn from(q: LoggedG2) -> Self {
            q.1.borrow_mut().push(Event::IntoG2);
            q.0.into()
        }
    }

    /// The sequence of `next` calls and conversions of a Miller loop, the `G2` items made by `g2`
    fn trace<B: Iterator>(
        g1: &[G1Affine],
        g2: impl FnOnce(&Log) -> B,
        miller_loop: impl FnOnce(Logged<std::vec::IntoIter<LoggedG1>>, Logged<B>) -> Fq12,
    ) -> (Vec<Event>, Option<Fq12>) {
        let log = Log::default();
        let a = Logged {
            inner: g1
                .iter()
                .map(|p| LoggedG1(*p, log.clone()))
                .collect::<Vec<_>>()
                .into_iter(),
            log: log.clone(),
            event: Event::NextG1,
        };
        let b = Logged {
            inner: g2(&log),
            log: log.clone(),
            event: Event::NextG2,
        };
        let result = catch_unwind(AssertUnwindSafe(|| miller_loop(a, b))).ok();
        let events = RefCell::borrow(&log).clone();
        (events, result)
    }

    #[test]
    fn items_are_pulled_and_converted_as_before() {
        let mut rng = test_rng();
        let pool = Pool::new(&mut rng);
        let mut cases = vec![];
        for n in SIZES {
            for slots in patterns(n) {
                let (g1, g2) = pool.pairs(&slots, &mut rng);
                cases.push((g1.clone(), g2.clone()));
                // and with one item more on either side
                let (p, q) = pool.pairs(&[Slot::Active], &mut rng);
                cases.push(([&g1[..], &p].concat(), g2.clone()));
                cases.push((g1, [&g2[..], &q].concat()));
            }
        }
        for (g1, g2) in &cases {
            let g2 = |_: &Log| g2.iter();
            let (old, old_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl_per_pair(None, a, b)
            });
            let (new, new_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<4, _>(None, a, b)
            });
            assert_eq!(new, old);
            assert_eq!(new_result, old_result);
            let (new, _) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<3, _>(None, a, b)
            });
            assert_eq!(new, old);
            let (new, _) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<1, _>(None, a, b)
            });
            assert_eq!(new, old);
        }
    }

    #[test]
    fn owned_items_are_pulled_and_converted_as_before() {
        // the `Pairing::multi_miller_loop` path: `G2Affine` items converted to the prepared form
        // by value while they are pulled
        let mut rng = test_rng();
        let points: Vec<_> = (0..4)
            .map(|_| (random_g1(&mut rng), random_g2(&mut rng)))
            .collect();
        let d: Fq12 = rand_fp12(&mut rng);
        let initial = Some(&d);
        let mut cases = vec![];
        for n in [0, 1, 2, 3, 4, 5, 9] {
            for slots in patterns(n) {
                let (g1, g2): (Vec<_>, Vec<_>) = slots
                    .iter()
                    .enumerate()
                    .map(|(i, slot)| {
                        let (p, q) = points[i % points.len()];
                        match slot {
                            Slot::Active => (p, q),
                            Slot::ZeroP => (G1Affine::identity(), q),
                            Slot::ZeroQ => (p, G2Affine::identity()),
                            Slot::ZeroBoth => (G1Affine::identity(), G2Affine::identity()),
                        }
                    })
                    .unzip();
                cases.push((g1.clone(), g2.clone()));
                // and with one item more on either side
                cases.push(([&g1[..], &[points[0].0]].concat(), g2.clone()));
                cases.push((g1, [&g2[..], &[points[1].1]].concat()));
            }
        }
        for (g1, g2) in &cases {
            let g2 = |log: &Log| {
                g2.iter()
                    .map(|q| LoggedG2(*q, log.clone()))
                    .collect::<Vec<_>>()
                    .into_iter()
            };
            let prepared = G2PreparedNoAlloc::from;
            let (old, old_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl_per_pair(None, a, b.map(prepared))
            });
            let (new, new_result) = trace(g1, g2, |a, b| Bls12_381::multi_miller_loop(a, b).0);
            assert_eq!(new, old);
            assert_eq!(new_result.map(|f| conjugated(&f)), old_result);
            let (new, new_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<2, _>(None, a, b.map(prepared))
            });
            assert_eq!(new, old);
            assert_eq!(new_result, old_result);
            let (new, _) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<4, _>(None, a, b.map(prepared))
            });
            assert_eq!(new, old);
            // with an initial value
            let (old, old_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl_per_pair(initial, a, b.map(prepared))
            });
            let (new, new_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<2, _>(initial, a, b.map(prepared))
            });
            assert_eq!(new, old);
            assert_eq!(new_result, old_result);
            let (new, new_result) = trace(g1, g2, |a, b| {
                Bls12_381::miller_loop_impl::<4, _>(initial, a, b.map(prepared))
            });
            assert_eq!(new, old);
            assert_eq!(new_result, old_result);
        }
    }

    fn to_ark_fq(a: &Fq) -> ark_bls12_381::Fq {
        let limbs = a.into_bigint().0;
        assert_eq!(limbs[6..], [0, 0]);
        ark_bls12_381::Fq::from_bigint(ark_ff::BigInt(limbs[..6].try_into().unwrap())).unwrap()
    }

    fn to_ark_fq2(a: &Fq2) -> ark_bls12_381::Fq2 {
        ark_bls12_381::Fq2::new(to_ark_fq(&a.c0), to_ark_fq(&a.c1))
    }

    fn to_ark_fq12(a: &Fq12) -> ark_bls12_381::Fq12 {
        let fq6 = |a: &Fq6| {
            ark_bls12_381::Fq6::new(to_ark_fq2(&a.c0), to_ark_fq2(&a.c1), to_ark_fq2(&a.c2))
        };
        ark_bls12_381::Fq12::new(fq6(&a.c0), fq6(&a.c1))
    }

    fn to_ark_g1(p: &G1Affine) -> ark_bls12_381::G1Affine {
        match p.infinity {
            true => ark_bls12_381::G1Affine::identity(),
            false => ark_bls12_381::G1Affine::new(to_ark_fq(&p.x), to_ark_fq(&p.y)),
        }
    }

    fn to_ark_g2(q: &G2Affine) -> ark_bls12_381::G2Affine {
        match q.infinity {
            true => ark_bls12_381::G2Affine::identity(),
            false => ark_bls12_381::G2Affine::new(to_ark_fq2(&q.x), to_ark_fq2(&q.y)),
        }
    }

    /// Our Miller loop (all entry points) and pairing against arkworks'
    fn check_against_arkworks(g1: &[G1Affine], g2: &[G2Affine]) -> bool {
        let ark_g1: Vec<_> = g1.iter().map(to_ark_g1).collect();
        let ark_g2: Vec<_> = g2.iter().map(to_ark_g2).collect();
        let expected = ark_bls12_381::Bls12_381::multi_miller_loop(&ark_g1, &ark_g2).0;
        let prepared: Vec<_> = g2.iter().map(G2PreparedNoAlloc::from).collect();
        let f = Bls12_381::multi_miller_loop(g1, g2).0;
        assert_eq!(to_ark_fq12(&f), expected);
        let borrowed = Bls12_381::multi_miller_loop_prepared(g1, &prepared);
        assert_eq!(to_ark_fq12(&borrowed), expected);
        let d: Fq12 = rand_fp12(&mut test_rng());
        let l = Bls12_381::multi_miller_loop_with_initial(&d, g1, &prepared);
        assert_eq!(l, d.pow(Config::X) * conjugated(&f));
        let expected = ark_bls12_381::Bls12_381::multi_pairing(&ark_g1, &ark_g2).0;
        let gt = Bls12_381::multi_pairing(g1, g2).0;
        assert_eq!(to_ark_fq12(&gt), expected);
        let gt = Bls12_381::final_exponentiation(MillerLoopOutput(borrowed))
            .unwrap()
            .0;
        assert_eq!(to_ark_fq12(&gt), expected);
        gt.is_one()
    }

    #[test]
    fn matches_arkworks() {
        let mut rng = test_rng();
        for n in 0..=9 {
            let mut g1 = vec![];
            let mut g2 = vec![];
            for _ in 0..n {
                g1.push(match rng.gen_range(0..6) {
                    0 => G1Affine::identity(),
                    _ => random_g1(&mut rng),
                });
                g2.push(match rng.gen_range(0..6) {
                    0 => G2Affine::identity(),
                    _ => random_g2(&mut rng),
                });
            }
            let is_one = check_against_arkworks(&g1, &g2);
            assert_eq!(
                is_one,
                g1.iter().zip(&g2).all(|(p, q)| p.infinity || q.infinity)
            );
        }
    }

    #[test]
    fn identity_products_match_arkworks() {
        let mut rng = test_rng();
        for m in 1..=4 {
            // `e(P, Q) e(-P, Q) = 1` for `m` points, with a pair at infinity among them
            let mut pairs = vec![(G1Affine::identity(), random_g2(&mut rng))];
            for _ in 0..m {
                let (p, q) = (random_g1(&mut rng), random_g2(&mut rng));
                pairs.push((p, q));
                pairs.insert(rng.gen_range(0..pairs.len()), (-p, q));
            }
            // `e(sP, Q) = e(P, sQ)`
            let (p, q, s) = (random_g1(&mut rng), random_g2(&mut rng), Fr::rand(&mut rng));
            pairs.push(((p * s).into_affine(), q));
            pairs.push((p, -(q * s).into_affine()));
            let (g1, g2): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
            assert!(check_against_arkworks(&g1, &g2));
        }
    }
}
