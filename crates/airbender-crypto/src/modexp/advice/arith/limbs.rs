//! The arithmetic of the multi-digit path on plain `u64` limbs: a digit is one limb, and the
//! operations are carry chains over limb slices.
//!
//! A limb product is `a * b + c + d` in two limbs: on RV64 a `mul`, a `mulhu`, and two
//! additions with their carries (`sltu`), which with the loads and the store is 11 instructions,
//! and 11.75 in the rows, unrolled by four (7.75 in the first row of a product, which adds
//! nothing but the carry). Products are schoolbook rows below [`KARATSUBA_MUL_MIN`] limbs and
//! Karatsuba above (three products of half the length, and about 50 instructions of additions
//! per limb of an operand), squares likewise from [`KARATSUBA_SQUARE_MIN`]; a schoolbook square
//! takes the products `a_i a_j` (`i < j`) once. The thresholds are measured on the RV64 guest.
//!
//! Nothing here is constant-time, and nothing divides.

use core::cmp::Ordering;
use core::mem::MaybeUninit;

/// The digit of the arithmetic: one limb
pub(crate) type Digit = u64;

/// Bytes of a digit
pub(crate) const DIGIT_BYTES: usize = 8;

/// The digit 1
pub(crate) const DIGIT_ONE: Digit = 1;

/// The most limbs of an operand (EIP-7823)
const MAX_LIMBS: usize = crate::modexp::MAX_OPERAND_BYTES / DIGIT_BYTES;

/// Operands of at least this many limbs, of one length, are multiplied by Karatsuba's method
const KARATSUBA_MUL_MIN: usize = 24;

/// Operands of at least this many limbs are squared by Karatsuba's method
const KARATSUBA_SQUARE_MIN: usize = 48;

/// Limbs of the working space of a Karatsuba product of `n` limbs: `4 ceil(n / 2)` at each
/// level, down to operands of 4 limbs or fewer (below any threshold)
const fn karatsuba_space(n: usize) -> usize {
    if n <= 4 {
        0
    } else {
        let half = n.div_ceil(2);
        4 * half + karatsuba_space(half)
    }
}

/// Limbs of [`Scratch`]: a product of two operands, and the working space of its Karatsuba
/// levels
const SCRATCH_LIMBS: usize = 2 * MAX_LIMBS + karatsuba_space(MAX_LIMBS);

/// The working space of the products, on the stack. Nothing in it is read before it is written.
pub(crate) struct Scratch {
    limbs: [MaybeUninit<u64>; SCRATCH_LIMBS],
}

impl Scratch {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            limbs: [const { MaybeUninit::uninit() }; SCRATCH_LIMBS],
        }
    }
}

/// Writes the digit of the big-endian `input` into `place`
///
/// # Safety
/// None here; `unsafe` as the delegated one is.
#[inline(always)]
pub(crate) unsafe fn from_be_bytes_in_place(
    input: &[u8; DIGIT_BYTES],
    place: &mut MaybeUninit<Digit>,
) {
    place.write(u64::from_be_bytes(*input));
}

/// The big-endian bytes of `digit`
#[inline(always)]
pub(crate) fn to_be_bytes(digit: &Digit) -> [u8; DIGIT_BYTES] {
    digit.to_be_bytes()
}

#[inline(always)]
pub(crate) fn is_zero(digit: &Digit) -> bool {
    *digit == 0
}

#[inline(always)]
pub(crate) fn is_one(digit: &Digit) -> bool {
    *digit == 1
}

/// Copies `src` into the first slots of `dst`
#[inline]
pub(crate) fn copy(dst: &mut [MaybeUninit<Digit>], src: &[Digit]) {
    assert!(dst.len() >= src.len());
    for (dst, src) in dst.iter_mut().zip(src) {
        dst.write(*src);
    }
}

/// `(low, high)` of `a * b + c + d`, which fits two limbs: at most
/// `(2^64 - 1)^2 + 2 (2^64 - 1) = 2^128 - 1`
#[inline(always)]
fn mac(a: u64, b: u64, c: u64, d: u64) -> (u64, u64) {
    let t = u128::from(a) * u128::from(b) + u128::from(c) + u128::from(d);
    (t as u64, (t >> 64) as u64)
}

/// `a + b + carry` and its carry, for a carry of 0 or 1
#[inline(always)]
fn adc(a: u64, b: u64, carry: u64) -> (u64, u64) {
    let (sum, c1) = a.overflowing_add(b);
    let (sum, c2) = sum.overflowing_add(carry);
    (sum, u64::from(c1 | c2))
}

