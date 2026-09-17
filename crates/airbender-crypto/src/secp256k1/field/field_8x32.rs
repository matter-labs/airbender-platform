use crate::ark_ff_delegation::{BigInt, BigIntMacro, BigInteger};
use crate::bigint_delegation::{
    u256, DelegatedBarretParams, DelegatedModParams, DelegatedMontParams,
};
use crate::k256::FieldBytes;

use super::field_10x26::FieldStorage10x26;

#[derive(Clone, Copy, Debug)]
pub struct FieldElement8x32(pub(super) BigInt<4>);

static MODULUS: BigInt<4> = FieldElement8x32::MODULUS;
static NEG_MODULUS: BigInt<4> = FieldElement8x32::NEG_MODULUS;
static REDUCTION_CONST: BigInt<4> = FieldElement8x32::REDUCTION_CONST;
/// `2^512 mod MODULUS`, Montgomery multiplication by it converts into Montgomery form
static R2: BigInt<4> = FieldElement8x32::R2;
static ONE_INTEGER: BigInt<4> = BigInt::one();

#[derive(Debug, Default)]
pub struct FieldParams;

impl DelegatedModParams<4> for FieldParams {
    const MODULUS_BITSIZE: usize = 256;

    fn modulus() -> &'static BigInt<4> {
        &MODULUS
    }
}

impl DelegatedBarretParams<4> for FieldParams {
    fn neg_modulus() -> &'static BigInt<4> {
        &NEG_MODULUS
    }
}

impl DelegatedMontParams<4> for FieldParams {
    fn reduction_const() -> &'static BigInt<4> {
        &REDUCTION_CONST
    }
}

/// `x * 2^256 mod MODULUS` in canonical form, for constants. `2^256 = NEG_MODULUS (mod MODULUS)`.
const fn to_montgomery_form_const(x: [u64; 4]) -> [u64; 4] {
    const C: u128 = FieldElement8x32::NEG_MODULUS.0[0] as u128;

    /// `x + y`, where y < 2^128, and the carry out
    const fn add(mut x: [u64; 4], y: u128) -> ([u64; 4], bool) {
        let mut carry = y;
        let mut i = 0;
        while i < 4 {
            let t = x[i] as u128 + (carry & (u64::MAX as u128));
            x[i] = t as u64;
            carry = (carry >> 64) + (t >> 64);
            i += 1;
        }
        (x, carry != 0)
    }

    // x * c = high * 2^256 + low, high < c
    let mut low = [0u64; 4];
    let mut high = 0u128;
    let mut i = 0;
    while i < 4 {
        let t = x[i] as u128 * C + high;
        low[i] = t as u64;
        high = t >> 64;
        i += 1;
    }

    // high * 2^256 = high * c < 2^128
    let (mut r, carry) = add(low, high * C);
    if carry {
        // it wrapped around, so can not overflow again
        (r, _) = add(r, C);
    }

    // r >= MODULUS if and only if r + c >= 2^256, and r - MODULUS = r + c - 2^256
    let (reduced, carry) = add(r, C);
    if carry {
        r = reduced;
    }

    r
}

impl FieldElement8x32 {
    pub(super) const ZERO: Self = Self(BigInt::zero());
    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    pub(super) const BETA: Self = Self(BigInt(to_montgomery_form_const(
        BigIntMacro!(
            "55594575648329892869085402983802832744385952214688224221778511981742606582254"
        )
        .0,
    )));
    // 2^256 mod MODULUS
    pub(super) const ONE: Self = Self(Self::NEG_MODULUS);
    // 2^256 - MODULUS
    const NEG_MODULUS: BigInt<4> = BigIntMacro!("4294968273");
    // NEG_MODULUS^2
    const R2: BigInt<4> = BigIntMacro!("18446752466076602529");
    // -1/MODULUS mod 2^256
    const REDUCTION_CONST: BigInt<4> = BigIntMacro!(
        "91248989341183975618893650062416139444822672217621753343178995607987479196977"
    );
    const MODULUS: BigInt<4> = BigIntMacro!(
        "115792089237316195423570985008687907853269984665640564039457584007908834671663"
    );

