use ark_ec::{scalar_mul::glv::GLVConfig, CurveConfig};
use ark_ff::PrimeField;
use ruint::aliases::U512;

// default ark implementation for scalar decomposition uses global allocator, so we need to write our own
pub(crate) trait GLVConfigNoAllocator: GLVConfig
where
    <<Self as CurveConfig>::ScalarField as PrimeField>::BigInt: AsRef<[u64]>,
{
    /// BETA_1 = n22 * 2^512 / modulus
    const BETA_1: (bool, U512);
    /// BETA_2 = -n12 * 2^512 / modulus
    const BETA_2: (bool, U512);

    /// The decomposition of `k` in two half-scalars with their signs (`true` is positive),
    /// below `2^128` each for the supported curves: on 256-bit integers with the delegated
    /// multiplications where the build has the bigint delegation, and with plain 64-bit limb
    /// arithmetic otherwise.
    #[inline(always)]
    fn scalar_decomposition_no_allocator(
        k: Self::ScalarField,
    ) -> ((bool, Self::ScalarField), (bool, Self::ScalarField)) {
        #[cfg(bigint_ops)]
        {
            decompose::<Self, wide::Delegated>(k)
        }
        #[cfg(not(bigint_ops))]
        {
            decompose::<Self, wide::Plain>(k)
        }
    }

    // default implementation from ark for comparison
    #[cfg(test)]
    fn scalar_decomposition_ref(
        k: Self::ScalarField,
    ) -> ((bool, Self::ScalarField), (bool, Self::ScalarField)) {
        use ark_std::ops::{AddAssign, Neg};
        use num_bigint::{BigInt, BigUint, Sign};
        use num_integer::Integer;
        use num_traits::{One, Signed};

        let scalar: BigInt = k.into_bigint().into().into();

        let coeff_bigints: [BigInt; 4] = Self::SCALAR_DECOMP_COEFFS.map(|x| {
            #[allow(clippy::obfuscated_if_else)]
            BigInt::from_biguint(x.0.then_some(Sign::Plus).unwrap_or(Sign::Minus), x.1.into())
        });

        let [n11, n12, n21, n22] = coeff_bigints;

        let r = BigInt::from(<<Self as CurveConfig>::ScalarField>::MODULUS.into());

        // beta = vector([k,0]) * self.curve.N_inv
        // The inverse of N is 1/r * Matrix([[n22, -n12], [-n21, n11]]).
        // so β = (k*n22, -k*n12)/r

        let beta_1 = {
            let (mut div, rem) = (&scalar * &n22).div_rem(&r);
            if (&rem + &rem) > r {
                div.add_assign(BigInt::one());
            }
            div
        };
        let beta_2 = {
            let (mut div, rem) = (&scalar * &n12.clone().neg()).div_rem(&r);
            if (&rem + &rem) > r {
                div.add_assign(BigInt::one());
            }
            div
        };

        // b = vector([int(beta[0]), int(beta[1])]) * self.curve.N
        // b = (β1N11 + β2N21, β1N12 + β2N22) with the signs!
        //   = (b11   + b12  , b21   + b22)   with the signs!

        // b1
        let b11 = &beta_1 * &n11;
        let b12 = &beta_2 * &n21;
        let b1 = b11 + b12;

        // b2
        let b21 = &beta_1 * &n12;
        let b22 = &beta_2 * &n22;
        let b2 = b21 + b22;

        let k1 = &scalar - b1;
        let k1_abs = BigUint::try_from(k1.abs()).unwrap();

        // k2
        let k2 = -b2;
        let k2_abs = BigUint::try_from(k2.abs()).unwrap();

        (
            (
                k1.sign() == Sign::Plus,
                <<Self as CurveConfig>::ScalarField>::from(k1_abs),
            ),
            (
                k2.sign() == Sign::Plus,
                <<Self as CurveConfig>::ScalarField>::from(k2_abs),
            ),
        )
    }
}

/// The 256-bit integers of the decomposition: what it needs of their arithmetic
pub(crate) trait Wide: Copy {
    fn from_limbs(limbs: [u64; 4]) -> Self;
    fn limbs(&self) -> [u64; 4];
    /// The low half of the product
    fn mul_low(&self, rhs: &Self) -> Self;
    /// The high half of the product
    fn mul_high(&self, rhs: &Self) -> Self;
    /// The sum modulo `2^256`, and whether it carried
    fn add(&self, rhs: &Self) -> (Self, bool);
    /// The difference modulo `2^256`, and whether it borrowed
    fn sub(&self, rhs: &Self) -> (Self, bool);
}

