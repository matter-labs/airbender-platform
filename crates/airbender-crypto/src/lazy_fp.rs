//! Lazy reduction for the products of the extension towers, over a Montgomery base field of `N`
//! 64-bit limbs: for the 64-bit targets without the bigint delegation (`cfg(fp_lazy_reduction)`),
//! whose base fields are arkworks' `Fp`. The functions work on the limbs of its elements.
//!
//! A product of the tower is a sum of products of base field elements. Each one is taken as a
//! double-width integer ([`Wide`]), the sums are taken on those, and each coefficient of the
//! result is reduced once: a Montgomery reduction is about half of a multiplication. An `Fp2`
//! product takes 2 reductions for its 3 base products, an `Fp6` product 6 for its 18.
//!
//! Both curves take their `Fp2` products here. Only bls12-381 takes its `Fp6` products too
//! (`ξ = 1 + u`): a double-width sum costs about as much as half a reduction on a target
//! without a carry flag, and the multiplications by `ξ = 9 + u` of bn254 are ten of them each
//! (see `extension_tower::MulByXi`).
//!
//! The base field products outside the towers that are hot (the affine curve arithmetic of
//! `affine_glv`, the checks of the hinted divisions) take [`mul`], the product and its reduction
//! inline, through `extension_tower::CopyAssign::mul_assign_inline`, and their differences
//! [`sub_mod`]. Elements stay canonical between operations: every coordinate of the affine
//! formulas is stored, compared or sent to the oracle, which needs it canonical, so a form
//! below `2 p` between operations would save no subtraction there.
//!
//! # Bounds
//!
//! With `p` the modulus and `R = 2^(64 N)`, the modulus leaves the two top bits of its top limb
//! free: `4 p < R` ([`Params::new`] asserts it).
//!
//! - A narrow operand of a product is below `2 p`: a field element (canonical, below `p`), or
//!   the plain sum of two. A product of two such is below `4 p² < p R`.
//! - A wide value is in `[0, p R)` wherever a function takes or returns one. That is the range
//!   [`Wide::reduce`] accepts, and the sums and differences keep it: they are taken modulo
//!   `p R`, by one conditional `-p` or `+p` on the high half ([`Wide::add_mod`],
//!   [`Wide::sub_mod`]). A multiple of `p R` is a multiple of `p`, so the class modulo `p` is
//!   that of the integer sum.
//! - Inside [`fp2_product`] the Karatsuba sums `a0 + a1` and `b0 + b1` are plain, below
//!   `4 p < R`; their product is below `16 p² < R²`, so it fits; and the difference
//!   `(a0 + a1)(b0 + b1) - a0 b0 - a1 b1` is the integer `a0 b1 + a1 b0`, below `8 p² < 2 p R`:
//!   no borrow, and one conditional `-p R`, needed only if `8 p >= R` (`Params::fold`), brings
//!   it below `p R`.

/// `a * b + c + d`: the low and the high words. It does not overflow:
/// `(2^64 - 1)² + 2 (2^64 - 1) = 2^128 - 1`.
#[inline(always)]
fn mac(a: u64, b: u64, c: u64, d: u64) -> (u64, u64) {
    let wide = a as u128 * b as u128 + c as u128 + d as u128;
    (wide as u64, (wide >> 64) as u64)
}

/// `a + b + carry`
#[inline(always)]
fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
    let (sum, first) = a.overflowing_add(b);
    let (sum, second) = sum.overflowing_add(carry as u64);
    (sum, first | second)
}

/// `a - b - borrow`
#[inline(always)]
fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
    let (difference, first) = a.overflowing_sub(b);
    let (difference, second) = difference.overflowing_sub(borrow as u64);
    (difference, first | second)
}

/// The modulus of a base field and what its reductions take
pub(crate) struct Params<const N: usize> {
    /// The modulus `p`, little-endian
    p: [u64; N],
    /// `-p^-1 mod 2^64`
    inv: u64,
    /// Whether `8 p >= R`: a value below `8 p²` may then be `p R` or more
    fold: bool,
}

impl<const N: usize> Params<N> {
    /// The parameters of the odd modulus `p` with `4 p < 2^(64 N)`
    pub(crate) const fn new(p: [u64; N]) -> Self {
        assert!(p[0] & 1 == 1 && p[N - 1] >> 62 == 0);
        // Newton's iteration doubles the correct low bits of the inverse: 1, 2, ..., 64
        let mut inverse = 1u64;
        let mut i = 0;
        while i < 6 {
            inverse = inverse.wrapping_mul(2u64.wrapping_sub(p[0].wrapping_mul(inverse)));
            i += 1;
        }
        Self {
            p,
            inv: inverse.wrapping_neg(),
            fold: p[N - 1] >> 61 != 0,
        }
    }
}

/// `a + b`, not reduced: for operands below `2 p` it is below `4 p < R`
#[inline(always)]
fn add_narrow<const N: usize>(a: &[u64; N], b: &[u64; N]) -> [u64; N] {
    let mut sum = [0u64; N];
    let mut carry = false;
    for i in 0..N {
        (sum[i], carry) = adc(a[i], b[i], carry);
    }
    debug_assert!(!carry);
    sum
}

/// `a - b` for `a >= b`
#[inline(always)]
fn sub_narrow<const N: usize>(a: &[u64; N], b: &[u64; N]) -> [u64; N] {
    let mut difference = [0u64; N];
    let mut borrow = false;
    for i in 0..N {
        (difference[i], borrow) = sbb(a[i], b[i], borrow);
    }
    debug_assert!(!borrow);
    difference
}