    // NOTE: the element `x` is kept in Montgomery form, as `x * 2^256 mod MODULUS`: Montgomery
    // multiplication takes 6 delegation calls, and multiplication with the reduction that uses
    // the special form of the modulus takes 9. So we convert on the way from and to bytes,
    // and the storage (precomputed tables) keeps Montgomery form too.

    /// For constants (software conversion)
    #[allow(dead_code)]
    #[inline(always)]
    pub(super) const fn from_bytes_unchecked(bytes: &[u8; 32]) -> Self {
        Self(BigInt(to_montgomery_form_const(
            u256::from_bytes_unchecked(bytes).0,
        )))
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn from_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let mut value = Self(u256::from_bytes_unchecked(bytes));

        if u256::lt(&value.0, &Self::MODULUS) {
            unsafe {
                u256::mul_assign_montgomery_weak::<FieldParams>(&mut value.0, &R2);
            }
            Some(value)
        } else {
            None
        }
    }

    /// Canonical integer that the element represents
    #[inline(always)]
    fn to_integer(mut self) -> BigInt<4> {
        unsafe {
            u256::mul_assign_montgomery_weak::<FieldParams>(&mut self.0, &ONE_INTEGER);
            u256::normalize_weak::<FieldParams>(&mut self.0);
        }
        self.0
    }

    #[inline(always)]
    pub(super) fn to_bytes(self) -> FieldBytes {
        u256::to_be_bytes(self.to_integer()).into()
    }

    /// NOTE: the words of the Montgomery form, what `to_storage` gives
    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    pub(super) fn from_words(words: [u64; 4]) -> Self {
        Self(BigInt(words))
    }

    // NOTE: arithmetic below is "weakly reduced": the value is any 256-bit representative of
    // the residue class (see `bigint_delegation::u256`), and only `normalize_in_place` makes it
    // canonical. It saves a `sub p`/`add p` pair of delegation calls on almost every operation.