pub(crate) mod wide {
    use super::Wide;

    /// Four 64-bit limbs, with the multiplications and the carries of the machine: for the
    /// builds without the bigint delegation (64-bit hosts, the RV64 guest)
    #[cfg(any(not(bigint_ops), test))]
    #[derive(Clone, Copy)]
    pub(crate) struct Plain([u64; 4]);

    #[cfg(any(not(bigint_ops), test))]
    impl Plain {
        /// The 512-bit product, low half and high half
        fn mul_wide(&self, rhs: &Self) -> ([u64; 4], [u64; 4]) {
            let (a, b) = (&self.0, &rhs.0);
            let mut t = [0u64; 8];
            for i in 0..4 {
                let mut carry = 0u64;
                for j in 0..4 {
                    // fits: (2^64 - 1)^2 + 2 (2^64 - 1) = 2^128 - 1
                    let wide = a[i] as u128 * b[j] as u128 + t[i + j] as u128 + carry as u128;
                    t[i + j] = wide as u64;
                    carry = (wide >> 64) as u64;
                }
                t[i + 4] = carry;
            }
            ([t[0], t[1], t[2], t[3]], [t[4], t[5], t[6], t[7]])
        }
    }

    #[cfg(any(not(bigint_ops), test))]
    impl Wide for Plain {
        fn from_limbs(limbs: [u64; 4]) -> Self {
            Self(limbs)
        }

        fn limbs(&self) -> [u64; 4] {
            self.0
        }

        fn mul_low(&self, rhs: &Self) -> Self {
            Self(self.mul_wide(rhs).0)
        }

        fn mul_high(&self, rhs: &Self) -> Self {
            Self(self.mul_wide(rhs).1)
        }

        fn add(&self, rhs: &Self) -> (Self, bool) {
            let mut r = [0u64; 4];
            let mut carry = false;
            for i in 0..4 {
                let (sum, first) = self.0[i].overflowing_add(rhs.0[i]);
                let (sum, second) = sum.overflowing_add(carry as u64);
                r[i] = sum;
                carry = first | second;
            }
            (Self(r), carry)
        }

        fn sub(&self, rhs: &Self) -> (Self, bool) {
            let mut r = [0u64; 4];
            let mut borrow = false;
            for i in 0..4 {
                let (difference, first) = self.0[i].overflowing_sub(rhs.0[i]);
                let (difference, second) = difference.overflowing_sub(borrow as u64);
                r[i] = difference;
                borrow = first | second;
            }
            (Self(r), borrow)
        }
    }

    /// A 256-bit operand of the bigint delegation, with the alignment it requires (the host
    /// `BigInt` has none): for the builds with the delegation, real or emulated. Never on
    /// `riscv64`.
    #[cfg(all(not(target_arch = "riscv64"), any(bigint_ops, test)))]
    #[derive(Clone, Copy)]
    #[repr(C, align(32))]
    pub(crate) struct Delegated(crate::BigInt<4>);

    #[cfg(all(not(target_arch = "riscv64"), any(bigint_ops, test)))]
    impl Wide for Delegated {
        fn from_limbs(limbs: [u64; 4]) -> Self {
            Self(crate::BigInt(limbs))
        }

        fn limbs(&self) -> [u64; 4] {
            self.0 .0
        }

        fn mul_low(&self, rhs: &Self) -> Self {
            let mut r = *self;
            crate::bigint_delegation::u256::mul_low_assign(&mut r.0, &rhs.0);
            r
        }

        fn mul_high(&self, rhs: &Self) -> Self {
            let mut r = *self;
            crate::bigint_delegation::u256::mul_high_assign(&mut r.0, &rhs.0);
            r
        }

        fn add(&self, rhs: &Self) -> (Self, bool) {
            let mut r = *self;
            let carry = crate::bigint_delegation::u256::add_assign(&mut r.0, &rhs.0);
            (r, carry)
        }