/// `a - b mod p`, canonical, for canonical operands
#[inline(always)]
pub(crate) fn sub_mod<const N: usize>(a: &[u64; N], b: &[u64; N], params: &Params<N>) -> [u64; N] {
    let mut difference = [0u64; N];
    let mut borrow = false;
    for i in 0..N {
        (difference[i], borrow) = sbb(a[i], b[i], borrow);
    }
    if borrow {
        // the difference plus `R` is here: plus `p` it wraps into `(0, p)`
        let mut carry = false;
        for i in 0..N {
            (difference[i], carry) = adc(difference[i], params.p[i], carry);
        }
        debug_assert!(carry);
    }
    difference
}

/// A double-width integer, `lo + hi R`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wide<const N: usize> {
    lo: [u64; N],
    hi: [u64; N],
}

impl<const N: usize> Wide<N> {
    /// `a b`
    #[inline(always)]
    fn product(a: &[u64; N], b: &[u64; N]) -> Self {
        let mut lo = [0u64; N];
        let mut hi = [0u64; N];
        for i in 0..N {
            let mut carry = 0;
            for j in 0..N {
                let k = i + j;
                if k < N {
                    (lo[k], carry) = mac(a[i], b[j], lo[k], carry);
                } else {
                    (hi[k - N], carry) = mac(a[i], b[j], hi[k - N], carry);
                }
            }
            hi[i] = carry;
        }
        Self { lo, hi }
    }

    /// `self += other` as integers, which must not overflow
    #[inline(always)]
    fn add(&mut self, other: &Self) {
        let mut carry = false;
        for i in 0..N {
            (self.lo[i], carry) = adc(self.lo[i], other.lo[i], carry);
        }
        for i in 0..N {
            (self.hi[i], carry) = adc(self.hi[i], other.hi[i], carry);
        }
        debug_assert!(!carry);
    }

    /// `self -= other` modulo `R²`: returns whether it borrowed
    #[inline(always)]
    fn sub(&mut self, other: &Self) -> bool {
        let mut borrow = false;
        for i in 0..N {
            (self.lo[i], borrow) = sbb(self.lo[i], other.lo[i], borrow);
        }
        for i in 0..N {
            (self.hi[i], borrow) = sbb(self.hi[i], other.hi[i], borrow);
        }
        borrow
    }

    /// `self -= p R` if that is not negative: a value below `2 p R` comes below `p R`
    #[inline(always)]
    fn fold(&mut self, p: &[u64; N]) {
        let mut folded = [0u64; N];
        let mut borrow = false;
        for i in 0..N {
            (folded[i], borrow) = sbb(self.hi[i], p[i], borrow);
        }
        if !borrow {
            self.hi = folded;
        }
    }

    /// `self += other` modulo `p R`, for values below `p R`
    #[inline(always)]
    pub(crate) fn add_mod(&mut self, other: &Self, params: &Params<N>) {
        // below `2 p R < R²`
        self.add(other);
        self.fold(&params.p);
    }

    /// `self -= other` modulo `p R`, for values below `p R`
    #[inline(always)]
    pub(crate) fn sub_mod(&mut self, other: &Self, params: &Params<N>) {
        if self.sub(other) {
            // the difference plus `R²` is here: plus `p R` it wraps into `(0, p R)`
            let mut carry = false;
            for i in 0..N {
                (self.hi[i], carry) = adc(self.hi[i], params.p[i], carry);
            }
            debug_assert!(carry);
        }
    }

    /// `self / R mod p`, canonical, for a value below `p R` (the Montgomery reduction)
    #[inline(always)]
    pub(crate) fn reduce(mut self, params: &Params<N>) -> [u64; N] {
        let p = &params.p;
        // the carry out of the limb that the previous round ended in
        let mut top = false;
        for i in 0..N {
            // `lo[i] + m p[0]` is zero modulo `2^64`: it carries unless `lo[i]` is zero
            let m = self.lo[i].wrapping_mul(params.inv);
            let mut carry = ((m as u128 * p[0] as u128) >> 64) as u64 + (self.lo[i] != 0) as u64;
            for j in 1..N {
                let k = i + j;
                if k < N {
                    (self.lo[k], carry) = mac(m, p[j], self.lo[k], carry);
                } else {
                    (self.hi[k - N], carry) = mac(m, p[j], self.hi[k - N], carry);
                }
            }
            let (sum, first) = self.hi[i].overflowing_add(carry);
            let (sum, second) = sum.overflowing_add(top as u64);
            self.hi[i] = sum;
            top = first | second;
        }
        // `(self + m p) / R < p + p R / R = 2 p < R`: it is the high half, and nothing above
        debug_assert!(!top);
        let mut reduced = [0u64; N];
        let mut borrow = false;
        for i in 0..N {
            (reduced[i], borrow) = sbb(self.hi[i], p[i], borrow);
        }
        if borrow {
            self.hi
        } else {
            reduced
        }
    }
}

/// `a b / R mod p`, canonical, for operands below `2 p`: the Montgomery product
#[inline(always)]
pub(crate) fn mul<const N: usize>(a: &[u64; N], b: &[u64; N], params: &Params<N>) -> [u64; N] {
    Wide::product(a, b).reduce(params)
}

/// Whether `a` is below `p`
#[cfg(any(feature = "hinted_operations", test))]
#[inline(always)]
pub(crate) fn is_canonical<const N: usize>(a: &[u64; N], params: &Params<N>) -> bool {
    let mut borrow = false;
    for i in 0..N {
        (_, borrow) = sbb(a[i], params.p[i], borrow);
    }
    borrow
}

/// The limbs of the two components of an `Fp2` element
pub(crate) type Fp2Limbs<'a, const N: usize> = [&'a [u64; N]; 2];