    #[inline(always)]
    pub(super) fn mul_in_place(&mut self, rhs: &Self) {
        unsafe {
            u256::mul_assign_montgomery_weak::<FieldParams>(&mut self.0, &rhs.0);
        }
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn mul_int_in_place(&mut self, rhs: u32) {
        // `rhs` is an integer and not in Montgomery form, so that's the plain multiplication
        let rhs = BigInt([rhs as u64, 0, 0, 0]);
        unsafe {
            u256::mul_assign_weak::<FieldParams>(&mut self.0, &rhs);
        }
    }

    #[inline(always)]
    pub(super) fn square_in_place(&mut self) {
        unsafe {
            u256::square_assign_montgomery_weak::<FieldParams>(&mut self.0);
        }
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn add_int_in_place(&mut self, rhs: u32) {
        let mut rhs = BigInt([rhs as u64, 0, 0, 0]);
        unsafe {
            u256::mul_assign_montgomery_weak::<FieldParams>(&mut rhs, &R2);
            u256::add_mod_assign_weak::<FieldParams>(&mut self.0, &rhs);
        }
    }

    #[inline(always)]
    pub(super) fn add_in_place(&mut self, rhs: &Self) {
        unsafe {
            u256::add_mod_assign_weak::<FieldParams>(&mut self.0, &rhs.0);
        }
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn double_in_place(&mut self) {
        unsafe {
            u256::double_mod_assign_weak::<FieldParams>(&mut self.0);
        }
    }

    #[inline(always)]
    pub(super) fn triple_in_place(&mut self) {
        unsafe {
            u256::triple_mod_assign_weak::<FieldParams>(&mut self.0);
        }
    }

    #[inline(always)]
    pub(super) fn sub_in_place(&mut self, rhs: &Self) {
        unsafe { u256::sub_mod_assign_weak::<FieldParams>(&mut self.0, &rhs.0) };
    }

    /// Computes `self = minuend - self`
    #[inline(always)]
    pub(super) fn sub_and_negate_in_place(&mut self, minuend: &Self) {
        unsafe { u256::sub_and_negate_mod_assign_weak::<FieldParams>(&mut self.0, &minuend.0) };
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn negate_in_place(&mut self, _magnitude: u32) {
        unsafe { u256::neg_mod_assign_weak::<FieldParams>(&mut self.0) };
    }

    /// Copies `src` with a single delegation call instead of a word-by-word copy
    #[inline(always)]
    pub(super) fn copy_from(&mut self, src: &Self) {
        u256::copy_assign(&mut self.0, &src.0);
    }

    /// Initializes `dst` with a copy of `src` by a single delegation call. The copy is made in
    /// place: returning the element by value ends up as a word-by-word move of it.
    #[inline(always)]
    pub(super) fn write_copy<'a>(
        dst: &'a mut core::mem::MaybeUninit<Self>,
        src: &Self,
    ) -> &'a mut Self {
        #[cfg(target_arch = "riscv32")]
        // SAFETY: `MemCpy` doesn't depend on the initial content of the destination,
        // it initializes all of it, and both pointers are properly aligned
        unsafe {
            crate::bigint_delegation::delegation::bigint_op_delegation_with_carry_bit(
                core::ptr::addr_of_mut!((*dst.as_mut_ptr()).0),
                &src.0,
                false,
                crate::BigIntOps::MemCpy,
            );
            dst.assume_init_mut()
        }

        // emulated delegation reads the destination
        #[cfg(not(target_arch = "riscv32"))]
        {
            dst.write(*src)
        }
    }

    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn normalize_in_place(&mut self) {
        unsafe { u256::normalize_weak::<FieldParams>(&mut self.0) };
    }

    #[inline(always)]
    pub(super) fn normalizes_to_zero(&self) -> bool {
        unsafe { u256::is_zero_mod::<FieldParams>(&self.0) }
    }

    /// Parity of the canonical representative
    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) fn is_odd(&self) -> bool {
        self.to_integer().is_odd()
    }

    /// NOTE: expects normalized element, gives the words of its Montgomery form
    #[allow(dead_code)] // TODO: to be fixed in `zksync-os/crypto` first
    #[inline(always)]
    pub(super) const fn to_storage(self) -> FieldStorage10x26 {
        let mut res = [0; 8];
        let words = self.0 .0;
        let mut i = 0;
        while i < 4 {
            res[2 * i] = words[i] as u32;
            res[2 * i + 1] = (words[i] >> 32) as u32;
            i += 1;
        }
        FieldStorage10x26(res)
    }

    #[inline(always)]
    fn pow2k_in_place(&mut self, k: usize) {
        for _ in 0..k {
            self.square_in_place();
        }
    }

    #[inline(always)]
    pub(super) fn invert_in_place(&mut self) {
        let x1 = *self;

        self.pow2k_in_place(1);
        self.mul_in_place(&x1);
        let x2 = *self;

        self.pow2k_in_place(1);
        self.mul_in_place(&x1);
        let x3 = *self;

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(2);
        self.mul_in_place(&x2);
        let x11 = *self;

        self.pow2k_in_place(11);
        self.mul_in_place(&x11);
        let x22 = *self;

        self.pow2k_in_place(22);
        self.mul_in_place(&x22);
        let x44 = *self;

        self.pow2k_in_place(44);
        self.mul_in_place(&x44);
        let x88 = *self;

        self.pow2k_in_place(88);
        self.mul_in_place(&x88);

        self.pow2k_in_place(44);
        self.mul_in_place(&x44);

        self.pow2k_in_place(3);
        self.mul_in_place(&x3);

        self.pow2k_in_place(23);
        self.mul_in_place(&x22);
        self.pow2k_in_place(5);
        self.mul_in_place(&x1);
        self.pow2k_in_place(3);
        self.mul_in_place(&x2);
        self.pow2k_in_place(2);
        self.mul_in_place(&x1);
    }
}

#[cfg(test)]
impl PartialEq for FieldElement8x32 {
    fn eq(&self, other: &Self) -> bool {
        let (mut a, mut b) = (*self, *other);
        a.normalize_in_place();
        b.normalize_in_place();
        u256::eq(&a.0, &b.0)
    }
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for FieldElement8x32 {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::{any, Strategy};

        any::<u256::U256Wrapper<FieldParams>>().prop_map(|x| Self(x.0))
    }