/// `a - b - borrow` and its borrow, for a borrow of 0 or 1
#[inline(always)]
fn sbb(a: u64, b: u64, borrow: u64) -> (u64, u64) {
    let (difference, b1) = a.overflowing_sub(b);
    let (difference, b2) = difference.overflowing_sub(borrow);
    (difference, u64::from(b1 | b2))
}

/// The limbs `out`, which the caller wrote
///
/// # Safety
/// Every limb of `out` must be initialized.
#[inline(always)]
unsafe fn written(out: &mut [MaybeUninit<u64>]) -> &mut [u64] {
    // SAFETY: guaranteed by the caller
    unsafe { out.assume_init_mut() }
}

/// `out[i], carry = step(x[i], carry)` over the limbs of `x`, unrolled by four; returns `out`,
/// all written (it has the length of `x`), and the last carry
#[inline(always)]
fn map_carry<'a>(
    out: &'a mut [MaybeUninit<u64>],
    x: &[u64],
    mut carry: u64,
    step: impl Fn(u64, u64) -> (u64, u64),
) -> (&'a mut [u64], u64) {
    assert!(out.len() == x.len());
    let (out_chunks, out_rest) = out.as_chunks_mut::<4>();
    let (x_chunks, x_rest) = x.as_chunks::<4>();
    for (out, x) in out_chunks.iter_mut().zip(x_chunks) {
        let (l0, c) = step(x[0], carry);
        let (l1, c) = step(x[1], c);
        let (l2, c) = step(x[2], c);
        let (l3, c) = step(x[3], c);
        out[0].write(l0);
        out[1].write(l1);
        out[2].write(l2);
        out[3].write(l3);
        carry = c;
    }
    for (out, x) in out_rest.iter_mut().zip(x_rest) {
        let (limb, c) = step(*x, carry);
        out.write(limb);
        carry = c;
    }
    // SAFETY: the chunks and the rest are all of `out`, as long as `x`
    (unsafe { written(out) }, carry)
}

/// `out[i], carry = step(x[i], y[i], carry)` over the limbs of `x` and `y`, of one length,
/// unrolled by four; returns `out`, all written (it has their length), and the last carry
#[inline(always)]
fn zip_carry<'a>(
    out: &'a mut [MaybeUninit<u64>],
    x: &[u64],
    y: &[u64],
    mut carry: u64,
    step: impl Fn(u64, u64, u64) -> (u64, u64),
) -> (&'a mut [u64], u64) {
    assert!(out.len() == x.len() && x.len() == y.len());
    let (out_chunks, out_rest) = out.as_chunks_mut::<4>();
    let (x_chunks, x_rest) = x.as_chunks::<4>();
    let (y_chunks, y_rest) = y.as_chunks::<4>();
    for ((out, x), y) in out_chunks.iter_mut().zip(x_chunks).zip(y_chunks) {
        let (l0, c) = step(x[0], y[0], carry);
        let (l1, c) = step(x[1], y[1], c);
        let (l2, c) = step(x[2], y[2], c);
        let (l3, c) = step(x[3], y[3], c);
        out[0].write(l0);
        out[1].write(l1);
        out[2].write(l2);
        out[3].write(l3);
        carry = c;
    }
    for ((out, x), y) in out_rest.iter_mut().zip(x_rest).zip(y_rest) {
        let (limb, c) = step(*x, *y, carry);
        out.write(limb);
        carry = c;
    }
    // SAFETY: the chunks and the rest are all of `out`, as long as `x` and `y`
    (unsafe { written(out) }, carry)
}

/// `acc[i], carry = step(acc[i], x[i], carry)` over the limbs of `acc` and `x`, of one length,
/// unrolled by four; returns the last carry
#[inline(always)]
fn zip_carry_assign(
    acc: &mut [u64],
    x: &[u64],
    mut carry: u64,
    step: impl Fn(u64, u64, u64) -> (u64, u64),
) -> u64 {
    assert!(acc.len() == x.len());
    let (acc_chunks, acc_rest) = acc.as_chunks_mut::<4>();
    let (x_chunks, x_rest) = x.as_chunks::<4>();
    for (acc, x) in acc_chunks.iter_mut().zip(x_chunks) {
        let (l0, c) = step(acc[0], x[0], carry);
        let (l1, c) = step(acc[1], x[1], c);
        let (l2, c) = step(acc[2], x[2], c);
        let (l3, c) = step(acc[3], x[3], c);
        *acc = [l0, l1, l2, l3];
        carry = c;
    }
    for (acc, x) in acc_rest.iter_mut().zip(x_rest) {
        (*acc, carry) = step(*acc, *x, carry);
    }
    carry
}