/// An `Fp2` element (`u² = -1`) with double-width components
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wide2<const N: usize> {
    pub(crate) c0: Wide<N>,
    pub(crate) c1: Wide<N>,
}

impl<const N: usize> Wide2<N> {
    #[inline(always)]
    pub(crate) fn add_mod(&mut self, other: &Self, params: &Params<N>) {
        self.c0.add_mod(&other.c0, params);
        self.c1.add_mod(&other.c1, params);
    }

    #[inline(always)]
    pub(crate) fn sub_mod(&mut self, other: &Self, params: &Params<N>) {
        self.c0.sub_mod(&other.c0, params);
        self.c1.sub_mod(&other.c1, params);
    }

    /// The components reduced
    #[inline(always)]
    pub(crate) fn reduce(self, params: &Params<N>) -> [[u64; N]; 2] {
        [self.c0.reduce(params), self.c1.reduce(params)]
    }
}

/// `a b` in `Fp2`, for components below `2 p`: three base products (Karatsuba), no reduction
#[inline(always)]
pub(crate) fn fp2_product<const N: usize>(
    a: Fp2Limbs<N>,
    b: Fp2Limbs<N>,
    params: &Params<N>,
) -> Wide2<N> {
    let v0 = Wide::product(a[0], b[0]);
    let v1 = Wide::product(a[1], b[1]);
    // a0 b1 + a1 b0 = (a0 + a1)(b0 + b1) - a0 b0 - a1 b1, as integers
    let mut c1 = Wide::product(&add_narrow(a[0], a[1]), &add_narrow(b[0], b[1]));
    let borrow = c1.sub(&v0) | c1.sub(&v1);
    debug_assert!(!borrow);
    if params.fold {
        c1.fold(&params.p);
    }
    // a0 b0 - a1 b1
    let mut c0 = v0;
    c0.sub_mod(&v1, params);
    Wide2 { c0, c1 }
}

/// `a²` in `Fp2`, for canonical components: two base products, `(a0 + a1)(a0 - a1)` and
/// `(2 a0) a1`, no reduction. Their factors are plain sums below `2 p`, the difference as
/// `a0 + (p - a1)`, so the products are below `4 p² < p R`.
#[inline(always)]
pub(crate) fn fp2_square<const N: usize>(a: Fp2Limbs<N>, params: &Params<N>) -> Wide2<N> {
    let difference = add_narrow(a[0], &sub_narrow(&params.p, a[1]));
    Wide2 {
        c0: Wide::product(&add_narrow(a[0], a[1]), &difference),
        c1: Wide::product(&add_narrow(a[0], a[0]), a[1]),
    }
}

/// The plain sums of the components of `a` and `b`, which are canonical: below `2 p`
#[inline(always)]
fn fp2_sum<const N: usize>(a: Fp2Limbs<N>, b: Fp2Limbs<N>) -> [[u64; N]; 2] {
    [add_narrow(a[0], b[0]), add_narrow(a[1], b[1])]
}

/// `(a + a')(b + b')` in `Fp2`, for canonical components
#[inline(always)]
fn fp2_product_of_sums<const N: usize>(
    a: [Fp2Limbs<N>; 2],
    b: [Fp2Limbs<N>; 2],
    params: &Params<N>,
) -> Wide2<N> {
    let s = fp2_sum(a[0], a[1]);
    let t = fp2_sum(b[0], b[1]);
    fp2_product([&s[0], &s[1]], [&t[0], &t[1]], params)
}

/// `w *= ξ` for `ξ = 1 + u`, the non-residue of the cubic extension `Fp6 = Fp2[v] / (v³ - ξ)`
/// of bls12-381: `(c0 - c1) + (c0 + c1) u`
#[inline(always)]
fn mul_by_xi<const N: usize>(w: &mut Wide2<N>, params: &Params<N>) {
    let c0 = w.c0;
    w.c0.sub_mod(&w.c1, params);
    w.c1.add_mod(&c0, params);
}

/// `a b` in `Fp6` with `ξ = 1 + u` (Karatsuba), for canonical components: 18 base products, no
/// reduction
#[inline(always)]
pub(crate) fn fp6_product<const N: usize>(
    a: [Fp2Limbs<N>; 3],
    b: [Fp2Limbs<N>; 3],
    params: &Params<N>,
) -> [Wide2<N>; 3] {
    let ad = fp2_product(a[0], b[0], params);
    let be = fp2_product(a[1], b[1], params);
    let cf = fp2_product(a[2], b[2], params);
    // x = (a1 + a2)(b1 + b2) - be - cf
    let mut x = fp2_product_of_sums([a[1], a[2]], [b[1], b[2]], params);
    x.sub_mod(&be, params);
    x.sub_mod(&cf, params);
    // y = (a0 + a1)(b0 + b1) - ad - be
    let mut y = fp2_product_of_sums([a[0], a[1]], [b[0], b[1]], params);
    y.sub_mod(&ad, params);
    y.sub_mod(&be, params);
    // c2 = (a0 + a2)(b0 + b2) - ad + be - cf
    let mut c2 = fp2_product_of_sums([a[0], a[2]], [b[0], b[2]], params);
    c2.sub_mod(&ad, params);
    c2.add_mod(&be, params);
    c2.sub_mod(&cf, params);
    // c0 = ad + ξ x
    mul_by_xi(&mut x, params);
    x.add_mod(&ad, params);
    // c1 = y + ξ cf
    let mut c1 = cf;
    mul_by_xi(&mut c1, params);
    c1.add_mod(&y, params);
    [x, c1, c2]
}

