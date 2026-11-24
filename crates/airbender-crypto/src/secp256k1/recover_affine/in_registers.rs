//! The affine group operations for the field of four 64-bit limbs (`cfg(secp256k1_field_4x64)`):
//! the formulas of the parent module, written on values. The coordinates are read once, the
//! intermediate elements are locals that never have their address taken, and the result is
//! stored once, so that the limbs stay in registers; only the fraction of a division and its
//! quotient are in memory, where the oracle reads and writes them. (The delegated field wants
//! the opposite: operands in memory, where the delegation works in place.)

use core::mem::MaybeUninit;

use crate::secp256k1::{
    context::{ECMultContext, ECMULT_TABLE_SIZE_A, WINDOW_A, WINDOW_G},
    field::FieldElement,
    hooks::Secp256k1Hooks,
    points::Affine,
    recover::{table_index, WnafDigit},
    recover_witnessed::WITNESS_COEFFICIENT_BITS,
    scalars::Scalar,
};

#[inline(always)]
fn sum(a: &FieldElement, b: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.add_in_place(b);
    r
}

#[inline(always)]
fn difference(a: &FieldElement, b: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.sub_in_place(b);
    r
}

/// `a² - b - c`: the `x` of a sum from its slope
#[inline(always)]
fn square_less(a: &FieldElement, b: &FieldElement, c: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.square_in_place();
    r.sub_two_in_place(b, c);
    r
}

#[inline(always)]
fn product(a: &FieldElement, b: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.mul_in_place(b);
    r
}

#[inline(always)]
fn square(a: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.square_in_place();
    r
}

#[inline(always)]
fn double_of(a: &FieldElement) -> FieldElement {
    let mut r = *a;
    r.double_in_place();
    r
}

/// `numerator / denominator` for a denominator that is not zero
#[inline(always)]
fn quotient<H: Secp256k1Hooks>(
    numerator: FieldElement,
    denominator: FieldElement,
    hooks: &mut H,
) -> FieldElement {
    let mut fraction = [numerator, denominator];
    let mut quotient = MaybeUninit::uninit();
    *hooks.fe_divide(&mut fraction, &mut quotient)
}

/// `r = p`, or `-p` if `negate`, for the point `p = (x, y)`
#[inline(always)]
pub(super) fn set(r: &mut Affine, x: &FieldElement, y: &FieldElement, negate: bool) {
    r.x = *x;
    r.y = *y;
    if negate {
        r.y.negate_in_place(1);
    }
    r.infinity = false;
}

/// The slope of `(x1, y1) + p` for the point `p = (x2, y2)`, or `(x2, -y2)` if `negate`, as a
/// numerator and a denominator. The denominator is zero if `p` is the first point or its
/// negation, and it is the first point if the numerator is zero too.
#[inline(always)]
fn slope_of_sum(
    x1: &FieldElement,
    y1: &FieldElement,
    x2: &FieldElement,
    y2: &FieldElement,
    negate: bool,
) -> (FieldElement, FieldElement) {
    if negate {
        // (-y2 - y1) / (x2 - x1) = (y2 + y1) / (x1 - x2)
        (sum(y2, y1), difference(x1, x2))
    } else {
        (difference(y2, y1), difference(x2, x1))
    }
}

/// `r = 2 r`
pub(super) fn double<H: Secp256k1Hooks>(r: &mut Affine, hooks: &mut H) {
    if r.infinity {
        return;
    }
    let (x, y) = (r.x, r.y);
    // the slope is 3 x² / 2 y
    let mut numerator = square(&x);
    numerator.triple_in_place();
    let denominator = double_of(&y);
    if denominator.is_zero() {
        // a point of order 2 (the curve has none, its order is odd)
        r.infinity = true;
        return;
    }
    let slope = quotient(numerator, denominator, hooks);
    // x3 = slope² - 2 x, y3 = slope (x - x3) - y
    let x3 = square_less(&slope, &x, &x);
    let y3 = difference(&product(&difference(&x, &x3), &slope), &y);
    r.x = x3;
    r.y = y3;
}