        fn sub(&self, rhs: &Self) -> (Self, bool) {
            let mut r = *self;
            let borrow = crate::bigint_delegation::u256::sub_assign(&mut r.0, &rhs.0);
            (r, borrow)
        }
    }
}

/// The decomposition of `k` (see `GLVConfigNoAllocator::scalar_decomposition_no_allocator`) on
/// the 256-bit integers `U`: `k` is below the group order and the half-scalars below `2^128`,
/// so every product fits 256 or 512 bits.
#[inline(always)]
pub(crate) fn decompose<C: GLVConfigNoAllocator + ?Sized, U: Wide>(
    k: C::ScalarField,
) -> ((bool, C::ScalarField), (bool, C::ScalarField))
where
    <C::ScalarField as PrimeField>::BigInt: AsRef<[u64]>,
{
    let limbs_of = |limbs: &[u64]| -> U {
        let mut four = [0u64; 4];
        four.copy_from_slice(&limbs[..4]);
        U::from_limbs(four)
    };
    let one = U::from_limbs([1, 0, 0, 0]);
    // s = k as an integer (below 2^256)
    let s = limbs_of(k.into_bigint().as_ref());
    // beta = round(s * BETA / 2^512) exactly, with `BETA = B1 2^256 + B0`:
    // `s BETA = L0 + (H0 + L1) 2^256 + (H1 + carry) 2^512`, rounded up for a positive
    // result when bit 511 of the product (bit 255 of the middle limb) is set
    let mul_shift = |(sign, beta): (bool, U512)| -> (bool, U) {
        let limbs = beta.as_limbs();
        let b0 = limbs_of(&limbs[..4]);
        let b1 = limbs_of(&limbs[4..]);
        let (mid, carry) = s.mul_high(&b0).add(&s.mul_low(&b1));
        let mut high = s.mul_high(&b1);
        if carry {
            let (sum, overflow) = high.add(&one);
            debug_assert!(!overflow);
            high = sum;
        }
        if sign && (mid.limbs()[3] >> 63) == 1 {
            let (sum, overflow) = high.add(&one);
            debug_assert!(!overflow);
            high = sum;
        }
        (sign, high)
    };
    let beta_1 = mul_shift(C::BETA_1);
    let beta_2 = mul_shift(C::BETA_2);
    let coeff = |i: usize| -> (bool, U) {
        let (sign, big) = C::SCALAR_DECOMP_COEFFS[i];
        (sign, limbs_of(big.as_ref()))
    };
    let [n11, n12, n21, n22] = [coeff(0), coeff(1), coeff(2), coeff(3)];
    // signed products of magnitudes below 2^128: they fit 256 bits
    let prod = |(sa, a): (bool, U), (sb, b): (bool, U)| -> (bool, U) {
        debug_assert!(a.mul_high(&b).limbs() == [0; 4]);
        (!(sa ^ sb), a.mul_low(&b))
    };
    // signed sum with the conventions of the former signed 512-bit helper: `true` is
    // positive and a cancellation gives a negative zero
    let add = |(sa, a): (bool, U), (sb, b): (bool, U)| -> (bool, U) {
        if sa == sb {
            let (r, overflow) = a.add(&b);
            debug_assert!(!overflow);
            (sa, r)
        } else {
            let (r, borrow) = a.sub(&b);
            if borrow {
                (sb, b.sub(&a).0)
            } else if r.limbs() == [0; 4] {
                (false, r)
            } else {
                (sa, r)
            }
        }
    };
    let neg = |(sign, a): (bool, U)| (!sign, a);
    let b1 = add(prod(beta_1, n11), prod(beta_2, n21));
    let b2 = add(prod(beta_1, n12), prod(beta_2, n22));
    let k1 = add((true, s), neg(b1));
    let k2 = neg(b2);
    let to_field = |(sign, a): (bool, U)| {
        let mut bytes = [0u8; 32];
        for (i, limb) in a.limbs().iter().enumerate() {
            bytes[8 * i..8 * i + 8].copy_from_slice(&limb.to_le_bytes());
        }
        (sign, C::ScalarField::from_le_bytes_mod_order(&bytes))
    };
    (to_field(k1), to_field(k2))
}

