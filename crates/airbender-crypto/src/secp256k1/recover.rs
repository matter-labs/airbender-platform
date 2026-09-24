use crate::k256::{
    elliptic_curve::{
        bigint::{CheckedAdd, U256},
        Curve, FieldBytesEncoding,
    },
    Secp256k1,
};

use core::{cfg_select, mem::MaybeUninit};

use super::{
    context::{ECMultContext, ECMULT_TABLE_SIZE_A, WINDOW_A, WINDOW_G, WNAF_BITS},
    field::FieldElement,
    points::{Affine, Jacobian},
    scalars::Scalar,
    Secp256k1Err,
};

#[cfg(feature = "secp256k1-static-context")]
pub fn recover(
    message: &crate::k256::Scalar,
    signature: &crate::k256::ecdsa::Signature,
    recovery_id: &crate::k256::ecdsa::RecoveryId,
) -> Result<Affine, Secp256k1Err> {
    use super::context::ECRECOVER_CONTEXT;

    recover_with_context(message, signature, recovery_id, &ECRECOVER_CONTEXT)
}

pub fn recover_with_context(
    message: &crate::k256::Scalar,
    signature: &crate::k256::ecdsa::Signature,
    recovery_id: &crate::k256::ecdsa::RecoveryId,
    context: &ECMultContext,
) -> Result<Affine, Secp256k1Err> {
    recover_with_context_and_hooks(
        message,
        signature,
        recovery_id,
        context,
        &mut super::hooks::DefaultSecp256k1Hooks,
    )
}

#[cfg(feature = "secp256k1-static-context")]
pub fn recover_with_hooks<H: super::hooks::Secp256k1Hooks>(
    message: &crate::k256::Scalar,
    signature: &crate::k256::ecdsa::Signature,
    recovery_id: &crate::k256::ecdsa::RecoveryId,
    hooks: &mut H,
) -> Result<Affine, Secp256k1Err> {
    use super::context::ECRECOVER_CONTEXT;

    recover_with_context_and_hooks(message, signature, recovery_id, &ECRECOVER_CONTEXT, hooks)
}

pub fn recover_with_context_and_hooks<H: super::hooks::Secp256k1Hooks>(
    message: &crate::k256::Scalar,
    signature: &crate::k256::ecdsa::Signature,
    recovery_id: &crate::k256::ecdsa::RecoveryId,
    context: &ECMultContext,
    hooks: &mut H,
) -> Result<Affine, Secp256k1Err> {
    let (mut sigr, mut sigs) = Scalar::from_signature(signature);
    let message = Scalar::from_k256_scalar(*message);

    // We go through bytes because it's mod GROUP_ORDER and later we need mod BASE FIELD
    let mut brx = sigr.to_repr();

    if recovery_id.is_x_reduced() {
        match <U256 as FieldBytesEncoding<Secp256k1>>::decode_field_bytes(&brx)
            .checked_add(&Secp256k1::ORDER)
            .into_option()
        {
            Some(restored) => {
                brx = <U256 as FieldBytesEncoding<Secp256k1>>::encode_field_bytes(&restored);
            }
            None => return Err(Secp256k1Err::OperationOverflow),
        }
    }

    let is_odd = recovery_id.is_y_odd();
    let x =
        Affine::decompress_with_hooks(&brx, is_odd, hooks).ok_or(Secp256k1Err::InvalidParams)?;

    let xj = x.to_jacobian();

    hooks.scalar_invert_and_assign(&mut sigr);
    sigs *= sigr;

    sigr *= message;
    sigr.negate_in_place();

    let mut pk = ecmult_with_hooks(&xj, &sigs, &sigr, context, hooks).to_affine_with_hooks(hooks);
    pk.normalize_in_place();

    if pk.is_infinity() {
        return Err(Secp256k1Err::RecoveredInfinity);
    }

    Ok(pk)
}