    type Strategy = proptest::arbitrary::Mapped<u256::U256Wrapper<FieldParams>, Self>;
}

#[cfg(test)]
mod tests {
    use super::FieldElement8x32;
    use proptest::{prop_assert_eq, proptest};

    #[test]
    fn test_invert() {
        proptest!(|(x: FieldElement8x32)| {
            let mut a = x;
            a.invert_in_place();
            a.invert_in_place();
            prop_assert_eq!(a, x);

            a = x;
            a.invert_in_place();
            a.mul_in_place(&x);

            if x.normalizes_to_zero() {
                prop_assert_eq!(a, FieldElement8x32::ZERO);
            } else {
                prop_assert_eq!(a, FieldElement8x32::ONE);
            }
        })
    }

    #[test]
    fn test_mul() {
        proptest!(|(x: FieldElement8x32, y: FieldElement8x32, z: FieldElement8x32)| {
            let mut a = x;
            let mut b = y;

            // x * y = y * x
            a.mul_in_place(&y);
            b.mul_in_place(&x);
            prop_assert_eq!(a, b);

            // (x * y) * z = x * (y * z)
            a = x;
            b = y;
            a.mul_in_place(&y);
            a.mul_in_place(&z);
            b.mul_in_place(&z);
            b.mul_in_place(&x);
            prop_assert_eq!(a, b);

            // x * 1 = x
            a = x;
            a.mul_in_place(&FieldElement8x32::ONE);
            prop_assert_eq!(a, x);

            // x * 0 = 0
            a = x;
            a.mul_in_place(&FieldElement8x32::ZERO);
            prop_assert_eq!(a, FieldElement8x32::ZERO);

            // x * (y + z) = x * y + x * z
            a = y;
            b = x;
            let mut c = x;
            a.add_in_place(&z);
            a.mul_in_place(&x);
            b.mul_in_place(&y);
            c.mul_in_place(&z);
            b.add_in_place(&c);
            prop_assert_eq!(a, b);
        })
    }

    #[test]
    fn test_add() {
        proptest!(|(x: FieldElement8x32, y: FieldElement8x32, z: FieldElement8x32)| {
            let mut a = x;
            let mut b = y;

            // x + y = y + x
            a.add_in_place(&y);
            b.add_in_place(&x);
            prop_assert_eq!(a, b);

            // x + 0 = x
            a = x;
            a.add_in_place(&FieldElement8x32::ZERO);
            prop_assert_eq!(a, x);

            // (x + y) + z = x + (y + z)
            a = x;
            b = y;
            a.add_in_place(&y);
            a.add_in_place(&z);
            b.add_in_place(&z);
            b.add_in_place(&x);
            prop_assert_eq!(a, b);

            // x - x = 0
            a = x;
            a.sub_in_place(&x);
            prop_assert_eq!(a, FieldElement8x32::ZERO);

            // x + y - y = x
            a = x;
            a.add_in_place(&y);
            a.sub_in_place(&y);
            prop_assert_eq!(a, x);

            // x - y + y = x
            a = x;
            a.sub_in_place(&y);
            a.add_in_place(&y);
            prop_assert_eq!(a, x);
        })
    }

