use super::*;
use crate::bn254::Fq12;
use crate::extension_tower::*;
use ark_ec::bn::g2::EllCoeff;
use ark_ec::pairing::Pairing;
use ark_ec::pairing::PairingOutput;
use ark_ec::short_weierstrass::SWCurveConfig;
use ark_ec::AffineRepr;
use ark_ec::CurveGroup;
use ark_ff::One;
use ark_ff::{AdditiveGroup, Field};
use ark_serialize::CanonicalDeserialize;
use ark_serialize::CanonicalSerialize;
use core::borrow::Borrow;
use core::mem::MaybeUninit;

impl Bn254 {
    /// The Miller loop with the accumulator of the first pair started at `initial` instead of
    /// `1`, and multiplied by `initial` (`initial_inverse`) at the `1` (`-1`) digits of the loop
    /// count, so that the result is `initial^(6x+2) f` for the plain Miller loop output `f`: the
    /// accumulator goes through the double-and-add of the exponent. This is how the residue
    /// witness pairing check (`crate::residue_witness`) gets `d^(6x+2)` almost for free (the
    /// squarings are the loop's own). `initial_inverse` must be the inverse of `initial`.
    ///
    /// The prepared `G2` points are borrowed: precomputed lines are read in place.
    pub fn multi_miller_loop_with_initial(
        initial: &Fq12,
        initial_inverse: &Fq12,
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        Self::miller_loop_impl(Some((initial, initial_inverse)), a, b)
    }

    /// The Miller loop over borrowed prepared `G2` points: the same as the `Pairing` trait's
    /// `multi_miller_loop`, which converts its points to the prepared form by value, but with
    /// precomputed lines read in place.
    pub fn multi_miller_loop_prepared(
        a: impl IntoIterator<Item = impl Into<<Self as Pairing>::G1Prepared>>,
        b: impl IntoIterator<Item = impl Borrow<G2PreparedNoAlloc>>,
    ) -> Fq12 {
        Self::miller_loop_impl(None, a, b)
    }

    fn miller_loop_impl(
        initial: Option<(&Fq12, &Fq12)>,
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
                        Some((initial, _)) => *initial,
                        None => Fq12::one(),
                    };
                    let mut ell_coeffs = q.ell_coeffs.iter();

                    for i in (1..Config::ATE_LOOP_COUNT.len()).rev() {
                        // the first squaring is of `1` unless the accumulator was started
                        // elsewhere
                        if i != Config::ATE_LOOP_COUNT.len() - 1 || base.is_some() {
                            fp12_square_in_place(&mut f);
                        }

                        Self::ell(&mut f, ell_coeffs.next().unwrap(), &p.0, q.affine_lines);

                        let bit = Config::ATE_LOOP_COUNT[i - 1];
                        if bit == 1 || bit == -1 {
                            Self::ell(&mut f, &ell_coeffs.next().unwrap(), &p.0, q.affine_lines);
                            if let Some((initial, initial_inverse)) = base {
                                let factor = if bit == 1 { initial } else { initial_inverse };
                                fp12_mul_assign(&mut f, factor);
                            }
                        }
                    }

                    if Config::X_IS_NEGATIVE {
                        fp12_cyclotomic_inverse_in_place(&mut f);
                    }