/// Joint sparse form of two non-negative scalars (Solinas): digits in {-1, 0, 1}, least
/// significant first, with at most half of the positions non-zero in both together.
/// The scalars must be below 2^127, as the GLV half-scalars are.
fn joint_sparse_form(mut k0: u128, mut k1: u128) -> ([i8; 130], [i8; 130], usize) {
    let mut d0 = [0i8; 130];
    let mut d1 = [0i8; 130];
    let mut len = 0;
    while k0 != 0 || k1 != 0 {
        let digit = |k: u128, other: u128| -> i8 {
            if k & 1 == 0 {
                return 0;
            }
            // 1 for k = 1 mod 4, -1 for k = 3 mod 4
            let mut u: i8 = if k & 3 == 3 { -1 } else { 1 };
            let m = k & 7;
            if (m == 3 || m == 5) && (other & 3) == 2 {
                u = -u;
            }
            u
        };
        let (u0, u1) = (digit(k0, k1), digit(k1, k0));
        // k = (k - u) / 2, on non-negative values (k + 1 cannot overflow below 2^127)
        // (k - u) / 2 without overflowing at 2^128: for an odd k, (k + 1) / 2 = (k >> 1) + 1
        k0 = (k0 >> 1) + (u0 == -1) as u128;
        k1 = (k1 >> 1) + (u1 == -1) as u128;
        d0[len] = u0;
        d1[len] = u1;
        len += 1;
    }
    (d0, d1, len)
}

/// The low 128 bits of a scalar that is known to fit in them
/// The half-scalar as an integer, `None` if it does not fit 128 bits
fn to_u128<F: PrimeField>(k: F) -> Option<u128> {
    let limbs = k.into_bigint();
    let limbs = limbs.as_ref();
    if limbs[2..].iter().any(|l| *l != 0) {
        return None;
    }
    Some((limbs[0] as u128) | ((limbs[1] as u128) << 64))
}

/// GLV scalar multiplication with the two half-scalars in joint sparse form: the same
/// doublings as a bit-by-bit loop but about half as many additions (one per non-zero joint
/// digit) on the in-place Jacobian group law: mixed additions of the affine `±b1`, `±b2` and
/// full additions of the precomputed `±(b1 + b2)`, `±(b1 - b2)`.
pub fn glv_mul_projective_jsf<C: GLVConfig + GLVConfigNoAllocator>(
    p: ark_ec::short_weierstrass::Projective<C>,
    k: C::ScalarField,
) -> ark_ec::short_weierstrass::Projective<C>
where
    C::BaseField: crate::extension_tower::CopyAssign,
{
    use crate::jacobian::Jacobian;
    use ark_std::Zero;
    use core::mem::MaybeUninit;
    if p.is_zero() {
        return p;
    }
    let Some((signs, d1, d2, len)) = jsf_digits::<C>(k) else {
        // the decomposition keeps both halves below 2^128 for the supported curves; a plain
        // double-and-add is the fallback should one not fit
        use ark_ec::PrimeGroup;
        return p.mul_bigint(k.into_bigint());
    };
    if len == 0 {
        return ark_ec::short_weierstrass::Projective::<C>::zero();
    }
    let mut slots = [const { MaybeUninit::uninit() }; 4];
    let table = JsfTable::<C>::new(&p, signs, &mut slots);

    let mut slot = MaybeUninit::uninit();
    let res = Jacobian::<C>::init_infinity(&mut slot);
    for i in (0..len).rev() {
        if i != len - 1 {
            res.double_in_place();
        }
        table.add_digits(res, d1[i], d2[i]);
    }
    res.to_projective()
}