/// Runs both flavours (with and without cheap inversions) and checks that they agree
#[cfg(test)]
fn ecmult(a: &Jacobian, na: &Scalar, ng: &Scalar, context: &ECMultContext) -> Jacobian {
    use super::hooks::{DefaultSecp256k1Hooks, Secp256k1Hooks};

    struct CheapInversionHooks;

    impl Secp256k1Hooks for CheapInversionHooks {
        const FE_INVERT_IS_CHEAP: bool = true;

        fn fe_sqrt_and_assign(&mut self, fe: &mut FieldElement) -> bool {
            DefaultSecp256k1Hooks.fe_sqrt_and_assign(fe)
        }

        fn fe_invert_and_assign(&mut self, fe: &mut FieldElement) {
            DefaultSecp256k1Hooks.fe_invert_and_assign(fe)
        }

        fn scalar_invert_and_assign(&mut self, scalar: &mut Scalar) {
            DefaultSecp256k1Hooks.scalar_invert_and_assign(scalar)
        }
    }

    let ret = ecmult_with_hooks(a, na, ng, context, &mut DefaultSecp256k1Hooks);
    let with_inversions = ecmult_with_hooks(a, na, ng, context, &mut CheapInversionHooks);
    assert_eq!(ret.to_affine(), with_inversions.to_affine());

    ret
}

/// Compute na*a+ng*g where g is the generator.
fn ecmult_with_hooks<H: super::hooks::Secp256k1Hooks>(
    a: &Jacobian,
    na: &Scalar,
    ng: &Scalar,
    context: &ECMultContext,
    hooks: &mut H,
) -> Jacobian {
    #[cfg(all(feature = "secp256k1-shamir-msm", feature = "bigint_ops"))]
    {
        let _ = context;
        super::recover_shamir::ecmult(a, na, ng, hooks)
    }

    #[cfg(not(all(feature = "secp256k1-shamir-msm", feature = "bigint_ops")))]
    ecmult_wnaf(a, na, ng, context, hooks)
}

