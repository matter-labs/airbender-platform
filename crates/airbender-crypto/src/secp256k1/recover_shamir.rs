//! Alternative structure of the `na*A + ng*G` multi-scalar multiplication, the one used by
//! evmone's `ecrecover` on Airbender (`ecrecover_msm_glv`). It exists to compare the two
//! structures on the same field and point arithmetic:
//! - both scalars are split by the GLV endomorphism into ~128 bit halves
//! - `ng*G` uses fixed (unsigned) 8 bit windows over precomputed affine tables of multiples
//!   of `G` and `phi(G)`: 16 additions per half, no odd-multiples tables or wNAF
//! - `na*A` uses the joint NAF of the two halves (Straus-Shamir trick) with the table
//!   `{A, phi(A), A + phi(A), A - phi(A)}`. There is nothing to bring to the same `z`: evmone
//!   keeps the two combined entries in Jacobian coordinates and adds them by the full addition.
//!   Here the inversion is a hook (an oracle hint for the guest), and both entries need
//!   the same one, so they are affine too.
//!
//! The default structure (`recover::ecmult`) is wNAF-5 over 8 odd multiples of `A` and `phi(A)`
//! with the same effective `z`, and wNAF-10 over tables of `G` and `2^128*G`.

use super::field::{FieldElement, FieldElementConst};
use super::hooks::Secp256k1Hooks;
use super::points::{Affine, AffineConst, AffineStorage, Jacobian, JacobianConst};
use super::scalars::Scalar;

const WINDOW: usize = 8;
const TABLE_SIZE: usize = (1 << WINDOW) - 1;
/// Halves are below 2^129
const NUM_WINDOWS: usize = 17;
const HALF_WORDS: usize = 5;

struct FixedWindowContext {
    /// `(i + 1) * G`
    g: [AffineStorage; TABLE_SIZE],
    /// `(i + 1) * phi(G)`
    phi_g: [AffineStorage; TABLE_SIZE],
}

static CONTEXT: FixedWindowContext = FixedWindowContext::const_new();

impl FixedWindowContext {
    const fn const_new() -> Self {
        use const_for::const_for;

        // 0x7ae96a2b657c07106e64479eac3434e99cf0497512f58995c1396c28719501ee
        const BETA: FieldElementConst = FieldElementConst::from_bytes_unchecked(&[
            0x7a, 0xe9, 0x6a, 0x2b, 0x65, 0x7c, 0x07, 0x10, 0x6e, 0x64, 0x47, 0x9e, 0xac, 0x34,
            0x34, 0xe9, 0x9c, 0xf0, 0x49, 0x75, 0x12, 0xf5, 0x89, 0x95, 0xc1, 0x39, 0x6c, 0x28,
            0x71, 0x95, 0x01, 0xee,
        ]);

        let mut context = Self {
            g: [AffineStorage::DEFAULT; TABLE_SIZE],
            phi_g: [AffineStorage::DEFAULT; TABLE_SIZE],
        };

        let mut acc = JacobianConst::GENERATOR;
        const_for!(i in 0..TABLE_SIZE => {
            let multiple = acc.to_affine_const();
            context.g[i] = multiple.to_storage();
            context.phi_g[i] = AffineConst {
                x: multiple.x.mul(&BETA),
                y: multiple.y,
                infinity: false,
            }
            .to_storage();

            acc = acc.add_ge(&AffineConst::GENERATOR, None);
        });

        context
    }
}

/// Absolute value of the half produced by `Scalar::decompose` (it is in the integer form there,
/// and a negative half is `ORDER - |half|`), and its sign
fn split_half(mut half: Scalar) -> ([u32; HALF_WORDS], bool) {
    let negative = half.bits(255, 1) != 0;
    if negative {
        half.negate_in_place();
    }

    let mut words = [0u32; HALF_WORDS];
    for (i, word) in words.iter_mut().enumerate() {
        *word = half.bits(32 * i, 32);
    }
    debug_assert!((HALF_WORDS..8).all(|i| half.bits(32 * i, 32) == 0));

    (words, negative)
}