    // The field is weakly reduced, so every operation has to be correct for non-canonical
    // inputs as well. Those are never hit by the uniform sampling, so they are forced here.
    // `fe(x)` is the element with the representation `x`, i.e. `x / 2^256`.
    mod weak_reduction {
        use super::super::{BigInt, FieldElement8x32};
        use proptest::prelude::{any, prop_oneof, Strategy};
        use proptest::{prop_assert, prop_assert_eq, proptest};
        use ruint::aliases::{U256, U512};

        const P: U256 = U256::from_limbs(FieldElement8x32::MODULUS.0);

        /// Any 256-bit integer, biased to the edges of `[0, p)` and `[p, 2^256)`
        fn any_weak() -> impl Strategy<Value = U256> {
            let small = || any::<u64>().prop_map(|x| U256::from(x % 5000));
            prop_oneof![
                any::<[u64; 4]>().prop_map(U256::from_limbs),
                small(),
                small().prop_map(|x| P - x),
                small().prop_map(|x| P + x),
                small().prop_map(|x| U256::MAX - x),
            ]
        }

        fn fe(x: U256) -> FieldElement8x32 {
            FieldElement8x32(BigInt(x.into_limbs()))
        }

        fn canonical(mut x: FieldElement8x32) -> U256 {
            x.normalize_in_place();
            U256::from_limbs(x.0 .0)
        }

        fn reduce(x: U512) -> U256 {
            let (_, r) = x.div_rem(U512::from(P));
            U256::from(r)
        }

        fn wide(x: U256) -> U512 {
            U512::from(x)
        }

        /// 2^256 mod p
        fn r() -> U256 {
            reduce(U512::from(1) << 256)
        }

        /// `x / 2^256 mod p`
        fn from_montgomery(x: U256) -> U256 {
            let r_inv = r().inv_mod(P).unwrap();
            reduce(wide(reduce(wide(x))) * wide(r_inv))
        }

        #[test]
        fn constants() {
            let p = U256::from_limbs(FieldElement8x32::MODULUS.0);
            let reduction_const = U256::from_limbs(FieldElement8x32::REDUCTION_CONST.0);
            prop_assert_eq_panics(p.wrapping_mul(reduction_const), U256::MAX);
            prop_assert_eq_panics(
                U256::from_limbs(FieldElement8x32::R2.0),
                reduce(wide(r()) * wide(r())),
            );
            prop_assert_eq_panics(U256::from_limbs(FieldElement8x32::ONE.0 .0), r());
            prop_assert_eq_panics(
                U256::from_limbs(FieldElement8x32::NEG_MODULUS.0),
                U256::ZERO.wrapping_sub(p),
            );
        }

        fn prop_assert_eq_panics(a: U256, b: U256) {
            assert_eq!(a, b);
        }

        /// Low half of the product decides the carry of the Montgomery reduction, and it is
        /// looked at in two steps: the lowest 32 bits, and (out of line) everything.
        #[test]
        fn montgomery_low_part_carry() {
            let one = U256::from(1);
            let cases = [
                // low half is zero
                (U256::ZERO, U256::MAX),
                (one << 128, one << 128),
                (one << 255, U256::from(2)),
                // non-zero with zero lowest 32 bits
                (one << 32, one),
                (one << 64, U256::MAX),
                (one << 255, one),
                (U256::MAX << 32, one),
                // non-zero lowest 32 bits
                (one, one),
                (U256::MAX, U256::MAX),
            ];
            for (x, y) in cases {
                let mut t = fe(x);
                t.mul_in_place(&fe(y));
                assert_eq!(canonical(t), from_montgomery(reduce(wide(x) * wide(y))));
            }
        }