/// Algorithm adapted from https://github.com/bitcoin-core/secp256k1/blob/master/src/ecmult_impl.h#L237
#[cfg_attr(
    all(feature = "secp256k1-shamir-msm", feature = "bigint_ops"),
    allow(dead_code)
)]
fn ecmult_wnaf<H: super::hooks::Secp256k1Hooks>(
    a: &Jacobian,
    na: &Scalar,
    ng: &Scalar,
    context: &ECMultContext,
    hooks: &mut H,
) -> Jacobian {
    // `z` is the denominator shared by the table of the multiples of `a`: those are handled as
    // affine points, and it is applied to the result at the end. The generator's tables have to
    // be brought to the same denominator on every use then. If inversion is cheap, the table
    // is made affine instead and `z` stays 1.
    let mut z = FieldElement::ONE;

    // The odd multiples of `a`, and `beta * x` of each: written only when `a` contributes,
    // and read only at a non-zero digit of `na`, which implies that
    let mut prea: [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A] =
        [const { MaybeUninit::uninit() }; ECMULT_TABLE_SIZE_A];
    let mut aux: [FieldElement; ECMULT_TABLE_SIZE_A] = [FieldElement::ZERO; ECMULT_TABLE_SIZE_A];

    // The digits are zero above the bits of their scalars, so a digit is looked at only for
    // being non-zero
    let mut wnaf_na_1 = [0 as WnafDigit; WNAF_BITS];
    let mut wnaf_na_lam = [0 as WnafDigit; WNAF_BITS];

    let mut wnaf_ng_1 = [0 as WnafDigit; WNAF_BITS];
    let mut wnaf_ng_128 = [0 as WnafDigit; WNAF_BITS];

    let mut bits = 0;

    let prea: &[Affine; ECMULT_TABLE_SIZE_A] = if !na.is_zero() && !a.is_infinity() {
        // split na into 129 bit scalars
        // where na_1 + na_lam * lambda = na

        // NOTE: this action moves integer representation to the normal form
        let (na_1, na_lam) = na.decompose();

        // build wnaf representation
        let bits_na_1 = wnaf(&mut wnaf_na_1, &na_1, WINDOW_A);
        let bits_na_lam = wnaf(&mut wnaf_na_lam, &na_lam, WINDOW_A);

        debug_assert!(bits_na_1 <= WNAF_BITS as i32);
        debug_assert!(bits_na_lam <= WNAF_BITS as i32);

        bits = bits_na_1.max(bits_na_lam);

        // Calculate odd multiples of a.
        // All multiples are brought to the same `z` "denominator".
        // Due to secp256k1 we can pretend the z coordinate is 1 and use affine addition formulas,
        // and correct the result at the end
        odd_multiples_table_windowa(&mut prea, &mut aux, &mut z, a);
        // SAFETY: `odd_multiples_table_windowa` wrote every entry
        let prea: &mut [Affine; ECMULT_TABLE_SIZE_A] =
            unsafe { &mut *(&mut prea as *mut [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A]).cast() };
        if H::FE_INVERT_IS_CHEAP {
            table_set_affine_windowa(prea, &aux, &z, hooks);
            z = FieldElement::ONE;
        } else {
            table_set_globalz_windowa(prea, &aux);
        }

        for i in 0..ECMULT_TABLE_SIZE_A {
            aux[i] = FieldElement::BETA;
            aux[i] *= &prea[i].x;
        }
        prea
    } else {
        // never read: every digit of `na` is zero
        // SAFETY: as above
        unsafe { &*(&prea as *const [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A]).cast() }
    };

    if !ng.is_zero() {
        // TODO: use u128 instead

        // split ng into ~128 bit scalars.
        // NOTE: it's NOT endomorphism decomposition, so it's 128 bits exact decomposition
        // where ng_1 + ng_128*2^128 = ng
        let (ng_1, ng_128) = ng.decompose_128(); // NOTE: must return normal form

        // build wnaf representation
        let bits_ng_1 = wnaf(&mut wnaf_ng_1, &ng_1, WINDOW_G);
        let bits_ng_128 = wnaf(&mut wnaf_ng_128, &ng_128, WINDOW_G);

        bits = bits.max(bits_ng_1).max(bits_ng_128);
    }

    let mut r = Jacobian::INFINITY;

    for i in (0..bits as usize).rev() {
        r.double_in_place(None);

        let n = wnaf_na_1[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_A);
            r.add_affine_in_place(&prea[idx].x, &prea[idx].y, negate, None);
        }

        let n = wnaf_na_lam[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_A);
            r.add_affine_in_place(&aux[idx], &prea[idx].y, negate, None);
        }

        let n = wnaf_ng_1[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_G);
            add_from_table(
                &mut r,
                &context.pre_g[idx],
                negate,
                &z,
                H::FE_INVERT_IS_CHEAP,
            );
        }

        let n = wnaf_ng_128[i];
        if n != 0 {
            let (idx, negate) = table_index(n, WINDOW_G);
            add_from_table(
                &mut r,
                &context.pre_g_128[idx],
                negate,
                &z,
                H::FE_INVERT_IS_CHEAP,
            );
        }
    }

    if !H::FE_INVERT_IS_CHEAP && !r.is_infinity() {
        r.z *= &z
    }

    r
}

/// A digit of a wNAF representation: `|digit| < 2^(w - 1)` for the windows in use
type WnafDigit = i16;
const _: () = assert!(WINDOW_A <= 16 && WINDOW_G <= 16);

/// `r += (x, y)` or `r -= (x, y)` for the stored generator multiple, an affine point if `affine`,
/// otherwise a point with the denominator `1 / z`
#[inline(always)]
fn add_from_table(
    r: &mut Jacobian,
    stored: &super::points::AffineStorage,
    negate: bool,
    z: &FieldElement,
    affine: bool,
) {
    let add = |r: &mut Jacobian, x: &FieldElement, y: &FieldElement| {
        if affine {
            r.add_affine_in_place(x, y, negate, None);
        } else {
            r.add_affine_zinv_in_place(x, y, negate, z);
        }
    };
    cfg_select! {
        // with the delegated field the table is read in place
        feature = "bigint_ops" => {
            let (x, y) = stored.coordinates();
            add(r, x, y);
        }
        _ => {
            let g = stored.to_affine();
            add(r, &g.x, &g.y);
        }
    }
}