/// Non-adjacent form of a number below 2^129: digit `i` is `positive[i] - negative[i]`
struct Naf {
    positive: [u32; HALF_WORDS],
    negative: [u32; HALF_WORDS],
}

impl Naf {
    const ZERO: Self = Self {
        positive: [0; HALF_WORDS],
        negative: [0; HALF_WORDS],
    };

    /// Digit `i` of NAF of `x` is `bit(3x, i + 1) - bit(x, i + 1)`
    fn new(x: &[u32; HALF_WORDS]) -> Self {
        // 3x < 2^131
        let mut x3 = [0u32; HALF_WORDS];
        let mut carry = 0u64;
        for i in 0..HALF_WORDS {
            let t = 3 * (x[i] as u64) + carry;
            x3[i] = t as u32;
            carry = t >> 32;
        }
        debug_assert!(carry == 0);

        let mut ret = Self::ZERO;
        for i in 0..HALF_WORDS {
            let next = |words: &[u32; HALF_WORDS]| {
                let high = if i + 1 < HALF_WORDS {
                    words[i + 1] << 31
                } else {
                    0
                };
                (words[i] >> 1) | high
            };
            let (x3, x) = (next(&x3), next(x));
            ret.positive[i] = x3 & !x;
            ret.negative[i] = !x3 & x;
        }

        ret
    }

    #[inline(always)]
    fn digit(&self, i: usize) -> i32 {
        let (word, shift) = (i / 32, i % 32);
        ((self.positive[word] >> shift) & 1) as i32 - ((self.negative[word] >> shift) & 1) as i32
    }

    /// Number of the digits
    fn len(&self) -> usize {
        for i in (0..HALF_WORDS).rev() {
            let word = self.positive[i] | self.negative[i];
            if word != 0 {
                return 32 * i + 32 - word.leading_zeros() as usize;
            }
        }
        0
    }
}

/// `p_a + p_b` and `p_a - p_b` by the chord rule: the slopes have the same denominator
fn affine_sum_and_difference<H: Secp256k1Hooks>(
    p_a: &Affine,
    p_b: &Affine,
    hooks: &mut H,
) -> Option<(Affine, Affine)> {
    let mut dx_inv = p_b.x;
    dx_inv.sub_in_place(&p_a.x);
    if dx_inv.normalizes_to_zero() {
        return None;
    }
    hooks.fe_invert_and_assign(&mut dx_inv);

    let chord = |negate_b: bool| {
        // slope = (+-y_b - y_a) / (x_b - x_a)
        let mut slope = p_b.y;
        if negate_b {
            slope.negate_in_place(1);
        }
        slope.sub_in_place(&p_a.y);
        slope *= &dx_inv;

        // x = slope^2 - x_a - x_b
        let mut x = slope;
        x.square_in_place();
        x.sub_in_place(&p_a.x);
        x.sub_in_place(&p_b.x);

        // y = slope * (x_a - x) - y_a
        let mut y = p_a.x;
        y.sub_in_place(&x);
        y *= &slope;
        y.sub_in_place(&p_a.y);

        Affine {
            x,
            y,
            infinity: false,
        }
    };

    Some((chord(false), chord(true)))
}