/// `a (c0 + c1 v)` in `Fp6` with `ξ = 1 + u`, for canonical components: 15 base products, no
/// reduction
#[inline(always)]
pub(crate) fn fp6_product_by_01<const N: usize>(
    a: [Fp2Limbs<N>; 3],
    c0: Fp2Limbs<N>,
    c1: Fp2Limbs<N>,
    params: &Params<N>,
) -> [Wide2<N>; 3] {
    let a_a = fp2_product(a[0], c0, params);
    let b_b = fp2_product(a[1], c1, params);
    // t1 = ξ (c1 (a1 + a2) - b_b) + a_a
    let s = fp2_sum(a[1], a[2]);
    let mut t1 = fp2_product([&s[0], &s[1]], c1, params);
    t1.sub_mod(&b_b, params);
    mul_by_xi(&mut t1, params);
    t1.add_mod(&a_a, params);
    // t3 = c0 (a0 + a2) - a_a + b_b
    let s = fp2_sum(a[0], a[2]);
    let mut t3 = fp2_product([&s[0], &s[1]], c0, params);
    t3.sub_mod(&a_a, params);
    t3.add_mod(&b_b, params);
    // t2 = (c0 + c1)(a0 + a1) - a_a - b_b
    let mut t2 = fp2_product_of_sums([a[0], a[1]], [c0, c1], params);
    t2.sub_mod(&a_a, params);
    t2.sub_mod(&b_b, params);
    [t1, t2, t3]
}

/// `a (c1 v)` in `Fp6` with `ξ = 1 + u`: `(ξ a2 c1, a0 c1, a1 c1)`, 9 base products, no
/// reduction
#[inline(always)]
pub(crate) fn fp6_product_by_1<const N: usize>(
    a: [Fp2Limbs<N>; 3],
    c1: Fp2Limbs<N>,
    params: &Params<N>,
) -> [Wide2<N>; 3] {
    let mut t = fp2_product(a[2], c1, params);
    mul_by_xi(&mut t, params);
    [
        t,
        fp2_product(a[0], c1, params),
        fp2_product(a[1], c1, params),
    ]
}

/// The products on the towers over arkworks' `Fp`, whose limbs are canonical: each coefficient
/// of the result is reduced once and stored
pub(crate) mod towers {
    use super::{
        fp2_product, fp2_square, fp6_product, fp6_product_by_01, fp6_product_by_1, mul, Fp2Limbs,
        Params, Wide2,
    };
    use ark_ff::{Fp, Fp2, Fp2Config, Fp6, Fp6Config, MontBackend, MontConfig};

    /// The parameters of the base field of the configuration `T`
    struct Base<T, const N: usize>(core::marker::PhantomData<T>);

    impl<T: MontConfig<N>, const N: usize> Base<T, N> {
        const PARAMS: Params<N> = Params::new(T::MODULUS.0);
    }

    #[inline(always)]
    fn limbs<T, P, const N: usize>(a: &Fp2<P>) -> Fp2Limbs<'_, N>
    where
        T: MontConfig<N>,
        P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
    {
        [&a.c0.0 .0, &a.c1.0 .0]
    }