/// `out = a * b + carry` for one limb `b`, in the limbs of `a`; returns the limb above them
#[inline(always)]
fn mul_row(out: &mut [MaybeUninit<u64>], a: &[u64], b: u64, carry: u64) -> u64 {
    map_carry(out, a, carry, |a, carry| mac(a, b, carry, 0)).1
}

/// `out = a * b + addend` for one limb `b`, in the limbs of `a`; returns the limb above them
#[inline(always)]
fn mul_add_row(out: &mut [MaybeUninit<u64>], a: &[u64], b: u64, addend: &[u64]) -> u64 {
    zip_carry(out, a, addend, 0, |a, addend, carry| {
        mac(a, b, addend, carry)
    })
    .1
}

/// `acc += a * b` for one limb `b`, in the limbs of `a`; returns the limb above them
#[inline(always)]
fn add_mul_row(acc: &mut [u64], a: &[u64], b: u64) -> u64 {
    zip_carry_assign(acc, a, 0, |acc, a, carry| mac(a, b, acc, carry))
}

/// `acc += x`, and its carry
#[inline(always)]
fn add_assign(acc: &mut [u64], x: &[u64]) -> u64 {
    zip_carry_assign(acc, x, 0, adc)
}

/// `acc -= x`, and its borrow
#[inline(always)]
fn sub_assign(acc: &mut [u64], x: &[u64]) -> u64 {
    zip_carry_assign(acc, x, 0, sbb)
}

/// `acc += limb`, and its carry
#[inline]
fn add_limb(acc: &mut [u64], limb: u64) -> u64 {
    let mut carry = limb;
    for acc in acc.iter_mut() {
        if carry == 0 {
            break;
        }
        let (sum, overflow) = acc.overflowing_add(carry);
        *acc = sum;
        carry = u64::from(overflow);
    }
    carry
}

/// `out = x + y` for a `y` of no more limbs than `x`, in the limbs of `x`, and its carry
#[inline(always)]
fn add_into<'a>(out: &'a mut [MaybeUninit<u64>], x: &[u64], y: &[u64]) -> (&'a mut [u64], u64) {
    let (out_low, out_high) = out.split_at_mut(y.len());
    let (x_low, x_high) = x.split_at(y.len());
    let (_, carry) = zip_carry(out_low, x_low, y, 0, adc);
    let (_, carry) = map_carry(out_high, x_high, carry, |x, carry| adc(x, 0, carry));
    // SAFETY: both parts were written
    (unsafe { written(out) }, carry)
}

/// `out = |x - y|` for a `y` of no more limbs than `x`, in the limbs of `x`, and whether
/// `x < y`
#[inline(always)]
fn abs_diff_into<'a>(
    out: &'a mut [MaybeUninit<u64>],
    x: &[u64],
    y: &[u64],
) -> (&'a mut [u64], bool) {
    let (x_low, x_high) = x.split_at(y.len());
    let order = if x_high.iter().any(|limb| *limb != 0) {
        Ordering::Greater
    } else {
        compare(x_low, y)
    };
    let (out_low, out_high) = out.split_at_mut(y.len());
    if order == Ordering::Less {
        // x < y, so the limbs of `x` above `y` are zero
        zip_carry(out_low, y, x_low, 0, sbb);
        for slot in out_high.iter_mut() {
            slot.write(0);
        }
    } else {
        let (_, borrow) = zip_carry(out_low, x_low, y, 0, sbb);
        map_carry(out_high, x_high, borrow, |x, borrow| sbb(x, 0, borrow));
    }
    // SAFETY: both parts were written
    (unsafe { written(out) }, order == Ordering::Less)
}