        #[test]
        fn binary_ops() {
            proptest!(|(x in any_weak(), y in any_weak())| {
                let mut t = fe(x);
                t.mul_in_place(&fe(y));
                prop_assert_eq!(canonical(t), from_montgomery(reduce(wide(x) * wide(y))));

                let mut t = fe(x);
                t.add_in_place(&fe(y));
                prop_assert_eq!(canonical(t), reduce(wide(x) + wide(y)));

                let mut t = fe(x);
                t.sub_in_place(&fe(y));
                prop_assert_eq!(canonical(t), reduce(wide(x) + wide(P) + wide(P) - wide(y)));

                let mut t = fe(x);
                t.sub_and_negate_in_place(&fe(y));
                prop_assert_eq!(canonical(t), reduce(wide(y) + wide(P) + wide(P) - wide(x)));
            })
        }

        #[test]
        fn unary_ops() {
            proptest!(|(x in any_weak(), k: u32)| {
                let mut t = fe(x);
                t.square_in_place();
                prop_assert_eq!(canonical(t), from_montgomery(reduce(wide(x) * wide(x))));

                let mut t = fe(x);
                t.double_in_place();
                prop_assert_eq!(canonical(t), reduce(wide(x) + wide(x)));

                let mut t = fe(x);
                t.triple_in_place();
                prop_assert_eq!(canonical(t), reduce(wide(x) + wide(x) + wide(x)));

                let mut t = fe(x);
                t.negate_in_place(1);
                prop_assert_eq!(canonical(t), reduce(wide(P) + wide(P) - wide(x)));

                let mut t = fe(x);
                t.mul_int_in_place(k);
                prop_assert_eq!(canonical(t), reduce(wide(x) * U512::from(k)));

                let mut t = fe(x);
                t.add_int_in_place(k);
                prop_assert_eq!(canonical(t), reduce(wide(x) + wide(r()) * U512::from(k)));

                let mut t = FieldElement8x32::ZERO;
                t.copy_from(&fe(x));
                prop_assert_eq!(t.0 .0, x.into_limbs());
                let mut t = core::mem::MaybeUninit::uninit();
                prop_assert_eq!(FieldElement8x32::write_copy(&mut t, &fe(x)).0 .0, x.into_limbs());

                let reduced = reduce(wide(x));
                prop_assert!(canonical(fe(x)) < P);
                prop_assert_eq!(canonical(fe(x)), reduced);
                prop_assert_eq!(fe(x).normalizes_to_zero(), reduced == U256::ZERO);

                let integer = from_montgomery(x);
                prop_assert_eq!(fe(x).is_odd(), integer.bit(0));
                prop_assert_eq!(&*fe(x).to_bytes(), &integer.to_be_bytes::<32>()[..]);
            })
        }

        #[test]
        fn from_bytes() {
            proptest!(|(x in any_weak())| {
                let bytes = x.to_be_bytes::<32>();
                let expected = reduce(wide(x) * wide(r()));

                // software conversion for the constants
                prop_assert_eq!(FieldElement8x32::from_bytes_unchecked(&bytes).0 .0, expected.into_limbs());

                match FieldElement8x32::from_bytes(&bytes) {
                    Some(t) => {
                        prop_assert!(x < P);
                        prop_assert_eq!(canonical(t), expected);
                    }
                    None => prop_assert!(x >= P),
                }
            })
        }

        #[test]
        fn invert() {
            proptest!(|(x in any_weak())| {
                let mut t = fe(x);
                t.invert_in_place();
                t.mul_in_place(&fe(x));
                let expected = if reduce(wide(x)) == U256::ZERO { U256::ZERO } else { r() };
                prop_assert_eq!(canonical(t), expected);
            })
        }
    }

    #[test]
    fn from_bytes_round() {
        proptest!(|(bytes: [u8; 32])| {
            prop_assert_eq!(&*FieldElement8x32::from_bytes_unchecked(&bytes).to_bytes(), &bytes);
        })
    }

    #[test]
    fn to_bytes_round() {
        proptest!(|(x: FieldElement8x32)| {
            let bytes = &*x.to_bytes();
            prop_assert_eq!(FieldElement8x32::from_bytes_unchecked(bytes.try_into().unwrap()), x);
        })
    }
}