    #[inline(always)]
    fn limbs6<T, P, const N: usize>(a: &Fp6<P>) -> [Fp2Limbs<'_, N>; 3]
    where
        T: MontConfig<N>,
        P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
    {
        [limbs(&a.c0), limbs(&a.c1), limbs(&a.c2)]
    }

    #[inline(always)]
    fn store<T, P, const N: usize>(dst: &mut Fp2<P>, w: Wide2<N>)
    where
        T: MontConfig<N>,
        P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
    {
        let [c0, c1] = w.reduce(&Base::<T, N>::PARAMS);
        dst.c0.0 .0 = c0;
        dst.c1.0 .0 = c1;
    }

    #[inline(always)]
    fn store6<T, P, const N: usize>(dst: &mut Fp6<P>, w: [Wide2<N>; 3])
    where
        T: MontConfig<N>,
        P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
    {
        let [c0, c1, c2] = w;
        store(&mut dst.c0, c0);
        store(&mut dst.c1, c1);
        store(&mut dst.c2, c2);
    }

    /// Whether the non-residue of the cubic extension is `1 + u`, the one of the `Fp6` products
    pub(super) fn xi_is_one_plus_u<P: Fp6Config>() -> bool {
        use ark_ff::Field;
        P::NONRESIDUE == Fp2::new(Field::ONE, Field::ONE)
    }

    /// The element with the limbs `limbs` (its Montgomery form), `None` unless they are
    /// canonical
    #[cfg(any(feature = "hinted_operations", test))]
    #[inline(always)]
    pub(crate) fn from_limbs<T: MontConfig<N>, const N: usize>(
        limbs: [u64; N],
    ) -> Option<Fp<MontBackend<T, N>, N>> {
        super::is_canonical(&limbs, &Base::<T, N>::PARAMS)
            .then(|| Fp::new_unchecked(ark_ff::BigInt(limbs)))
    }

    /// `a *= b`
    #[inline(always)]
    pub(crate) fn fp_mul_assign<T: MontConfig<N>, const N: usize>(
        a: &mut Fp<MontBackend<T, N>, N>,
        b: &Fp<MontBackend<T, N>, N>,
    ) {
        a.0 .0 = mul(&a.0 .0, &b.0 .0, &Base::<T, N>::PARAMS);
    }

    /// `a -= b`
    #[inline(always)]
    pub(crate) fn fp_sub_assign<T: MontConfig<N>, const N: usize>(
        a: &mut Fp<MontBackend<T, N>, N>,
        b: &Fp<MontBackend<T, N>, N>,
    ) {
        a.0 .0 = super::sub_mod(&a.0 .0, &b.0 .0, &Base::<T, N>::PARAMS);
    }

    /// Whether `a b = product`
    #[cfg(any(feature = "hinted_operations", test))]
    #[inline(always)]
    pub(crate) fn is_product<T: MontConfig<N>, const N: usize>(
        a: &Fp<MontBackend<T, N>, N>,
        b: &Fp<MontBackend<T, N>, N>,
        product: &Fp<MontBackend<T, N>, N>,
    ) -> bool {
        // both canonical
        mul(&a.0 .0, &b.0 .0, &Base::<T, N>::PARAMS) == product.0 .0
    }

    /// `a *= b` in a quadratic extension with `u² = -1`
    pub(crate) fn fp2_mul_assign<T, P, const N: usize>(a: &mut Fp2<P>, b: &Fp2<P>)
    where
        T: MontConfig<N>,
        P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
    {
        let w = fp2_product(limbs(a), limbs(b), &Base::<T, N>::PARAMS);
        store(a, w);
    }

    /// `a = a²` in a quadratic extension with `u² = -1`
    pub(crate) fn fp2_square_in_place<T, P, const N: usize>(a: &mut Fp2<P>)
    where
        T: MontConfig<N>,
        P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
    {
        let w = fp2_square(limbs(a), &Base::<T, N>::PARAMS);
        store(a, w);
    }

    /// `a *= b` in a cubic extension with `ξ = 1 + u` over such a quadratic one
    pub(crate) fn fp6_mul_assign<T, P, const N: usize>(a: &mut Fp6<P>, b: &Fp6<P>)
    where
        T: MontConfig<N>,
        P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
    {
        debug_assert!(xi_is_one_plus_u::<P>());
        let w = fp6_product(limbs6(a), limbs6(b), &Base::<T, N>::PARAMS);
        store6(a, w);
    }

    /// `a *= c0 + c1 v`, as [`fp6_mul_assign`]
    pub(crate) fn fp6_mul_by_01<T, P, const N: usize>(
        a: &mut Fp6<P>,
        c0: &Fp2<P::Fp2Config>,
        c1: &Fp2<P::Fp2Config>,
    ) where
        T: MontConfig<N>,
        P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
    {
        debug_assert!(xi_is_one_plus_u::<P>());
        let w = fp6_product_by_01(limbs6(a), limbs(c0), limbs(c1), &Base::<T, N>::PARAMS);
        store6(a, w);
    }

    /// `a *= c1 v`, as [`fp6_mul_assign`]
    pub(crate) fn fp6_mul_by_1<T, P, const N: usize>(a: &mut Fp6<P>, c1: &Fp2<P::Fp2Config>)
    where
        T: MontConfig<N>,
        P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
    {
        debug_assert!(xi_is_one_plus_u::<P>());
        let w = fp6_product_by_1(limbs6(a), limbs(c1), &Base::<T, N>::PARAMS);
        store6(a, w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;
    use num_traits::{One, Zero};
    use std::vec::Vec;

    /// A base field of `N` limbs, and the reference arithmetic of its tower (`u² = -1`,
    /// `ξ = 1 + u`) on integers
    struct Tower<const N: usize> {
        params: Params<N>,
        p: BigUint,
        r: BigUint,
    }

    type Big2 = (BigUint, BigUint);

    fn big<const N: usize>(limbs: &[u64; N]) -> BigUint {
        let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
        BigUint::from_bytes_le(&bytes)
    }

    fn limbs<const N: usize>(value: &BigUint) -> [u64; N] {
        let digits = value.to_u64_digits();
        assert!(digits.len() <= N);
        core::array::from_fn(|i| digits.get(i).copied().unwrap_or(0))
    }

    impl<const N: usize> Tower<N> {
        fn new(p: [u64; N]) -> Self {
            Self {
                params: Params::new(p),
                p: big(&p),
                r: BigUint::one() << (64 * N),
            }
        }

        fn wide(&self, value: &BigUint) -> Wide<N> {
            Wide {
                lo: limbs(&(value % &self.r)),
                hi: limbs(&(value / &self.r)),
            }
        }

        fn value(&self, wide: &Wide<N>) -> BigUint {
            big(&wide.lo) + big(&wide.hi) * &self.r
        }

        /// `p R`, the bound of the wide values
        fn bound(&self) -> BigUint {
            &self.p * &self.r
        }

        fn sub(&self, a: &BigUint, b: &BigUint) -> BigUint {
            ((a % &self.p) + &self.p - (b % &self.p)) % &self.p
        }

        fn mul2(&self, a: &Big2, b: &Big2) -> Big2 {
            (
                self.sub(&(&a.0 * &b.0), &(&a.1 * &b.1)),
                (&a.0 * &b.1 + &a.1 * &b.0) % &self.p,
            )
        }

        fn add2(&self, a: &Big2, b: &Big2) -> Big2 {
            ((&a.0 + &b.0) % &self.p, (&a.1 + &b.1) % &self.p)
        }

        fn mul_xi(&self, a: &Big2) -> Big2 {
            self.mul2(a, &(BigUint::one(), BigUint::one()))
        }

        /// The schoolbook product in `Fp6`
        fn mul6(&self, a: &[Big2; 3], b: &[Big2; 3]) -> [Big2; 3] {
            let m = |i: usize, j: usize| self.mul2(&a[i], &b[j]);
            [
                self.add2(&m(0, 0), &self.mul_xi(&self.add2(&m(1, 2), &m(2, 1)))),
                self.add2(&self.add2(&m(0, 1), &m(1, 0)), &self.mul_xi(&m(2, 2))),
                self.add2(&self.add2(&m(0, 2), &m(1, 1)), &m(2, 0)),
            ]
        }

        /// The wide value is within its bound and of the class of `expected`
        fn assert_wide(&self, wide: &Wide<N>, expected: &BigUint) {
            let value = self.value(wide);
            assert!(value < self.bound());
            assert_eq!(value % &self.p, expected % &self.p);
        }

        fn assert_wide2(&self, wide: &Wide2<N>, expected: &Big2) {
            self.assert_wide(&wide.c0, &expected.0);
            self.assert_wide(&wide.c1, &expected.1);
        }

        /// Canonical elements: the extremes, and `count` pseudo-random ones
        fn elements(&self, count: usize) -> Vec<BigUint> {
            let mut elements = std::vec![
                BigUint::zero(),
                BigUint::one(),
                &self.p - 1u32,
                &self.p - 2u32,
                &self.p >> 1,
                (BigUint::one() << (64 * N - 3)) % &self.p,
                BigUint::from(u64::MAX),
            ];
            let mut state = 0x9e37_79b9_7f4a_7c15u64;
            for _ in 0..count {
                elements.push(self.random(&mut state) % &self.p);
            }
            elements
        }

        /// A pseudo-random integer below `R²` (SplitMix64)
        fn random(&self, state: &mut u64) -> BigUint {
            let mut value = BigUint::zero();
            for _ in 0..2 * N {
                *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
                let mut z = *state;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
                value = (value << 64) + (z ^ (z >> 31));
            }
            value
        }

        /// Wide values below `p R`: the extremes, and `count` pseudo-random ones
        fn wides(&self, count: usize) -> Vec<BigUint> {
            let bound = self.bound();
            let mut wides = std::vec![
                BigUint::zero(),
                BigUint::one(),
                &bound - 1u32,
                &bound >> 1,
                (&bound >> 1) + 1u32,
                self.r.clone(),
                &self.r - 1u32,
                &bound - &self.r,
                &self.p * &self.p,
                (&self.p - 1u32) * (&self.p - 1u32),
            ];
            let mut state = 0x1234_5678_9abc_def1u64;
            for _ in 0..count {
                wides.push(self.random(&mut state) % &bound);
            }
            wides
        }

        fn check_wide_arithmetic(&self) {
            let bound = self.bound();
            let wides = self.wides(24);
            for x in &wides {
                // the reduction: `x / R mod p`, canonical
                let reduced = big(&self.wide(x).reduce(&self.params));
                assert!(reduced < self.p);
                assert_eq!((reduced * &self.r) % &self.p, x % &self.p);

                for y in &wides {
                    let mut sum = self.wide(x);
                    sum.add_mod(&self.wide(y), &self.params);
                    assert_eq!(self.value(&sum), (x + y) % &bound);
                    let mut difference = self.wide(x);
                    difference.sub_mod(&self.wide(y), &self.params);
                    assert_eq!(self.value(&difference), (x + &bound - y) % &bound);
                }
            }
        }

        /// The products of operands up to the bound of a Karatsuba sum, `4 p`
        fn check_products(&self) {
            let mut operands = self.elements(8);
            operands.extend([
                &self.p * 2u32 - 1u32,
                &self.p * 4u32 - 1u32,
                &self.p * 4u32 - 2u32,
                &self.r - 1u32,
            ]);
            for a in &operands {
                for b in &operands {
                    let product = Wide::product(&limbs::<N>(a), &limbs::<N>(b));
                    assert_eq!(self.value(&product), a * b);
                }
            }
        }

        /// `Fp2` products of components up to the bound, `2 p - 1`
        fn check_fp2(&self) {
            let mut components = self.elements(4);
            components.extend([self.p.clone(), &self.p * 2u32 - 2u32, &self.p * 2u32 - 1u32]);
            let mut folded = false;
            for a0 in &components {
                for a1 in &components {
                    for b0 in &components {
                        for b1 in &components {
                            let (a, b) = ((a0.clone(), a1.clone()), (b0.clone(), b1.clone()));
                            let la = [limbs::<N>(a0), limbs(a1)];
                            let lb = [limbs::<N>(b0), limbs(b1)];
                            let product =
                                fp2_product([&la[0], &la[1]], [&lb[0], &lb[1]], &self.params);
                            self.assert_wide2(&product, &self.mul2(&a, &b));
                            folded |= a0 * b1 + a1 * b0 >= self.bound();
                        }
                    }
                }
            }
            // the extremes reach the fold of the cross term exactly where the modulus has it
            assert_eq!(folded, self.params.fold);

            // the squares, of canonical components
            let components = self.elements(8);
            for a0 in &components {
                for a1 in &components {
                    let a = (a0.clone(), a1.clone());
                    let la = [limbs::<N>(a0), limbs(a1)];
                    let square = fp2_square([&la[0], &la[1]], &self.params);
                    self.assert_wide2(&square, &self.mul2(&a, &a));
                }
            }
        }

        fn fp2_limbs(a: &Big2) -> [[u64; N]; 2] {
            [limbs(&a.0), limbs(&a.1)]
        }

        fn check_fp6_case(&self, a: &[Big2; 3], b: &[Big2; 3]) {
            let la = a.clone().map(|c| Self::fp2_limbs(&c));
            let lb = b.clone().map(|c| Self::fp2_limbs(&c));
            let ra: [Fp2Limbs<N>; 3] = core::array::from_fn(|i| [&la[i][0], &la[i][1]]);
            let rb: [Fp2Limbs<N>; 3] = core::array::from_fn(|i| [&lb[i][0], &lb[i][1]]);
            let zero = (BigUint::zero(), BigUint::zero());

            // ξ on a wide value
            let mut w = fp2_product(ra[0], rb[0], &self.params);
            mul_by_xi(&mut w, &self.params);
            self.assert_wide2(&w, &self.mul_xi(&self.mul2(&a[0], &b[0])));

            let expected = self.mul6(a, b);
            let product = fp6_product(ra, rb, &self.params);
            for (wide, expected) in product.iter().zip(&expected) {
                self.assert_wide2(wide, expected);
                // reduced: the Montgomery form of the product of the Montgomery forms
                let reduced = wide.reduce(&self.params);
                assert!(big(&reduced[0]) < self.p && big(&reduced[1]) < self.p);
                assert_eq!((big(&reduced[0]) * &self.r) % &self.p, expected.0);
                assert_eq!((big(&reduced[1]) * &self.r) % &self.p, expected.1);
            }

            let sparse = [b[0].clone(), b[1].clone(), zero.clone()];
            let product = fp6_product_by_01(ra, rb[0], rb[1], &self.params);
            for (wide, expected) in product.iter().zip(&self.mul6(a, &sparse)) {
                self.assert_wide2(wide, expected);
            }

            let sparse = [zero.clone(), b[1].clone(), zero];
            let product = fp6_product_by_1(ra, rb[1], &self.params);
            for (wide, expected) in product.iter().zip(&self.mul6(a, &sparse)) {
                self.assert_wide2(wide, expected);
            }
        }

        fn check_fp6(&self) {
            // every pattern of the extremes of the components: zero and `p - 1`
            let extremes = [BigUint::zero(), &self.p - 1u32];
            let element = |pattern: usize| -> [Big2; 3] {
                core::array::from_fn(|i| {
                    (
                        extremes[(pattern >> (2 * i)) & 1].clone(),
                        extremes[(pattern >> (2 * i + 1)) & 1].clone(),
                    )
                })
            };
            for pattern_a in 0..64 {
                for pattern_b in 0..64 {
                    self.check_fp6_case(&element(pattern_a), &element(pattern_b));
                }
            }
            // and elements mixing the extremes with pseudo-random components
            let elements = self.elements(40);
            let pick = |seed: usize| -> [Big2; 3] {
                core::array::from_fn(|i| {
                    (
                        elements[(seed * 7 + 2 * i * 5) % elements.len()].clone(),
                        elements[(seed * 11 + (2 * i + 1) * 3) % elements.len()].clone(),
                    )
                })
            };
            for seed in 0..120 {
                self.check_fp6_case(&pick(seed), &pick(seed + 61));
            }
        }
    }

    /// The modulus of bn254 under the tower of bls12-381 (`ξ = 1 + u`): the `Fp6` products are
    /// used with the modulus of bls12-381 only, and identities of the ring for any modulus,
    /// which this one checks where `8 p >= R` (the fold of the cross terms)
    fn bn254() -> Tower<4> {
        Tower::new(<ark_bn254::Fq as ark_ff::PrimeField>::MODULUS.0)
    }

    fn bls12_381() -> Tower<6> {
        Tower::new(<ark_bls12_381::Fq as ark_ff::PrimeField>::MODULUS.0)
    }

    /// The products on arkworks' towers of the two curves against arkworks' own
    mod on_arkworks_towers {
        use super::super::towers;
        use ark_ff::{BigInt, Field, Fp, Fp2, Fp2Config, Fp6, Fp6Config, MontBackend, MontConfig};
        use ark_ff::{PrimeField, UniformRand, Zero};
        use std::vec::Vec;

        /// Base field elements: the extremes of the limbs (0, 1, `p - 1` and its neighbour as
        /// they are stored, not as values) and random ones
        fn base<T: MontConfig<N>, const N: usize>() -> Vec<Fp<MontBackend<T, N>, N>> {
            let mut rng = ark_std::test_rng();
            let mut top = T::MODULUS;
            top.0[0] -= 1;
            let mut below = top;
            below.0[0] -= 1;
            let mut one = BigInt::<N>::zero();
            one.0[0] = 1;
            let mut elements = std::vec![
                Fp::new_unchecked(BigInt::zero()),
                Fp::new_unchecked(one),
                Fp::new_unchecked(top),
                Fp::new_unchecked(below),
                Fp::ONE,
                -Fp::<MontBackend<T, N>, N>::ONE,
            ];
            elements.extend((0..6).map(|_| Fp::rand(&mut rng)));
            assert!(elements.iter().all(|e| e.0 < T::MODULUS));
            elements
        }

        /// `Fp2` elements over `base`: every pair of the extremes, and mixed ones
        fn fp2<T, P, const N: usize>() -> Vec<Fp2<P>>
        where
            T: MontConfig<N>,
            P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
        {
            let base = base::<T, N>();
            let mut elements = Vec::new();
            for c0 in &base[..4] {
                for c1 in &base[..4] {
                    elements.push(Fp2::new(*c0, *c1));
                }
            }
            for (i, c0) in base.iter().enumerate() {
                elements.push(Fp2::new(*c0, base[(5 * i + 3) % base.len()]));
            }
            elements
        }

        fn check_fp2<T, P, const N: usize>()
        where
            T: MontConfig<N>,
            P: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>,
        {
            let elements = fp2::<T, P, N>();
            for a in &elements {
                for b in &elements {
                    let mut product = *a;
                    towers::fp2_mul_assign(&mut product, b);
                    assert_eq!(product, *a * *b);
                    assert!(product.c0.0 < T::MODULUS && product.c1.0 < T::MODULUS);
                }
                let mut square = *a;
                towers::fp2_mul_assign(&mut square, &a.clone());
                assert_eq!(square, a.square());
                let mut square = *a;
                towers::fp2_square_in_place(&mut square);
                assert_eq!(square, a.square());
                assert!(square.c0.0 < T::MODULUS && square.c1.0 < T::MODULUS);
            }
        }

        fn check_fp6<T, P, const N: usize>()
        where
            T: MontConfig<N>,
            P: Fp6Config<Fp2Config: Fp2Config<Fp = Fp<MontBackend<T, N>, N>>>,
        {
            let fp2 = fp2::<T, P::Fp2Config, N>();
            let element = |i: usize| {
                Fp6::<P>::new(
                    fp2[i % fp2.len()],
                    fp2[(7 * i + 5) % fp2.len()],
                    fp2[(11 * i + 2) % fp2.len()],
                )
            };
            // the extremes in every component, then the mixed elements
            let mut elements: Vec<Fp6<P>> =
                [0, 2, 10].map(|i| Fp6::new(fp2[i], fp2[i], fp2[i])).into();
            elements.extend((0..40).map(element));
            for (i, a) in elements.iter().enumerate() {
                for b in elements.iter().skip(i % 3).step_by(3) {
                    let mut product = *a;
                    towers::fp6_mul_assign(&mut product, b);
                    assert_eq!(product, *a * *b);

                    let (c0, c1) = (b.c0, b.c2);
                    let mut product = *a;
                    towers::fp6_mul_by_01(&mut product, &c0, &c1);
                    let mut expected = *a;
                    expected.mul_by_01(&c0, &c1);
                    assert_eq!(product, expected);

                    let mut product = *a;
                    towers::fp6_mul_by_1(&mut product, &c1);
                    let mut expected = *a;
                    expected.mul_by_1(&c1);
                    assert_eq!(product, expected);
                }
            }
        }

        fn check_base<T: MontConfig<N>, const N: usize>() {
            let elements = base::<T, N>();
            for a in &elements {
                assert_eq!(towers::from_limbs::<T, N>(a.0 .0), Some(*a));
                for b in &elements {
                    let product = *a * *b;
                    let mut inline = *a;
                    towers::fp_mul_assign(&mut inline, b);
                    assert_eq!(inline, product);
                    assert!(inline.0 < T::MODULUS);
                    let mut difference = *a;
                    towers::fp_sub_assign(&mut difference, b);
                    assert_eq!(difference, *a - *b);
                    assert!(difference.0 < T::MODULUS);
                    assert!(towers::is_product(a, b, &product));
                    assert!(!towers::is_product(a, b, &(product + Fp::ONE)));
                    assert!(!towers::is_product(a, b, &-product) || product.is_zero());
                }
            }
            // the modulus and what is above it are not elements
            let mut limbs = T::MODULUS.0;
            assert_eq!(towers::from_limbs::<T, N>(limbs), None);
            limbs[0] += 1;
            assert_eq!(towers::from_limbs::<T, N>(limbs), None);
            limbs[0] -= 2;
            assert!(towers::from_limbs::<T, N>(limbs).is_some());
            limbs[N - 1] += 1;
            assert_eq!(towers::from_limbs::<T, N>(limbs), None);
            assert_eq!(towers::from_limbs::<T, N>([u64::MAX; N]), None);
        }

        #[test]
        fn base_field_products() {
            check_base::<ark_bn254::FqConfig, 4>();
            check_base::<ark_bls12_381::FqConfig, 6>();
        }

        #[test]
        fn fp2_products() {
            check_fp2::<ark_bn254::FqConfig, ark_bn254::Fq2Config, 4>();
            check_fp2::<ark_bls12_381::FqConfig, ark_bls12_381::Fq2Config, 6>();
        }

        #[test]
        fn fp6_products() {
            check_fp6::<ark_bls12_381::FqConfig, ark_bls12_381::Fq6Config, 6>();
        }

        /// The `Fp6` products are those of `ξ = 1 + u`: the tower of bls12-381, not of bn254
        #[test]
        fn the_non_residue_of_the_fp6_products() {
            assert!(towers::xi_is_one_plus_u::<ark_bls12_381::Fq6Config>());
            assert!(!towers::xi_is_one_plus_u::<ark_bn254::Fq6Config>());
            let modulus = <ark_bn254::Fq as PrimeField>::MODULUS;
            assert_eq!(modulus.0[3] >> 61, 1);
        }
    }

    #[test]
    fn parameters() {
        let (bn254, bls12_381) = (bn254(), bls12_381());
        // `8 p >= R` for bn254 only
        assert!(bn254.params.fold && !bls12_381.params.fold);
        assert!(&bn254.p * 8u32 >= bn254.r && &bn254.p * 4u32 < bn254.r);
        assert!(&bls12_381.p * 8u32 < bls12_381.r);
        // `p inv = -1 mod 2^64`
        assert_eq!(bn254.params.p[0].wrapping_mul(bn254.params.inv), u64::MAX);
        assert_eq!(
            bls12_381.params.p[0].wrapping_mul(bls12_381.params.inv),
            u64::MAX
        );
    }

    #[test]
    fn wide_arithmetic_matches_big_integers() {
        bn254().check_wide_arithmetic();
        bls12_381().check_wide_arithmetic();
    }

    #[test]
    fn products_match_big_integers() {
        bn254().check_products();
        bls12_381().check_products();
    }

    #[test]
    fn fp2_products_stay_within_the_bound() {
        bn254().check_fp2();
        bls12_381().check_fp2();
    }

    #[test]
    fn fp6_products_stay_within_the_bound() {
        bn254().check_fp6();
        bls12_381().check_fp6();
    }
}