/// `r = r + p`, or `r - p` if `negate`, for the point `p = (x, y)`
pub(super) fn add<H: Secp256k1Hooks>(
    r: &mut Affine,
    x: &FieldElement,
    y: &FieldElement,
    negate: bool,
    hooks: &mut H,
) {
    if r.infinity {
        set(r, x, y, negate);
        return;
    }
    let (x1, y1, x2) = (r.x, r.y, *x);
    let (numerator, denominator) = slope_of_sum(&x1, &y1, &x2, y, negate);
    if denominator.is_zero() {
        if numerator.is_zero() {
            double(r, hooks);
        } else {
            r.infinity = true;
        }
        return;
    }
    let slope = quotient(numerator, denominator, hooks);
    // x3 = slope² - x1 - x2, y3 = slope (x1 - x3) - y1
    let x3 = square_less(&slope, &x1, &x2);
    let y3 = difference(&product(&difference(&x1, &x3), &slope), &y1);
    r.x = x3;
    r.y = y3;
}

/// `r = 2 r + p`, or `2 r - p` if `negate`, for the point `p = (x, y)`: `(r + p) + r` without the
/// `y` coordinate of `r + p` (see the documentation of the parent module). The cases it does
/// not cover (`p` is `r` or `-r`, or `r + p` is `r` or `-r`) are a doubling and an addition.
pub(super) fn double_and_add<H: Secp256k1Hooks>(
    r: &mut Affine,
    x: &FieldElement,
    y: &FieldElement,
    negate: bool,
    hooks: &mut H,
) {
    if r.infinity {
        set(r, x, y, negate);
        return;
    }
    let (x1, y1, x2) = (r.x, r.y, *x);
    let (numerator, denominator) = slope_of_sum(&x1, &y1, &x2, y, negate);
    if denominator.is_zero() {
        double(r, hooks);
        add(r, x, y, negate, hooks);
        return;
    }
    let slope = quotient(numerator, denominator, hooks);

    // x_t = slope² - x1 - x2, the coordinate of r + p
    let x_t = square_less(&slope, &x1, &x2);

    // the slope of (r + p) + r is -(slope + 2 y1 / (x_t - x1))
    let denominator = difference(&x_t, &x1);
    if denominator.is_zero() {
        double(r, hooks);
        add(r, x, y, negate, hooks);
        return;
    }
    let negated_slope = sum(&quotient(double_of(&y1), denominator, hooks), &slope);

    // x3 = slope_2² - x1 - x_t, y3 = slope_2 (x1 - x3) - y1 = -slope_2 (x3 - x1) - y1
    let x3 = square_less(&negated_slope, &x1, &x_t);
    let y3 = difference(&product(&difference(&x3, &x1), &negated_slope), &y1);
    r.x = x3;
    r.y = y3;
}

/// The point `(x, y)`
#[inline(always)]
fn point(x: &FieldElement, y: &FieldElement) -> Affine {
    Affine {
        x: *x,
        y: *y,
        infinity: false,
    }
}

/// The odd multiples `p, 3 p, ..., (2^WINDOW_A - 1) p` of a point that is not the point at
/// infinity, and the `x` of their images under the endomorphism (`β x`). Each multiple is
/// computed in its place in the table, from the one before it.
#[inline(never)]
fn odd_multiples<H: Secp256k1Hooks>(
    p: &Affine,
    multiples: &mut [MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A],
    images: &mut [MaybeUninit<FieldElement>; ECMULT_TABLE_SIZE_A],
    hooks: &mut H,
) {
    let mut doubled = point(&p.x, &p.y);
    double(&mut doubled, hooks);
    let mut entries = multiples.iter_mut().zip(images.iter_mut());
    let (first, image) = entries.next().expect("the table is not empty");
    let mut previous: &Affine = first.write(point(&p.x, &p.y));
    image.write(product(&FieldElement::BETA, &previous.x));
    for (multiple, image) in entries {
        let multiple = multiple.write(Affine {
            x: previous.x,
            y: previous.y,
            infinity: previous.infinity,
        });
        if !doubled.infinity {
            add(multiple, &doubled.x, &doubled.y, false, hooks);
        }
        image.write(product(&FieldElement::BETA, &multiple.x));
        previous = multiple;
    }
}