/// Compute na*a+ng*g where g is the generator.
pub(super) fn ecmult<H: Secp256k1Hooks>(
    a: &Jacobian,
    na: &Scalar,
    ng: &Scalar,
    hooks: &mut H,
) -> Jacobian {
    // `na*A` part: the table and the joint NAF
    let mut naf_a = Naf::ZERO;
    let mut naf_b = Naf::ZERO;
    let mut naf_len = 0;

    let mut p_a = Affine::DEFAULT;
    let mut p_b = Affine::DEFAULT;
    let mut p_sum = Affine::DEFAULT;
    let mut p_diff = Affine::DEFAULT;

    if !na.is_zero() && !a.is_infinity() {
        let (na_1, na_lam) = na.decompose();
        let (k_a, a_negative) = split_half(na_1);
        let (k_b, b_negative) = split_half(na_lam);

        // that is the case for the recovery, no inversion is needed then
        let mut z_minus_one = a.z;
        z_minus_one.sub_in_place(&FieldElement::ONE);
        p_a = if z_minus_one.normalizes_to_zero() {
            Affine {
                x: a.x,
                y: a.y,
                infinity: false,
            }
        } else {
            a.to_affine_with_hooks(hooks)
        };

        p_b = p_a;
        p_b.x *= &FieldElement::BETA;
        if a_negative {
            p_a.y.negate_in_place(1);
        }
        if b_negative {
            p_b.y.negate_in_place(1);
        }

        (p_sum, p_diff) = match affine_sum_and_difference(&p_a, &p_b, hooks) {
            Some(points) => points,
            // `x == beta * x`, there is no such point on the curve
            None => {
                let mut sum = p_a.to_jacobian();
                sum.add_affine_in_place(&p_b.x, &p_b.y, false, None);
                let mut diff = p_a.to_jacobian();
                diff.add_affine_in_place(&p_b.x, &p_b.y, true, None);
                (
                    sum.to_affine_with_hooks(hooks),
                    diff.to_affine_with_hooks(hooks),
                )
            }
        };

        naf_a = Naf::new(&k_a);
        naf_b = Naf::new(&k_b);
        naf_len = core::cmp::max(naf_a.len(), naf_b.len());
    }

    // `ng*G` part: windows of the halves
    let mut g_windows = [(0u8, 0u8); NUM_WINDOWS];
    let mut g_negative = false;
    let mut phi_g_negative = false;
    if !ng.is_zero() {
        let (ng_1, ng_lam) = ng.decompose();
        let (k_a, a_negative) = split_half(ng_1);
        let (k_b, b_negative) = split_half(ng_lam);
        g_negative = a_negative;
        phi_g_negative = b_negative;

        for (i, window) in g_windows.iter_mut().enumerate() {
            let shift = (WINDOW * i) % 32;
            let word = (WINDOW * i) / 32;
            *window = ((k_a[word] >> shift) as u8, (k_b[word] >> shift) as u8);
        }
    }

    let mut r = Jacobian::INFINITY;
    // there is nothing to double until the first addition
    let mut started = false;

    for i in (0..NUM_WINDOWS * WINDOW).rev() {
        if started {
            r.double_in_place(None);
        }

        if i < naf_len {
            match (naf_a.digit(i), naf_b.digit(i)) {
                (0, 0) => {}
                (d_a, 0) => {
                    r.add_affine_in_place(&p_a.x, &p_a.y, d_a < 0, None);
                    started = true;
                }
                (0, d_b) => {
                    r.add_affine_in_place(&p_b.x, &p_b.y, d_b < 0, None);
                    started = true;
                }
                (d_a, d_b) => {
                    // d_a * (P_a + d_a * d_b * P_b)
                    let p = if d_a == d_b { &p_sum } else { &p_diff };
                    if !p.is_infinity() {
                        r.add_affine_in_place(&p.x, &p.y, d_a < 0, None);
                        started = true;
                    }
                }
            }
        }

        if i % WINDOW == 0 {
            let (window_g, window_phi_g) = g_windows[i / WINDOW];
            if window_g != 0 {
                let p = CONTEXT.g[window_g as usize - 1].to_affine();
                r.add_affine_in_place(&p.x, &p.y, g_negative, None);
                started = true;
            }
            if window_phi_g != 0 {
                let p = CONTEXT.phi_g[window_phi_g as usize - 1].to_affine();
                r.add_affine_in_place(&p.x, &p.y, phi_g_negative, None);
                started = true;
            }
        }
    }

    r
}