/// The order of `x` and `y`, of one length: the first limbs that differ, from the top, decide
#[inline]
fn compare(x: &[u64], y: &[u64]) -> Ordering {
    assert!(x.len() == y.len());
    for (x, y) in x.iter().zip(y).rev() {
        if x != y {
            return if x < y {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
    }
    Ordering::Equal
}

/// `out = a * b` by rows, for non-empty `a` and `b`, in the `a.len() + b.len()` limbs of `out`,
/// all written: a row of the longer operand for each limb of the shorter
fn mul_schoolbook(out: &mut [MaybeUninit<u64>], a: &[u64], b: &[u64]) {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let n = long.len();
    assert!(!short.is_empty() && out.len() == n + short.len());
    let top = mul_row(&mut out[..n], long, short[0], 0);
    out[n].write(top);
    for (j, limb) in short.iter().enumerate().skip(1) {
        // SAFETY: the rows before wrote `out[..j + n]`
        let acc = unsafe { written(&mut out[j..j + n]) };
        let top = add_mul_row(acc, long, *limb);
        out[j + n].write(top);
    }
}

/// `out = a * a` by rows, for a non-empty `a`, in the `2 * a.len()` limbs of `out`, all
/// written: the products `a_i a_j` (`i < j`) once, then one pass that doubles them and adds the
/// squares `a_i^2`. `n (n + 1) / 2` limb products instead of `n^2`.
fn square_schoolbook(out: &mut [MaybeUninit<u64>], a: &[u64]) {
    let n = a.len();
    assert!(n > 0 && out.len() == 2 * n);
    // the cross products, in limbs `1..2 n - 1`
    out[0].write(0);
    if n > 1 {
        let top = mul_row(&mut out[1..n], &a[1..], a[0], 0);
        out[n].write(top);
        for i in 1..n - 1 {
            // SAFETY: the rows before wrote `out[..i + n]`
            let acc = unsafe { written(&mut out[2 * i + 1..i + n]) };
            let top = add_mul_row(acc, &a[i + 1..], a[i]);
            out[i + n].write(top);
        }
    }
    out[2 * n - 1].write(0);
    // SAFETY: limb 0, the rows (limbs `1..2 n - 1`) and the last limb were written
    let out = unsafe { written(out) };
    let (mut shifted_out, mut carry) = (0, false);
    for (pair, limb) in out.as_chunks_mut::<2>().0.iter_mut().zip(a) {
        let low = (pair[0] << 1) | shifted_out;
        let high = (pair[1] << 1) | (pair[0] >> 63);
        shifted_out = pair[1] >> 63;
        // a_i^2 + low + carry fits two limbs
        let (low, square_high) = mac(*limb, *limb, low, u64::from(carry));
        pair[0] = low;
        (pair[1], carry) = high.overflowing_add(square_high);
    }
    // a square of `n` limbs has `2 n`
    debug_assert!(!carry && shifted_out == 0);
}

/// `out = a * b` for operands of one length `n > 0`, in the `2 n` limbs of `out`, all written.
///
/// From [`KARATSUBA_MUL_MIN`] limbs on, with `a = a0 + a1 B^h` and `b = b0 + b1 B^h`
/// (`h = floor(n / 2)`): `a b = z0 + (z0 + z2 - (a1 - a0) (b1 - b0)) B^h + z2 B^(2 h)` for
/// `z0 = a0 b0` and `z2 = a1 b1`, three products of half the length. `scratch` holds
/// `karatsuba_space(n)` limbs.
fn mul_balanced(
    out: &mut [MaybeUninit<u64>],
    a: &[u64],
    b: &[u64],
    scratch: &mut [MaybeUninit<u64>],
) {
    let n = a.len();
    assert!(n == b.len() && out.len() == 2 * n);
    if n < KARATSUBA_MUL_MIN {
        return mul_schoolbook(out, a, b);
    }
    let low = n / 2;
    let high = n - low;
    let (a0, a1) = a.split_at(low);
    let (b0, b1) = b.split_at(low);
    let (work, deeper) = scratch.split_at_mut(4 * high);
    let (sums, product) = work.split_at_mut(2 * high);
    let (a_difference, b_difference) = sums.split_at_mut(high);
    let (a_difference, a_negative) = abs_diff_into(a_difference, a1, a0);
    let (b_difference, b_negative) = abs_diff_into(b_difference, b1, b0);
    let (z0, z2) = out.split_at_mut(2 * low);
    mul_balanced(z0, a0, b0, deeper);
    mul_balanced(z2, a1, b1, deeper);
    mul_balanced(product, a_difference, b_difference, deeper);
    // SAFETY: the three products wrote all their limbs
    let (out, product) = unsafe { (written(out), written(product)) };
    finish_karatsuba(out, sums, product, low, a_negative == b_negative);
}

/// `out = a * a` for `n > 0` limbs, in the `2 n` limbs of `out`, all written: as
/// [`mul_balanced`], with three squares from [`KARATSUBA_SQUARE_MIN`] limbs on
fn square_balanced(out: &mut [MaybeUninit<u64>], a: &[u64], scratch: &mut [MaybeUninit<u64>]) {
    let n = a.len();
    assert!(out.len() == 2 * n);
    if n < KARATSUBA_SQUARE_MIN {
        return square_schoolbook(out, a);
    }
    let low = n / 2;
    let high = n - low;
    let (a0, a1) = a.split_at(low);
    let (work, deeper) = scratch.split_at_mut(4 * high);
    let (sums, product) = work.split_at_mut(2 * high);
    let (difference, _) = abs_diff_into(&mut sums[..high], a1, a0);
    let (z0, z2) = out.split_at_mut(2 * low);
    square_balanced(z0, a0, deeper);
    square_balanced(z2, a1, deeper);
    square_balanced(product, difference, deeper);
    // SAFETY: the three squares wrote all their limbs
    let (out, product) = unsafe { (written(out), written(product)) };
    finish_karatsuba(out, sums, product, low, true);
}

/// The last step of a Karatsuba product: `out` holds `z0` in its `2 low` low limbs and `z2`
/// above them, and gets `(z0 + z2 - product) B^low` added (`+ product` unless `subtract`).
/// `sums` is working space of the limbs of `z2`.
#[inline(always)]
fn finish_karatsuba(
    out: &mut [u64],
    sums: &mut [MaybeUninit<u64>],
    product: &[u64],
    low: usize,
    subtract: bool,
) {
    let (z0, z2) = out.split_at(2 * low);
    // the middle term, `a0 b1 + a1 b0`, in the limbs of `z2` and a limb above them: it is not
    // negative, and below `2 B^n`
    let (middle, mut top) = add_into(sums, z2, z0);
    if subtract {
        top = top.wrapping_sub(sub_assign(middle, product));
    } else {
        top += add_assign(middle, product);
    }
    let (target, above) = out[low..].split_at_mut(middle.len());
    top += add_assign(target, middle);
    let carry = add_limb(above, top);
    // the product has the limbs of `out`
    debug_assert!(carry == 0);
}

/// `out = a * b` for non-empty `a` and `b`, in the `a.len() + b.len()` limbs of `out`, all
/// written
pub(crate) fn mul(out: &mut [MaybeUninit<Digit>], a: &[Digit], b: &[Digit], scratch: &mut Scratch) {
    if a.len() == b.len() {
        mul_balanced(out, a, b, &mut scratch.limbs);
    } else {
        mul_schoolbook(out, a, b);
    }
}

/// `out = a * a` for a non-empty `a`, in the `2 * a.len()` limbs of `out`, all written
pub(crate) fn square(out: &mut [MaybeUninit<Digit>], a: &[Digit], scratch: &mut Scratch) {
    square_balanced(out, a, &mut scratch.limbs);
}

/// The limbs of `x` without its zero top limbs
#[inline]
fn significant(x: &[u64]) -> &[u64] {
    let mut len = x.len();
    while len > 0 && x[len - 1] == 0 {
        len -= 1;
    }
    &x[..len]
}

/// `out = q * m + r` in exactly the limbs of `out`. Returns whether the value fits, so that a
/// hint reconstructing a truncated value cannot pass the comparison with the product; when it
/// does, all of `out` is written. `q`, `m` and `r` may have any content and any number of limbs
/// (they are untrusted): nothing is written outside `out`.
#[must_use]
pub(crate) fn mul_add(
    out: &mut [MaybeUninit<Digit>],
    q: &[Digit],
    m: &[Digit],
    r: &[Digit],
    scratch: &mut Scratch,
) -> bool {
    let (q, m, r) = (significant(q), significant(m), significant(r));
    let len = out.len();
    if r.len() > len {
        return false;
    }
    if q.is_empty() || m.is_empty() {
        // the value is `r`
        let (low, high) = out.split_at_mut(r.len());
        copy(low, r);
        for slot in high.iter_mut() {
            slot.write(0);
        }
        return true;
    }
    let (long, short) = if q.len() >= m.len() { (q, m) } else { (m, q) };
    let n = long.len();
    // a product of operands with non-zero top limbs has at least this many limbs
    if n + short.len() - 1 > len {
        return false;
    }
    if r.len() > n {
        mul_add_long_addend(out, long, short, r)
    } else if short.len() == n && n >= KARATSUBA_MUL_MIN {
        mul_add_balanced(out, long, short, r, scratch)
    } else {
        mul_add_rows(out, long, short, r)
    }
}

/// [`mul_add`] by rows, for `long` and `short` with non-zero top limbs, whose product has at
/// most a limb more than `out`, and an `r` of at most the limbs of `long`: the first row adds
/// `r`, and every row writes the limb above it
#[must_use]
fn mul_add_rows(out: &mut [MaybeUninit<u64>], long: &[u64], short: &[u64], r: &[u64]) -> bool {
    let (len, n) = (out.len(), long.len());
    // the first row, within `out`: n <= n + short.len() - 1 <= len
    let (with_r, without_r) = out[..n].split_at_mut(r.len());
    let (long_low, long_high) = long.split_at(r.len());
    let carry = mul_add_row(with_r, long_low, short[0], r);
    let mut top = mul_row(without_r, long_high, short[0], carry);
    for (j, limb) in short.iter().enumerate().skip(1) {
        // the limb above the row before: j + n - 1 <= len - 1
        out[j + n - 1].write(top);
        // SAFETY: the rows before wrote `out[..j + n]`, which is within `out`
        let acc = unsafe { written(&mut out[j..j + n]) };
        top = add_mul_row(acc, long, *limb);
    }
    // the limb above the last row, and zeroes above it
    let end = short.len() + n - 1;
    if end == len {
        return top == 0;
    }
    out[end].write(top);
    for slot in out[end + 1..].iter_mut() {
        slot.write(0);
    }
    true
}

/// [`mul_add`] for operands of one length `n >= KARATSUBA_MUL_MIN` with non-zero top limbs
/// (so `out` has at least `2 n - 1` limbs) and an `r` of at most `n` limbs: the product, then
/// `r` added. The product is made in `out` if that has its `2 n` limbs, else in the working
/// space.
#[must_use]
fn mul_add_balanced(
    out: &mut [MaybeUninit<u64>],
    a: &[u64],
    b: &[u64],
    r: &[u64],
    scratch: &mut Scratch,
) -> bool {
    let (len, n) = (out.len(), a.len());
    if len == 2 * n {
        mul_balanced(out, a, b, &mut scratch.limbs);
        // SAFETY: the product wrote all its limbs
        let out = unsafe { written(out) };
        let (low, high) = out.split_at_mut(r.len());
        let carry = add_assign(low, r);
        return add_limb(high, carry) == 0;
    }
    let (product, deeper) = scratch.limbs.split_at_mut(2 * n);
    mul_balanced(product, a, b, deeper);
    // SAFETY: the product wrote all its limbs
    let product = unsafe { written(product) };
    // the limbs of the sum that `out` holds: all of them, or all but the top one
    let (product, above) = product.split_at(len.min(2 * n));
    if above.iter().any(|limb| *limb != 0) {
        return false;
    }
    let (out, zeroes) = out.split_at_mut(product.len());
    let (_, carry) = add_into(out, product, r);
    match zeroes.split_first_mut() {
        Some((first, zeroes)) => {
            first.write(carry);
            for slot in zeroes.iter_mut() {
                slot.write(0);
            }
            true
        }
        None => carry == 0,
    }
}

/// [`mul_add`] for an `r` of any length up to that of `out`: `out = r`, then a row of `long`
/// for each limb of `short`, and the limb above each row into the limbs above it. Not on the
/// path of ModExp, where a remainder has at most the limbs of the modulus.
#[must_use]
fn mul_add_long_addend(
    out: &mut [MaybeUninit<u64>],
    long: &[u64],
    short: &[u64],
    r: &[u64],
) -> bool {
    let n = long.len();
    let (low, high) = out.split_at_mut(r.len());
    copy(low, r);
    for slot in high.iter_mut() {
        slot.write(0);
    }
    // SAFETY: `r` and the zeroes after it were just written
    let out = unsafe { written(out) };
    for (j, limb) in short.iter().enumerate() {
        // within `out`: j + n <= n + short.len() - 1 <= out.len()
        let top = add_mul_row(&mut out[j..j + n], long, *limb);
        if add_limb(&mut out[j + n..], top) != 0 {
            return false;
        }
    }
    true
}

/// Whether `a` and `b` are the same integer (zero top limbs do not matter)
#[must_use]
pub(crate) fn eq_value(a: &[Digit], b: &[Digit]) -> bool {
    let common = a.len().min(b.len());
    a[..common] == b[..common]
        && a[common..]
            .iter()
            .chain(&b[common..])
            .all(|limb| *limb == 0)
}

/// Whether `a < m`, for strings of one length. `m` is not written (the delegated one uses it
/// as scratch).
#[must_use]
pub(crate) fn is_below(a: &[Digit], m: &mut [Digit]) -> bool {
    compare(a, m) == Ordering::Less
}

/// Limbs of a 256-bit digit of the advice request
const PROTOCOL_DIGIT_LIMBS: usize = 4;

/// `limbs` limbs, in the 256-bit digits of the advice request (which counts its dividend and
/// its modulus in them), rounded up
#[inline(always)]
pub(crate) const fn protocol_digits(limbs: usize) -> usize {
    limbs.div_ceil(PROTOCOL_DIGIT_LIMBS)
}

/// Zeroes the limbs after a value of `limbs` limbs that complete its last 256-bit digit of the
/// advice request (at most three), which the oracle reads with it. `spare` is the buffer after
/// the value, with room for them.
#[inline]
pub(crate) fn pad_for_query(spare: &mut [MaybeUninit<Digit>], limbs: usize) {
    let padding = protocol_digits(limbs) * PROTOCOL_DIGIT_LIMBS - limbs;
    for slot in spare[..padding].iter_mut() {
        slot.write(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;
    use std::vec;
    use std::vec::Vec;

    /// A deterministic xorshift64* generator
    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        /// `len` limbs: random, or mostly all ones or zero (long carry and borrow chains)
        fn limbs(&mut self, len: usize) -> Vec<u64> {
            let kind = self.next_u64() % 4;
            (0..len)
                .map(|_| match (kind, self.next_u64() % 8) {
                    (0 | 1, _) | (_, 0) => self.next_u64(),
                    (2, _) => u64::MAX,
                    _ => 0,
                })
                .collect()
        }
    }

    fn big(limbs: &[u64]) -> BigUint {
        let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
        BigUint::from_bytes_le(&bytes)
    }

    /// The `len` low limbs of `x`, which has no more
    fn limbs_of(x: &BigUint, len: usize) -> Vec<u64> {
        let mut limbs: Vec<u64> = x.iter_u64_digits().collect();
        assert!(limbs.len() <= len, "the value has more than {len} limbs");
        limbs.resize(len, 0);
        limbs
    }

    /// Lengths around the thresholds, odd ones, and the longest operands
    const LENGTHS: [usize; 22] = [
        1, 2, 3, 4, 5, 7, 8, 15, 16, 23, 24, 25, 31, 32, 47, 48, 49, 63, 64, 97, 127, 128,
    ];

    #[test]
    fn the_working_space_holds_the_longest_product() {
        // every level of a product of `n` limbs takes `4 ceil(n / 2)` of the space it is given
        fn needed(n: usize, min: usize) -> usize {
            if n < min {
                0
            } else {
                4 * n.div_ceil(2) + needed(n.div_ceil(2), min)
            }
        }
        const { assert!(KARATSUBA_MUL_MIN > 4 && KARATSUBA_SQUARE_MIN > 4) };
        let min = KARATSUBA_MUL_MIN.min(KARATSUBA_SQUARE_MIN);
        for n in 1..=MAX_LIMBS {
            assert!(needed(n, min) <= karatsuba_space(MAX_LIMBS));
            assert!(2 * n + needed(n, min) <= SCRATCH_LIMBS);
        }
    }

    #[test]
    fn products_match_reference() {
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        let mut scratch = Scratch::new();
        for round in 0..40 {
            for a_len in LENGTHS {
                for b_len in LENGTHS {
                    // every pair of lengths once, and equal lengths every round
                    if round > 0 && a_len != b_len {
                        continue;
                    }
                    let (a, b) = (rng.limbs(a_len), rng.limbs(b_len));
                    let mut out = vec![MaybeUninit::uninit(); a_len + b_len];
                    mul(&mut out, &a, &b, &mut scratch);
                    // SAFETY: a product writes all its limbs
                    let product = unsafe { written(&mut out) };
                    assert_eq!(
                        product,
                        limbs_of(&(big(&a) * big(&b)), a_len + b_len),
                        "{a_len} by {b_len} limbs"
                    );
                }
                let a = rng.limbs(a_len);
                let mut out = vec![MaybeUninit::uninit(); 2 * a_len];
                square(&mut out, &a, &mut scratch);
                // SAFETY: a square writes all its limbs
                let squared = unsafe { written(&mut out) };
                assert_eq!(
                    squared,
                    limbs_of(&(big(&a) * big(&a)), 2 * a_len),
                    "{a_len} limbs squared"
                );
            }
        }
    }

    /// `q * m + r` in `len` limbs, against the reference: the value when it fits, and that it
    /// does not otherwise
    fn check_mul_add(q: &[u64], m: &[u64], r: &[u64], len: usize, scratch: &mut Scratch) -> bool {
        let expected = big(q) * big(m) + big(r);
        let mut out = vec![MaybeUninit::uninit(); len];
        let fits = mul_add(&mut out, q, m, r, scratch);
        let expected_fits = expected.bits() <= 64 * len as u64;
        assert_eq!(
            fits,
            expected_fits,
            "{} * {} + {} limbs in {len}",
            q.len(),
            m.len(),
            r.len()
        );
        if fits {
            // SAFETY: a value that fits is written in all the limbs
            assert_eq!(unsafe { written(&mut out) }, limbs_of(&expected, len));
        }
        fits
    }

    #[test]
    fn products_with_addend_match_reference() {
        let mut rng = Rng(0xFEDC_BA98_7654_3210);
        let mut scratch = Scratch::new();
        let (mut fit, mut overflowed) = (0, 0);
        for round in 0..4 {
            for q_len in LENGTHS {
                for m_len in LENGTHS {
                    if round > 0 && q_len != m_len && !rng.next_u64().is_multiple_of(4) {
                        continue;
                    }
                    let (mut q, mut m) = (rng.limbs(q_len), rng.limbs(m_len));
                    // zero top limbs, which the operands may have
                    if rng.next_u64().is_multiple_of(8) {
                        q.push(0);
                    }
                    if rng.next_u64().is_multiple_of(8) {
                        m.push(0);
                    }
                    // a remainder as the path gives (no longer than the modulus), a short one,
                    // none, and one of any length
                    let r_len = match rng.next_u64() % 8 {
                        0 => 0,
                        1 => (rng.next_u64() as usize) % (q_len + m_len + 3),
                        2 => (rng.next_u64() as usize) % (m_len + 1),
                        _ => m_len,
                    };
                    let r = rng.limbs(r_len);
                    // the limbs of the product, one fewer, more, and any number
                    let product = q_len + m_len;
                    for len in [
                        product,
                        product - 1,
                        product + 1,
                        product + 5,
                        product.saturating_sub(2),
                        (rng.next_u64() as usize) % (product + 2),
                    ] {
                        if check_mul_add(&q, &m, &r, len, &mut scratch) {
                            fit += 1;
                        } else {
                            overflowed += 1;
                        }
                    }
                }
            }
        }
        assert!(
            fit > 1000 && overflowed > 1000,
            "{fit} fit, {overflowed} do not"
        );
    }

    /// The edges of the range of `q * m + r`: sums that fill the limbs exactly, and ones a
    /// limb above
    #[test]
    fn products_with_addend_at_the_edge_of_the_limbs() {
        let mut scratch = Scratch::new();
        for n in [1, 2, 5, 8, 23, 24, 32, 33, 64, 128] {
            let ones = vec![u64::MAX; n];
            // (B^n - 1)^2 + (B^n - 1) = B^n (B^n - 1): the largest sum of the path, which fits
            assert!(check_mul_add(&ones, &ones, &ones, 2 * n, &mut scratch));
            assert!(!check_mul_add(&ones, &ones, &ones, 2 * n - 1, &mut scratch));
            // B^(n - 1) * B^(n - 1) = B^(2 n - 2): in 2 n - 1 limbs, with any remainder of
            // fewer limbs
            let mut power = vec![0; n];
            power[n - 1] = 1;
            assert!(check_mul_add(
                &power,
                &power,
                &ones[1..],
                2 * n - 1,
                &mut scratch
            ));
            assert_eq!(
                check_mul_add(&power, &power, &ones, 2 * n - 1, &mut scratch),
                n > 1
            );
            // (B^n - 1) * 1 + r carries into limb n exactly when r is not zero
            assert!(check_mul_add(&ones, &[1], &[], n, &mut scratch));
            assert!(!check_mul_add(&ones, &[1], &[1], n, &mut scratch));
            assert!(check_mul_add(&ones, &[1], &[1], n + 1, &mut scratch));
            // zero operands: the value is the remainder
            assert!(check_mul_add(&[], &ones, &ones, n, &mut scratch));
            assert!(check_mul_add(&ones, &[0, 0], &ones, n + 2, &mut scratch));
            assert!(!check_mul_add(&[], &ones, &ones, n - 1, &mut scratch));
        }
    }

    #[test]
    fn comparisons_and_padding() {
        assert!(eq_value(&[1, 2], &[1, 2, 0, 0]) && eq_value(&[], &[0]));
        assert!(!eq_value(&[1, 2], &[1, 2, 0, 1]) && !eq_value(&[1, 2], &[1, 3]));
        assert!(is_below(&[5, 1], &mut [4, 2]) && !is_below(&[4, 2], &mut [4, 2]));
        assert!(!is_below(&[0, 3], &mut [u64::MAX, 2]) && is_below(&[3], &mut [4]));
        assert_eq!([0, 1, 4, 5, 8].map(protocol_digits), [0, 1, 1, 2, 2]);
        for limbs in 0..9 {
            let mut spare = [MaybeUninit::new(7u64); 4];
            pad_for_query(&mut spare, limbs);
            // SAFETY: the slots were initialized above
            let spare = unsafe { written(&mut spare) };
            let padding = (4 - limbs % 4) % 4;
            assert!(spare[..padding].iter().all(|limb| *limb == 0));
            assert!(spare[padding..].iter().all(|limb| *limb == 7));
        }
    }
}