/// The streams of a witnessed multiplication: the coefficients of `p`, `φ(p)`, `q`, `φ(q)`, then
/// the four 64-bit chunks of the scalar of the generator
const STREAMS: usize = 8;
/// The bits of the wNAF digits of a stream: one more than a witnessed coefficient has (a chunk
/// has 64, and so 65 digits)
const DIGITS: usize = WITNESS_COEFFICIENT_BITS + 1;

/// The wNAF digits of the streams by bit, and for each bit the set of the streams that have
/// one: the multiplication looks at a bit once, and at its digits only if there are any. A
/// digit is written only where its stream is in the set, and read only there.
struct Schedule {
    digits: [[MaybeUninit<WnafDigit>; STREAMS]; DIGITS],
    streams: [u8; DIGITS],
    /// One more than the highest bit with a digit
    bits: usize,
}

impl Schedule {
    /// Records the wNAF digits of `value` (below `2^(DIGITS - 1)`) for the window `WINDOW`:
    /// `value = Σ digit_i 2^i` with odd digits below `2^(WINDOW - 1)` in absolute value, any
    /// two at least `WINDOW` bits apart. The same digits as `recover::wnaf` gives.
    ///
    /// The value is consumed from its lowest bit, in two limbs; almost all of the steps find
    /// the upper one zero (a chunk of the generator's scalar has one limb, a witnessed
    /// coefficient a few bits more), and those shift one limb only.
    #[inline(never)]
    fn record<const WINDOW: usize>(&mut self, stream: usize, value: u128) {
        debug_assert!(value >> (DIGITS - 1) == 0);
        let (mut low, mut high) = (value as u64, (value >> 64) as u64);
        let mut bit = 0;
        let mut bits = self.bits;
        loop {
            if low == 0 {
                if high == 0 {
                    break;
                }
                (low, high) = (high, 0);
                bit += 64;
            }
            // the lowest limb is not zero: up to its lowest set bit
            if high == 0 {
                while low & 1 == 0 {
                    low >>= 1;
                    bit += 1;
                }
            } else {
                while low & 1 == 0 {
                    low = low >> 1 | high << 63;
                    high >>= 1;
                    bit += 1;
                }
            }
            // The digit is the odd residue of the lowest `WINDOW` bits nearest to zero. Without
            // it the value is a multiple of `2^WINDOW`: shifted, what is above those bits, and
            // one more if the digit is negative (the bits were rounded up).
            let residue = (low & ((1 << WINDOW) - 1)) as i32;
            let negative = residue >> (WINDOW - 1) != 0;
            let digit = if negative {
                residue - (1 << WINDOW)
            } else {
                residue
            };
            low = low >> WINDOW | high << (64 - WINDOW);
            high >>= WINDOW;
            if negative {
                let (sum, carry) = low.overflowing_add(1);
                low = sum;
                high += carry as u64;
            }
            self.digits[bit][stream].write(digit as WnafDigit);
            self.streams[bit] |= 1 << stream;
            bits = bits.max(bit + 1);
            bit += WINDOW;
        }
        self.bits = bits;
    }
}