/// `k P + l Q`, the two GLV double-and-adds interleaved so that the doublings are shared:
/// each point costs its additions only (about a quarter of the digit pairs are zero)
pub fn glv_mul_two_projective_jsf<C: GLVConfig + GLVConfigNoAllocator>(
    p: ark_ec::short_weierstrass::Projective<C>,
    k: C::ScalarField,
    q: ark_ec::short_weierstrass::Projective<C>,
    l: C::ScalarField,
) -> ark_ec::short_weierstrass::Projective<C>
where
    C::BaseField: crate::extension_tower::CopyAssign,
{
    use crate::jacobian::Jacobian;
    use ark_std::Zero;
    use core::mem::MaybeUninit;
    if p.is_zero() {
        return glv_mul_projective_jsf::<C>(q, l);
    }
    if q.is_zero() {
        return glv_mul_projective_jsf::<C>(p, k);
    }
    let (Some((signs_p, d1, d2, len_p)), Some((signs_q, e1, e2, len_q))) =
        (jsf_digits::<C>(k), jsf_digits::<C>(l))
    else {
        return glv_mul_projective_jsf::<C>(p, k) + glv_mul_projective_jsf::<C>(q, l);
    };
    let len = len_p.max(len_q);
    if len == 0 {
        return ark_ec::short_weierstrass::Projective::<C>::zero();
    }
    let mut slots_p = [const { MaybeUninit::uninit() }; 4];
    let table_p = JsfTable::<C>::new(&p, signs_p, &mut slots_p);
    let mut slots_q = [const { MaybeUninit::uninit() }; 4];
    let table_q = JsfTable::<C>::new(&q, signs_q, &mut slots_q);

    let mut slot = MaybeUninit::uninit();
    let res = Jacobian::<C>::init_infinity(&mut slot);
    for i in (0..len).rev() {
        if i != len - 1 {
            res.double_in_place();
        }
        if i < len_p {
            table_p.add_digits(res, d1[i], d2[i]);
        }
        if i < len_q {
            table_q.add_digits(res, e1[i], e2[i]);
        }
    }
    res.to_projective()
}

/// The GLV halves of `k`, each as whether it is positive and its magnitude, `None` if a half
/// does not fit 128 bits
#[allow(clippy::type_complexity)]
pub(crate) fn glv_halves<C: GLVConfig + GLVConfigNoAllocator>(
    k: C::ScalarField,
) -> Option<((bool, u128), (bool, u128))> {
    let ((sgn_k1, k1), (sgn_k2, k2)) = C::scalar_decomposition(k);
    Some(((sgn_k1, to_u128(k1)?), (sgn_k2, to_u128(k2)?)))
}

/// The signs of the GLV halves of `k` and the joint sparse form of their magnitudes, `None`
/// if a half does not fit 128 bits
#[allow(clippy::type_complexity)]
fn jsf_digits<C: GLVConfig + GLVConfigNoAllocator>(
    k: C::ScalarField,
) -> Option<((bool, bool), [i8; 130], [i8; 130], usize)> {
    let ((sgn_k1, k1), (sgn_k2, k2)) = C::scalar_decomposition(k);
    let (k1, k2) = (to_u128(k1)?, to_u128(k2)?);
    let (d1, d2, len) = joint_sparse_form(k1, k2);
    Some(((sgn_k1, sgn_k2), d1, d2, len))
}

/// The points a joint sparse form of the GLV halves of a scalar adds: `±P`, `±φ(P)` with the
/// signs of the halves folded in, and their sum and difference
struct JsfTable<'a, C: ark_ec::short_weierstrass::SWCurveConfig>
where
    C::BaseField: crate::extension_tower::CopyAssign,
{
    b1: ark_ec::short_weierstrass::Affine<C>,
    b2: ark_ec::short_weierstrass::Affine<C>,
    neg_b1: ark_ec::short_weierstrass::Affine<C>,
    neg_b2: ark_ec::short_weierstrass::Affine<C>,
    sum: &'a crate::jacobian::Jacobian<C>,
    diff: &'a crate::jacobian::Jacobian<C>,
    neg_sum: &'a crate::jacobian::Jacobian<C>,
    neg_diff: &'a crate::jacobian::Jacobian<C>,
}

