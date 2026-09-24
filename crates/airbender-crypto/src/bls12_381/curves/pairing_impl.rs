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

impl Bls12_381 {
    /// The product of the Miller functions `f_|u|` of the pairs, with the accumulator of the
    /// first pair started at `initial` instead of `1` and multiplied by `initial` at the set
    /// bits of `|u|`, so that the result is `initial^|u| f`: the accumulator goes through the
    /// double-and-add of the exponent, and the residue witness pairing check
    /// (`crate::residue_witness`) gets `d^|u|` almost for free (the squarings are the loop's
    /// own). Unlike `multi_miller_loop` the result is not conjugated for the negative seed: the
    /// conjugation changes it by an `r`-th residue only, which the check is invariant to, and
    /// which the final exponentiation would remove.
    ///
    /// The prepared `G2` points are borrowed: precomputed lines are read in place.
    pub fn multi_miller_loop_with_initial(
        initial: &Fq12,
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        Self::miller_loop_impl(Some(initial), a, b)
    }

    /// The Miller loop over borrowed prepared `G2` points: the same as the `Pairing` trait's
    /// `multi_miller_loop` (conjugated for the negative seed), which converts its points to
    /// the prepared form by value, but with precomputed lines read in place.
    pub fn multi_miller_loop_prepared(
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        let mut result = Self::miller_loop_impl(None, a, b);
        if Config::X_IS_NEGATIVE {
            fp12_cyclotomic_inverse_in_place(&mut result);
        }
        result
    }

    fn miller_loop_impl(
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
                        Self::ell(&mut f, ell_coeffs.next().unwrap(), &p.0);
                        if i {
                            Self::ell(&mut f, ell_coeffs.next().unwrap(), &p.0);
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

    /// Evaluates the line function at point p.
    fn ell(f: &mut Fq12, coeffs: &EllCoeff<Config>, p: &G1Affine) {
        match Config::TWIST_TYPE {
            TwistType::M => {
                fp2_tmp!(c2 = &coeffs.2);
                fp2_mul_by_fp(c2, &p.y);
                fp2_tmp!(c1 = &coeffs.1);
                fp2_mul_by_fp(c1, &p.x);
                fp12_mul_by_014(f, &coeffs.0, c1, c2);
            }
            TwistType::D => {
                fp2_tmp!(c0 = &coeffs.0);
                fp2_mul_by_fp(c0, &p.y);
                fp2_tmp!(c1 = &coeffs.1);
                fp2_mul_by_fp(c1, &p.x);
                fp12_mul_by_034(f, c0, c1, &coeffs.2);
            }
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
        ark_ec::pairing::MillerLoopOutput(Self::multi_miller_loop_prepared(
            a,
            b.into_iter().map(Into::<G2PreparedNoAlloc>::into),
        ))
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

    #[test]
    fn compute_x_naf() {
        let t = ark_ff::biginteger::arithmetic::find_naf(Config::X);
        let t: Vec<_> = t.into_iter().rev().collect();
        dbg!(t);
    }
}