/// Fill `pre_a` with odd multiples of a.
/// Although pre_a is an array of affine points, it actually represents elements in jacobian coordinates
/// with their z coordinate omitted. The omitted z-coordinate can be recovered with `zr` and `z`.
/// Using `b.z` to denote the omitted z-coordinate of b:
/// - `pre_a[n-1].z = z`
/// - `pre_a[i-1].z = pre_a[i].z / zr[i]` for `n > i > 0`
///
/// Lastly, `zr[0]` is set so that `a.z = pre_a[0].z / zr[0]`
/// Based on https://github.com/bitcoin-core/secp256k1/blob/master/src/ecmult_impl.h#L73
fn odd_multiples_table_windowa(
    pre_a: &mut [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A],
    zr: &mut [FieldElement; ECMULT_TABLE_SIZE_A],
    z: &mut FieldElement,
    a: &Jacobian,
) {
    debug_assert!(!a.is_infinity());

    let mut d = *a;
    d.double_in_place(None);

    // we perform additions using an isomorphic curve Y^2 = X^3 + 7*C^6 where  C := d.z
    // The isomorphism, phi, is given by (x,y) -> (x*C^2, y*C^3).
    // In Jacobian coordinates, phi is given by (x, y, z) -> (x*C^2, y*C^3, z) = (x, y, z/C)
    // So
    //      d_ge = phi(d) = (d.x, d.y, 1)
    //      ai = phi(a) = (a.x*C^2, a.y*C^3, a.z)
    // This lets us use the faster add_ge_var

    let first = pre_a[0].write(Affine::DEFAULT);
    first.set_gej_zinv(a, &d.z);

    let mut ai = Jacobian {
        x: first.x,
        y: first.y,
        z: a.z,
    };

    // pre_a[0] is the point (a.x*C^2, a.y*C^3, a.z*C) which is equivalent to a.
    // Set zr[0] to C, which is the ratio between the omitted z(pre_a[0]) value and a.z.
    zr[0] = d.z;

    for i in 1..ECMULT_TABLE_SIZE_A {
        // `d` as the affine point `(d.x, d.y)` of the isomorphic curve, read in place
        ai.add_affine_in_place(&d.x, &d.y, false, Some(&mut zr[i]));
        pre_a[i].write(Affine {
            x: ai.x,
            y: ai.y,
            infinity: false,
        });
    }

    // Multiply the last z-coordinate by C to undo the isomorphism.
    // Since the z-coordinates of the pre_a values are implied by the zr array of z-coordinate ratios,
    // undoing the isomorphism here undoes the isomorphism for all pre_a values.
    *z = ai.z;
    *z *= &d.z;
}

fn table_set_globalz_windowa(
    pre_a: &mut [Affine; ECMULT_TABLE_SIZE_A],
    zr: &[FieldElement; ECMULT_TABLE_SIZE_A],
) {
    let mut i = ECMULT_TABLE_SIZE_A - 1;

    pre_a[i].y.normalize_in_place();

    let mut zs = zr[i];

    i -= 1;

    let mut ai = pre_a[i];
    pre_a[i].set_ge_zinv(&ai, &zs);

    while i > 0 {
        zs *= &zr[i];
        i -= 1;

        ai = pre_a[i];
        pre_a[i].set_ge_zinv(&ai, &zs);
    }
}

/// Makes the table produced by `odd_multiples_table_windowa` affine by a single inversion:
/// `pre_a[n-1].z = z`, and `1 / pre_a[i-1].z = zr[i] / pre_a[i].z`
fn table_set_affine_windowa<H: super::hooks::Secp256k1Hooks>(
    pre_a: &mut [Affine; ECMULT_TABLE_SIZE_A],
    zr: &[FieldElement; ECMULT_TABLE_SIZE_A],
    z: &FieldElement,
    hooks: &mut H,
) {
    let mut zinv = *z;
    hooks.fe_invert_and_assign(&mut zinv);

    let mut i = ECMULT_TABLE_SIZE_A - 1;
    loop {
        let ai = pre_a[i];
        pre_a[i].set_ge_zinv(&ai, &zinv);
        if i == 0 {
            break;
        }

        zinv *= &zr[i];
        i -= 1;
    }
}