impl<'a, C: GLVConfig + GLVConfigNoAllocator> JsfTable<'a, C>
where
    C::BaseField: crate::extension_tower::CopyAssign,
{
    /// `signs` are those of the two GLV halves of the scalar
    fn new(
        p: &ark_ec::short_weierstrass::Projective<C>,
        (sgn_k1, sgn_k2): (bool, bool),
        slots: &'a mut [core::mem::MaybeUninit<crate::jacobian::Jacobian<C>>; 4],
    ) -> Self {
        use crate::jacobian::Jacobian;
        use ark_ec::CurveGroup;
        use ark_ff::{AdditiveGroup, One};
        // b1 = ±P, b2 = ±φ(P) as affine points, and their negations
        let mut b1 = if p.z.is_one() {
            ark_ec::short_weierstrass::Affine::<C>::new_unchecked(p.x, p.y)
        } else {
            p.into_affine()
        };
        let mut b2 = C::endomorphism_affine(&b1);
        if !sgn_k1 {
            b1.y.neg_in_place();
        }
        if !sgn_k2 {
            b2.y.neg_in_place();
        }
        let neg_b1 = -b1;
        let neg_b2 = -b2;
        // sum = b1 + b2 and diff = b1 - b2 in Jacobian coordinates, and their negations
        let [sum_slot, diff_slot, neg_sum_slot, neg_diff_slot] = slots;
        let sum = Jacobian::<C>::init_infinity(sum_slot);
        sum.add_assign_affine(&b1);
        sum.add_assign_affine(&b2);
        let diff = Jacobian::<C>::init_infinity(diff_slot);
        diff.add_assign_affine(&b1);
        diff.add_assign_affine(&neg_b2);
        let neg_sum = Jacobian::<C>::init_copy(neg_sum_slot, sum);
        neg_sum.neg_in_place();
        let neg_diff = Jacobian::<C>::init_copy(neg_diff_slot, diff);
        neg_diff.neg_in_place();
        Self {
            b1,
            b2,
            neg_b1,
            neg_b2,
            sum,
            diff,
            neg_sum,
            neg_diff,
        }
    }

    #[inline(always)]
    fn add_digits(&self, res: &mut crate::jacobian::Jacobian<C>, d1: i8, d2: i8) {
        match (d1, d2) {
            (0, 0) => {}
            (1, 0) => res.add_assign_affine(&self.b1),
            (-1, 0) => res.add_assign_affine(&self.neg_b1),
            (0, 1) => res.add_assign_affine(&self.b2),
            (0, -1) => res.add_assign_affine(&self.neg_b2),
            (1, 1) => res.add_assign(self.sum),
            (-1, -1) => res.add_assign(self.neg_sum),
            (1, -1) => res.add_assign(self.diff),
            (-1, 1) => res.add_assign(self.neg_diff),
            _ => unreachable!("joint sparse form digits are -1, 0 or 1"),
        }
    }
}

#[cfg(test)]
mod jsf_tests {
    use super::joint_sparse_form;

    #[test]
    fn jsf_evaluates_back_and_is_sparse() {
        let mut state = 0x9E3779B97F4A7C15u128;
        let mut pairs = vec![
            (u128::MAX, u128::MAX),
            (u128::MAX, 0),
            (1u128 << 127, u128::MAX - 1),
            ((1u128 << 127) + 3, (1u128 << 127) + 5),
            (0, 1),
        ];
        for _ in 0..2000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let k0 = state;
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let k1 = state >> (state & 0x7f) as u32;
            pairs.push((k0, k1));
        }
        for (k0, k1) in pairs {
            let (d0, d1, len) = joint_sparse_form(k0, k1);
            assert!(len <= 129);
            // reconstruction in two's complement: the values fit 129 bits, compared modulo 2^128
            let (mut v0, mut v1) = (0i128, 0i128);
            let mut joint_weight = 0;
            for i in (0..len).rev() {
                v0 = v0.wrapping_mul(2).wrapping_add(d0[i] as i128);
                v1 = v1.wrapping_mul(2).wrapping_add(d1[i] as i128);
                joint_weight += (d0[i] != 0 || d1[i] != 0) as usize;
                // of any three consecutive columns at least one is (0, 0), and adjacent
                // digits of one scalar never have opposite signs
                if i >= 2 {
                    let zero = |j: usize| d0[j] == 0 && d1[j] == 0;
                    assert!(zero(i) || zero(i - 1) || zero(i - 2));
                }
                if i > 0 {
                    assert_ne!(d0[i] * d0[i - 1], -1);
                    assert_ne!(d1[i] * d1[i - 1], -1);
                }
            }
            assert_eq!(v0 as u128, k0);
            assert_eq!(v1 as u128, k1);
            // at least one zero column in every three: at most two thirds are non-zero
            assert!(
                joint_weight <= 2 * len / 3 + 2,
                "weight {joint_weight} of {len}"
            );
        }
        assert_eq!(joint_sparse_form(0, 0).2, 0);
    }
}