                    Self::ell(&mut f, ell_coeffs.next().unwrap(), &p.0, q.affine_lines);
                    Self::ell(&mut f, ell_coeffs.next().unwrap(), &p.0, q.affine_lines);

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
            Some((initial, _)) => initial.pow(crate::residue_witness::bn254::SIX_X_PLUS_2),
            None => result,
        }
    }

    /// Evaluates the line function at point p.
    fn ell(f: &mut Fq12, coeffs: &EllCoeff<Config>, p: &G1Affine, affine_lines: bool) {
        match Config::TWIST_TYPE {
            TwistType::M => {
                let mut c1 = coeffs.1;
                let mut c2 = coeffs.2;
                c2.mul_assign_by_fp(&p.y);
                c1.mul_assign_by_fp(&p.x);
                f.mul_by_014(&coeffs.0, &c1, &c2);
            }
            TwistType::D => {
                // an affine line has a first coefficient of one: its evaluation is y_P
                let c0 = if affine_lines {
                    Fq2::new(p.y, Fq::ZERO)
                } else {
                    let mut c0 = coeffs.0;
                    c0.mul_assign_by_fp(&p.y);
                    c0
                };
                fp2_tmp!(c1 = &coeffs.1);
                fp2_mul_by_fp(c1, &p.x);
                fp12_mul_by_034(f, &c0, c1, &coeffs.2);
            }
        }
    }

    fn exp_by_neg_x_in_place(f: &mut Fq12) {
        Self::spec_cyclotomic_exp_by_x_inplace(f);

        if !Config::X_IS_NEGATIVE {
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

    const X_NAF: [i8; 63] = [
        1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1,
        0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1,
        0, 0, 0, 1,
    ];

    /// `exp_loop` taken from arkworks
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

impl Pairing for Bn254 {
    type BaseField = Fq;
    type ScalarField = crate::bn254::Fr;
    type G1 = G1Projective;
    type G1Affine = G1Affine;
    type G1Prepared = ark_ec::bn::G1Prepared<Config>;
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

impl Bn254 {
    /// The final exponentiation of `f`, with the one field inversion it needs (`f^-1`, for the
    /// easy part) supplied by `inverse`: a prover can take it from a hint that it then checks
    /// with one multiplication. `inverse` returns `None` only for a zero `f`, and so does this.
    pub fn final_exponentiation_with_inverse(
        f: &Fq12,
        inverse: impl FnOnce(&Fq12) -> Option<Fq12>,
    ) -> Option<Fq12> {
        // Easy part: result = elt^((q^6-1)*(q^2+1)).
        // Follows, e.g., Beuchat et al page 9, by computing result as follows:
        //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)

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

            // Hard part follows Laura Fuentes-Castaneda et al. "Faster hashing to G2"
            // by computing:
            //
            // result = elt^(q^3 * (12*z^3 + 6z^2 + 4z - 1) +
            //               q^2 * (12*z^3 + 6z^2 + 6z) +
            //               q   * (12*z^3 + 6z^2 + 4z) +
            //               1   * (12*z^3 + 12z^2 + 6z + 1))
            // which equals
            //
            // result = elt^( 2z * ( 6z^2 + 3z + 1 ) * (q^4 - q^2 + 1)/r ).
            //
            // Same sequence as the arkworks implementation, each value updated in place.
            fp12_tmp!(y0 = r);
            Self::exp_by_neg_x_in_place(y0);
            fp12_tmp!(y1 = y0);
            fp12_cyclotomic_square_in_place(y1);
            fp12_tmp!(y2 = y1);
            fp12_cyclotomic_square_in_place(y2);
            fp12_tmp!(y3 = y2);
            fp12_mul_assign(y3, y1);
            fp12_tmp!(y4 = y3);
            Self::exp_by_neg_x_in_place(y4);
            fp12_tmp!(y5 = y4);
            fp12_cyclotomic_square_in_place(y5);
            fp12_tmp!(y6 = y5);
            Self::exp_by_neg_x_in_place(y6);
            fp12_cyclotomic_inverse_in_place(y3);
            fp12_cyclotomic_inverse_in_place(y6);
            // y7 = y6 * y4, y8 = y7 * y3
            let y8 = y6;
            fp12_mul_assign(y8, y4);
            fp12_mul_assign(y8, y3);
            // y9 = y8 * y1
            fp12_tmp!(y9 = y8);
            fp12_mul_assign(y9, y1);
            // y11 = y10 * r, y10 = y8 * y4
            fp12_tmp!(y11 = y8);
            fp12_mul_assign(y11, y4);
            fp12_mul_assign(y11, r);
            // y13 = y12 * y11, y12 = y9^p
            fp12_tmp!(y13 = y9);
            y13.frobenius_map_in_place(1);
            fp12_mul_assign(y13, y11);
            // y14 = y8^(p^2) * y13
            y8.frobenius_map_in_place(2);
            fp12_mul_assign(y8, y13);
            let y14 = y8;
            // y16 = (conj(r) * y9)^(p^3) * y14
            fp12_cyclotomic_inverse_in_place(r);
            fp12_mul_assign(r, y9);
            r.frobenius_map_in_place(3);
            fp12_mul_assign(r, y14);

            *r
        })
    }
}

impl From<G2Affine> for G2PreparedNoAlloc {
    fn from(q: G2Affine) -> Self {
        if q.infinity {
            // coeffs should not be used
            Self {
                ell_coeffs: [Default::default(); BN254_NUM_ELL_COEFFS],
                infinity: true,
                affine_lines: false,
            }
        } else {
            use ark_ff::{AdditiveGroup, One};
            let two_inv = Fq::one().double().inverse().unwrap();
            let mut i = 0;
            let mut ell_coeffs: [core::mem::MaybeUninit<EllCoeff<Config>>; BN254_NUM_ELL_COEFFS] =
                [const { core::mem::MaybeUninit::uninit() }; BN254_NUM_ELL_COEFFS];

            let mut r = G2HomProjective {
                x: q.x,
                y: q.y,
                z: Fq2::one(),
            };

            let neg_q = -q;

            for bit in Config::ATE_LOOP_COUNT.iter().rev().skip(1) {
                r.double_in_place(&two_inv, &mut ell_coeffs[i]);
                i += 1;

                match bit {
                    1 => r.add_in_place(&q, &mut ell_coeffs[i]),
                    -1 => r.add_in_place(&neg_q, &mut ell_coeffs[i]),
                    _ => continue,
                };
                i += 1;
            }

            let q1 = mul_by_char(q);
            let mut q2 = mul_by_char(q1);

            if Config::X_IS_NEGATIVE {
                r.y = -r.y;
            }

            q2.y = -q2.y;

            r.add_in_place(&q1, &mut ell_coeffs[i]);
            i += 1;
            r.add_in_place(&q2, &mut ell_coeffs[i]);
            i += 1;

            assert_eq!(i, ell_coeffs.len());

            Self {
                ell_coeffs: unsafe { ell_coeffs.map(|el| el.assume_init()) },
                infinity: false,
                affine_lines: false,
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
pub(crate) fn write_coeff(out: &mut MaybeUninit<EllCoeff<Config>>, a: &Fq2, b: &Fq2, c: &Fq2) {
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
        fp2_mul_assign(e, &<Config as BnConfig>::G2Config::COEFF_B);
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

pub(crate) fn mul_by_char(r: G2Affine) -> G2Affine {
    // multiply by field characteristic
    use ark_ff::Field;

    let mut s = r;
    s.x.frobenius_map_in_place(1);
    s.x *= &Config::TWIST_MUL_BY_Q_X;
    s.y.frobenius_map_in_place(1);
    s.y *= &Config::TWIST_MUL_BY_Q_Y;

    s
}

pub const BN254_NUM_ELL_COEFFS: usize = const {
    let mut result = 2;

    let mut i = 0;
    while i < Config::ATE_LOOP_COUNT.len() - 1 {
        result += 1;
        if Config::ATE_LOOP_COUNT[i] != 0 {
            result += 1;
        }

        i += 1;
    }

    result
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct G2PreparedNoAlloc {
    /// Stores the coefficients of the line evaluations as calculated in
    /// <https://eprint.iacr.org/2013/722.pdf>
    pub ell_coeffs: [ark_ec::bn::g2::EllCoeff<Config>; BN254_NUM_ELL_COEFFS],
    pub infinity: bool,
    /// The lines were computed in affine coordinates (`g2_affine`): their first coefficient
    /// is one, and the evaluation at `P` takes `y_P` as it is
    pub affine_lines: bool,
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