/// Convert a scalar to a wnaf representation,
/// i.e. `a=sum(2^i * wnaf[i])`, with te following guarantees:
///     - each `wnaf[i]` is either 0, or an odd integer between `-(1<<(w-1) - 1)` and `1<<(w-1) - 1`
///     - two non-zero entries are separated by at least `w-1` zeros
///     - the number of set values in wnaf is returned
///
/// NOTE: the function assumes that `wnaf` is zeroed
fn wnaf(wnaf: &mut [WnafDigit], s: &Scalar, w: usize) -> i32 {
    debug_assert!(wnaf.len() <= 224);
    debug_assert!((2..=16).contains(&w));
    debug_assert!(wnaf.iter().all(|&x| x == 0));

    let mut s = *s;

    let mut last_set_bit: i32 = -1;
    let mut bit = 0;
    let mut sign = 1;
    let mut carry = 0;

    if s.bits(255, 1) > 0 {
        // Negation is the same in any form
        s.negate_in_place();
        sign = -1;
    }
    // the scalar as 32-bit words: a bit test is then one shift on this target
    let limbs = s.limbs();
    let words: [u32; 8] = core::array::from_fn(|i| (limbs[i / 2] >> (32 * (i % 2))) as u32);

    while bit < wnaf.len() {
        if ((words[bit >> 5] >> (bit & 31)) & 1) as i32 == carry {
            bit += 1;
            continue;
        }

        let mut now = w;
        if now > wnaf.len() - bit {
            now = wnaf.len() - bit;
        }

        // the `now <= 16` bits from `bit` on, which may straddle two words
        let pair = words[bit >> 5] as u64 | ((words[(bit >> 5) + 1] as u64) << 32);
        let mut word = (((pair >> (bit & 31)) as u32) & ((1 << now) - 1)) as i32 + carry;

        carry = (word >> (w - 1)) & 1;
        word -= carry << w;

        wnaf[bit] = (sign * word) as WnafDigit;
        last_set_bit = bit as i32;

        bit += now;
    }
    debug_assert_eq!(carry, 0);
    debug_assert!({
        let mut t = true;
        while bit < 256 {
            t = t && (s.bits(bit, 1) == 0);
            bit += 1;
        }
        t
    });

    last_set_bit + 1
}

/// Position of the odd multiple `|n|` in the table, and whether it has to be negated
#[inline(always)]
fn table_index(n: WnafDigit, w: usize) -> (usize, bool) {
    let n = n as i32;
    debug_assert!(table_verify(n, w));

    if n > 0 {
        ((n - 1) as usize / 2, false)
    } else {
        ((-n - 1) as usize / 2, true)
    }
}