/// The two arithmetics of the decomposition against each other and against the reference, for
/// both curves that use it
#[cfg(test)]
mod decomposition_tests {
    use super::wide::{Delegated, Plain};
    use super::{decompose, GLVConfigNoAllocator, Wide};
    use ark_ec::CurveConfig;
    use ark_ff::{Field, PrimeField, Zero};
    use proptest::{prop_assert_eq, proptest};

    fn check<C: GLVConfigNoAllocator>(k: C::ScalarField)
    where
        <C::ScalarField as PrimeField>::BigInt: AsRef<[u64]>,
    {
        let plain = decompose::<C, Plain>(k);
        let delegated = decompose::<C, Delegated>(k);
        assert_eq!(plain, delegated, "{k:?}");
        assert_eq!(plain, C::scalar_decomposition_no_allocator(k));
        // the same halves as arkworks gives, up to the sign of a zero
        let reference = C::scalar_decomposition_ref(k);
        for (ours, theirs) in [(plain.0, reference.0), (plain.1, reference.1)] {
            assert_eq!(ours.1, theirs.1, "{k:?}");
            assert!(ours.0 == theirs.0 || ours.1.is_zero(), "{k:?}");
        }
        // and they fit 128 bits
        for half in [plain.0 .1, plain.1 .1] {
            assert!(half.into_bigint().as_ref()[2..]
                .iter()
                .all(|limb| *limb == 0));
        }
    }

    fn edges<C: GLVConfigNoAllocator>()
    where
        <C::ScalarField as PrimeField>::BigInt: AsRef<[u64]>,
    {
        type Scalar<C> = <C as CurveConfig>::ScalarField;
        let one = Scalar::<C>::ONE;
        let two = one + one;
        let mut power = one;
        for _ in 0..256 {
            // 2^i, 2^i - 1, 2^i + 1 and their negations
            for k in [power, power - one, power + one] {
                check::<C>(k);
                check::<C>(-k);
            }
            power *= two;
        }
        check::<C>(Scalar::<C>::zero());
        // the eigenvalue and the half of the order
        let lambda = C::LAMBDA;
        for k in [
            lambda,
            -lambda,
            lambda + one,
            lambda * lambda,
            two.inverse().unwrap(),
        ] {
            check::<C>(k);
        }
    }

    #[test]
    fn the_arithmetics_agree_on_edge_scalars() {
        edges::<crate::bn254::curves::g1::Config>();
        edges::<crate::bls12_381::curves::g1::Config>();
    }

    #[test]
    fn the_arithmetics_agree_on_random_scalars() {
        proptest!(|(bytes: [u8; 32])| {
            check::<crate::bn254::curves::g1::Config>(PrimeField::from_be_bytes_mod_order(&bytes));
            check::<crate::bls12_381::curves::g1::Config>(PrimeField::from_be_bytes_mod_order(&bytes));
        })
    }

    /// The plain multiplication, addition and subtraction against the delegated ones
    #[test]
    fn the_plain_integers_are_the_delegated_ones() {
        let edge = [
            0,
            1,
            2,
            u64::MAX - 1,
            u64::MAX,
            1 << 63,
            (1 << 63) - 1,
            0xffff_ffff,
            1 << 32,
        ];
        let compare = |a: [u64; 4], b: [u64; 4]| {
            let (pa, pb) = (Plain::from_limbs(a), Plain::from_limbs(b));
            let (da, db) = (Delegated::from_limbs(a), Delegated::from_limbs(b));
            assert_eq!(pa.mul_low(&pb).limbs(), da.mul_low(&db).limbs());
            assert_eq!(pa.mul_high(&pb).limbs(), da.mul_high(&db).limbs());
            let (ps, pc) = pa.add(&pb);
            let (ds, dc) = da.add(&db);
            assert_eq!((ps.limbs(), pc), (ds.limbs(), dc));
            let (ps, pc) = pa.sub(&pb);
            let (ds, dc) = da.sub(&db);
            assert_eq!((ps.limbs(), pc), (ds.limbs(), dc));
        };
        for x in edge {
            for y in edge {
                compare([x, y, x, y], [y, y, x, x]);
                compare([x, 0, 0, y], [y, u64::MAX, u64::MAX, x]);
                compare([x; 4], [y; 4]);
            }
        }
        proptest!(|(a: [u64; 4], b: [u64; 4])| {
            compare(a, b);
            prop_assert_eq!(Plain::from_limbs(a).limbs(), a);
        })
    }
}