/// `a p + b φ(p) + c q + d φ(q) + k g` for the points `p` and `q` (not the point at infinity),
/// the signed coefficients `[a, b, c, d]` of at most `WITNESS_COEFFICIENT_BITS` bits in
/// absolute value and any scalar `k` of the generator, which is split into four 64-bit
/// chunks over the four generator tables. The doublings are those of the longest coefficient:
/// about 64 for the witnessed recovery instead of the 128 of `ecmult_affine`.
pub(in crate::secp256k1) fn ecmult_affine_4d<H: Secp256k1Hooks>(
    p: &Affine,
    q: &Affine,
    coefficients: &[i128; 4],
    k: &Scalar,
    context: &ECMultContext,
    hooks: &mut H,
) -> Affine {
    let mut multiples: [[MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A]; 2] =
        [const { [const { MaybeUninit::uninit() }; ECMULT_TABLE_SIZE_A] }; 2];
    let mut images: [[MaybeUninit<FieldElement>; ECMULT_TABLE_SIZE_A]; 2] =
        [const { [const { MaybeUninit::uninit() }; ECMULT_TABLE_SIZE_A] }; 2];
    for (base, (multiples, images)) in [p, q]
        .into_iter()
        .zip(multiples.iter_mut().zip(&mut images))
    {
        odd_multiples(base, multiples, images, hooks);
    }
    // SAFETY: `odd_multiples` wrote every entry of both tables, and `MaybeUninit<T>` has the
    // layout of `T`
    let (multiples, images) = unsafe {
        (
            &*(&multiples as *const [[MaybeUninit<Affine>; ECMULT_TABLE_SIZE_A]; 2])
                .cast::<[[Affine; ECMULT_TABLE_SIZE_A]; 2]>(),
            &*(&images as *const [[MaybeUninit<FieldElement>; ECMULT_TABLE_SIZE_A]; 2])
                .cast::<[[FieldElement; ECMULT_TABLE_SIZE_A]; 2]>(),
        )
    };

    // the digits of the magnitudes; a negative coefficient negates its points
    let mut schedule = Schedule {
        digits: [const { [const { MaybeUninit::uninit() }; STREAMS] }; DIGITS],
        streams: [0; DIGITS],
        bits: 0,
    };
    for (stream, coefficient) in coefficients.iter().enumerate() {
        schedule.record::<WINDOW_A>(stream, coefficient.unsigned_abs());
    }
    let negative = coefficients.map(|coefficient| coefficient < 0);
    // the limbs of this representation of a scalar are those of its canonical integer
    for (chunk, limb) in k.limbs().into_iter().enumerate() {
        schedule.record::<WINDOW_G>(4 + chunk, u128::from(limb));
    }
    let g_tables = [
        &context.pre_g,
        &context.pre_g_64,
        &context.pre_g_128,
        &context.pre_g_192,
    ];

    let mut r = Affine {
        x: FieldElement::ZERO,
        y: FieldElement::ZERO,
        infinity: true,
    };
    for bit in (0..schedule.bits).rev() {
        // the first addition after a doubling goes together with it
        let mut doubled = false;
        let streams = schedule.streams[bit];
        if streams != 0 {
            let digits = &schedule.digits[bit];
            for stream in 0..STREAMS {
                if streams & (1 << stream) == 0 {
                    continue;
                }
                // SAFETY: the stream is in the set of the bit, so `record` wrote its digit
                let digit = unsafe { digits[stream].assume_init() };
                let (x, y, negate) = if stream < 4 {
                    let (index, negate) = table_index(digit, WINDOW_A);
                    let multiple = &multiples[stream / 2][index];
                    if multiple.infinity {
                        continue;
                    }
                    // the odd streams are those of the images, `(β x, y)`
                    let x = if stream % 2 == 0 {
                        &multiple.x
                    } else {
                        &images[stream / 2][index]
                    };
                    (x, &multiple.y, negate != negative[stream])
                } else {
                    // a stored multiple of the generator is its coordinates, in place
                    let (index, negate) = table_index(digit, WINDOW_G);
                    let (x, y) = g_tables[stream - 4][index].coordinates();
                    (x, y, negate)
                };
                if doubled {
                    add(&mut r, x, y, negate, hooks);
                } else {
                    double_and_add(&mut r, x, y, negate, hooks);
                    doubled = true;
                }
            }
        }
        if !doubled {
            double(&mut r, hooks);
        }
    }

    // by its fields: as a whole it is one word too long to be copied without a call
    Affine {
        x: r.x,
        y: r.y,
        infinity: r.infinity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secp256k1::recover::wnaf;
    use proptest::{prop_assert_eq, proptest};

    /// The digits of a stream by bit, as `recover::wnaf` gives them for the scalar of the value
    fn reference(value: u128, window: usize) -> ([WnafDigit; DIGITS], usize) {
        let mut digits = [0; DIGITS];
        let bits = if value == 0 {
            0
        } else {
            wnaf(&mut digits, &Scalar::wnaf_form_u128(value), window) as usize
        };
        (digits, bits)
    }

    fn new_schedule() -> Schedule {
        Schedule {
            digits: [const { [const { MaybeUninit::uninit() }; STREAMS] }; DIGITS],
            streams: [0; DIGITS],
            bits: 0,
        }
    }

    /// Records the value with the window of the stream, which it returns
    fn record(schedule: &mut Schedule, stream: usize, value: u128) -> usize {
        if stream < 4 {
            schedule.record::<WINDOW_A>(stream, value);
            WINDOW_A
        } else {
            schedule.record::<WINDOW_G>(stream, value);
            WINDOW_G
        }
    }

    /// The digits of a stream by bit: zero where the stream is not in the set
    fn digits_of(schedule: &Schedule, stream: usize) -> [WnafDigit; DIGITS] {
        core::array::from_fn(|bit| {
            if schedule.streams[bit] & (1 << stream) == 0 {
                0
            } else {
                // SAFETY: the stream is in the set of the bit, so its digit was written
                let digit = unsafe { schedule.digits[bit][stream].assume_init() };
                assert_ne!(digit, 0);
                digit
            }
        })
    }

    #[test]
    fn the_schedule_has_the_digits_of_the_wnaf() {
        let check = |values: [u128; STREAMS]| {
            let mut schedule = new_schedule();
            let mut bits = 0;
            for (stream, value) in values.into_iter().enumerate() {
                let window = record(&mut schedule, stream, value);
                let (digits, stream_bits) = reference(value, window);
                bits = bits.max(stream_bits);
                assert_eq!(digits_of(&schedule, stream), digits, "{value:x}");
            }
            assert_eq!(schedule.bits, bits);
        };
        let top = (1u128 << WITNESS_COEFFICIENT_BITS) - 1;
        check([0; STREAMS]);
        check([1, 2, 3, 15, 16, 17, 511, 512]);
        check([
            top,
            top - 1,
            top >> 1,
            1 << (WITNESS_COEFFICIENT_BITS - 1),
            top,
            1 << 79,
            513,
            1023,
        ]);
        // a rounding that carries out of the lower limb, and zeros across the limbs
        check([u64::MAX as u128; STREAMS]);
        check([
            u64::MAX as u128 - 1,
            1 << 63,
            (1 << 63) + 1,
            (1 << 64) - 16,
            (1 << 64) + 16,
            (1 << 65) - 1,
            1 << 64,
            1 << 65,
        ]);
        check([
            (0xffff << 64) | 1,
            0xffff << 64,
            (0xffff << 64) | (1 << 63),
            (1 << 80) - (1 << 64),
            0x8000_0000_0000_0000_0000,
            0xffff_0000_0000_0000_0001,
            0x1_ffff_ffff_ffff_fff0,
            0x7_ffff_ffff_ffff_ffff,
        ]);
        proptest!(|(values: [u128; STREAMS], shifts: [u8; STREAMS])| {
            // values of every length up to the bound
            let values: [u128; STREAMS] = core::array::from_fn(|i| {
                (values[i] & top) >> (shifts[i] as usize % WITNESS_COEFFICIENT_BITS)
            });
            let mut schedule = new_schedule();
            for (stream, value) in values.into_iter().enumerate() {
                let window = record(&mut schedule, stream, value);
                prop_assert_eq!(digits_of(&schedule, stream), reference(value, window).0);
            }
        });
    }
}