fn table_verify(n: i32, w: usize) -> bool {
    (2..=31).contains(&w) && ((n & 1) == 1) && (n >= -((1 << (w - 1)) - 1)) && (n < (1 << (w - 1)))
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::ecmult;
    use crate::secp256k1::scalars::Scalar;
    use crate::secp256k1::{context::ECRECOVER_CONTEXT, test_vectors::MUL_TEST_VECTORS};
    use crate::secp256k1::{
        field::FieldElement,
        points::{Affine, Jacobian},
    };

    use proptest::{prop_assert_eq, proptest};

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_0I_0G() {
        assert_eq!(ECRECOVER_CONTEXT.pre_g[0].to_affine(), Affine::GENERATOR);

        // 0*infinity + 0*G = 0
        let res = ecmult(
            &Jacobian::INFINITY,
            &Scalar::ZERO,
            &Scalar::ZERO,
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert!(res.is_infinity());
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_0I_1G() {
        // 0*infinity + 1*G = G
        let mut res = ecmult(
            &Jacobian::INFINITY,
            &Scalar::ZERO,
            &Scalar::ONE,
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        res.normalize_in_place();

        assert_eq!(res, Affine::GENERATOR);
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_0I_3G() {
        // 0*infinity + 3*G = 3*G
        let res = ecmult(
            &Jacobian::INFINITY,
            &Scalar::ZERO,
            &Scalar::from_u128(3),
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert_eq!(res, ECRECOVER_CONTEXT.pre_g[1].to_affine())
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_0I_5G() {
        let res = ecmult(
            &Jacobian::INFINITY,
            &Scalar::ZERO,
            &Scalar::from_u128(5),
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert_eq!(res, ECRECOVER_CONTEXT.pre_g[2].to_affine());
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_0I_8G() {
        // t = 5G + 3G
        let mut t = ECRECOVER_CONTEXT.pre_g[2].to_affine().to_jacobian();
        t.add_ge_in_place(ECRECOVER_CONTEXT.pre_g[1].to_affine(), None);

        let t = t.to_affine();

        // 0*infinity + 8*G = 8*G
        let res = ecmult(
            &Jacobian::INFINITY,
            &Scalar::ZERO,
            &Scalar::from_u128(8),
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert_eq!(res, t);
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_1G_0G() {
        let res = ecmult(
            &Affine::GENERATOR.to_jacobian(),
            &Scalar::ONE,
            &Scalar::ZERO,
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert_eq!(res, Affine::GENERATOR);
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_1G_1G() {
        let res = ecmult(
            &Affine::GENERATOR.to_jacobian(),
            &Scalar::ONE,
            &Scalar::ONE,
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        let mut expected = Affine::GENERATOR.to_jacobian();
        expected.double_in_place(None);

        assert_eq!(res, expected.to_affine())
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_1G_2G() {
        let res = ecmult(
            &Affine::GENERATOR.to_jacobian(),
            &Scalar::ONE,
            &Scalar::from_u128(2),
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        assert_eq!(res, ECRECOVER_CONTEXT.pre_g[1].to_affine());
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn ecmult_2G_1G() {
        let mut res = ecmult(
            &Affine::GENERATOR.to_jacobian(),
            &Scalar::from_u128(2),
            &Scalar::ONE,
            &ECRECOVER_CONTEXT,
        )
        .to_affine();

        res.normalize_in_place();

        assert_eq!(res, ECRECOVER_CONTEXT.pre_g[1].to_affine());
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn compare_ecmult() {
        proptest!(|(k: Scalar)| {
            let res1 = ecmult(
                &Jacobian::INFINITY,
                &Scalar::ZERO,
                &k,
                &ECRECOVER_CONTEXT
            ).to_affine();

            let res2 = ecmult(
                &Affine::GENERATOR.to_jacobian(),
                &k,
                &Scalar::ZERO,
                &ECRECOVER_CONTEXT
            ).to_affine();

            prop_assert_eq!(res1, res2);
        })
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn test_generator_multipules() {
        for (k, x, y) in MUL_TEST_VECTORS {
            let k = Scalar::from_repr((*k).into());

            let computed_ctx =
                ecmult(&Jacobian::INFINITY, &Scalar::ZERO, &k, &ECRECOVER_CONTEXT).to_affine();

            let computed = ecmult(
                &Affine::GENERATOR.to_jacobian(),
                &k,
                &Scalar::ZERO,
                &ECRECOVER_CONTEXT,
            )
            .to_affine();

            let expected = Affine {
                x: FieldElement::from_bytes_unchecked(x),
                y: FieldElement::from_bytes_unchecked(y),
                infinity: false,
            };

            assert_eq!(computed_ctx, computed);
            assert_eq!(computed_ctx, expected);
        }
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn test_regressions() {
        // recover point at infinity
        assert_eq!(
            recover_from_digest(
                [
                    107, 141, 44, 129, 177, 27, 45, 105, 149, 40, 221, 228, 136, 219, 223, 47, 148,
                    41, 61, 13, 51, 195, 46, 52, 127, 37, 95, 164, 166, 193, 240, 169
                ],
                [
                    121, 190, 102, 126, 249, 220, 187, 172, 85, 160, 98, 149, 206, 135, 11, 7, 2,
                    155, 252, 219, 45, 206, 40, 217, 89, 242, 129, 91, 22, 248, 23, 152
                ],
                [
                    107, 141, 44, 129, 177, 27, 45, 105, 149, 40, 221, 228, 136, 219, 223, 47, 148,
                    41, 61, 13, 51, 195, 46, 52, 127, 37, 95, 164, 166, 193, 240, 169
                ],
                0
            )
            .unwrap_err(),
            super::Secp256k1Err::RecoveredInfinity
        );

        // geth
        assert_eq!(
            recover_from_digest(
                [
                    56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112,
                    84, 246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62
                ],
                [
                    56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112,
                    84, 246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62
                ],
                [
                    120, 157, 29, 212, 35, 210, 95, 7, 114, 210, 116, 141, 96, 247, 228, 184, 27,
                    177, 77, 8, 110, 186, 142, 142, 142, 251, 109, 207, 248, 164, 174, 2
                ],
                0
            ),
            Ok(Affine {
                x: FieldElement::from_bytes_unchecked(&[
                    134, 18, 84, 164, 207, 141, 253, 45, 96, 226, 163, 62, 49, 67, 234, 198, 40,
                    88, 134, 240, 174, 217, 23, 17, 171, 126, 44, 1, 63, 38, 92, 85
                ]),
                y: FieldElement::from_bytes_unchecked(&[
                    253, 69, 102, 103, 243, 176, 134, 87, 121, 95, 230, 117, 75, 111, 188, 17, 24,
                    103, 20, 196, 228, 244, 141, 91, 133, 104, 0, 227, 232, 28, 48, 200
                ]),
                infinity: false
            })
        )
    }

    #[cfg(feature = "secp256k1-static-context")]
    fn recover_from_digest(
        digest: [u8; 32],
        r: [u8; 32],
        s: [u8; 32],
        rec_id: u8,
    ) -> Result<Affine, super::Secp256k1Err> {
        use k256::ecdsa::{RecoveryId, Signature};
        use {
            k256::elliptic_curve::ops::Reduce,
            k256::{ecdsa::hazmat::bits2field, Scalar},
        };

        let signature = Signature::from_scalars(r, s).unwrap();
        let recovery_id = RecoveryId::try_from(rec_id).unwrap();

        let message = <Scalar as Reduce<k256::U256>>::reduce_bytes(
            &bits2field::<k256::Secp256k1>(&digest).unwrap(),
        );

        super::recover(&message, &signature, &recovery_id)
    }

    #[cfg(feature = "secp256k1-static-context")]
    #[test]
    fn recover_with_default_hooks_matches_recover() {
        use crate::secp256k1::hooks::DefaultSecp256k1Hooks;
        use k256::ecdsa::{RecoveryId, Signature};
        use {
            k256::elliptic_curve::ops::Reduce,
            k256::{ecdsa::hazmat::bits2field, Scalar},
        };

        let digest = [
            56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112, 84,
            246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62,
        ];
        let r = digest;
        let s = [
            120, 157, 29, 212, 35, 210, 95, 7, 114, 210, 116, 141, 96, 247, 228, 184, 27, 177, 77,
            8, 110, 186, 142, 142, 142, 251, 109, 207, 248, 164, 174, 2,
        ];

        let signature = Signature::from_scalars(r, s).unwrap();
        let recovery_id = RecoveryId::try_from(0u8).unwrap();
        let message = <Scalar as Reduce<k256::U256>>::reduce_bytes(
            &bits2field::<k256::Secp256k1>(&digest).unwrap(),
        );

        let without_hooks = super::recover(&message, &signature, &recovery_id).unwrap();
        let with_hooks = super::recover_with_hooks(
            &message,
            &signature,
            &recovery_id,
            &mut DefaultSecp256k1Hooks,
        )
        .unwrap();

        assert_eq!(without_hooks, with_hooks);
    }
}
