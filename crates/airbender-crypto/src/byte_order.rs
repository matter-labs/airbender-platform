//! Byte-order helpers for RV32, used by the precompiles and ModExp.
//!
//! Airbender runs the Zbb `rev8` instruction in one cycle but no other Zbb instruction, so
//! `+zbb` cannot be enabled target-wide and LLVM lowers `swap_bytes` to 11 instructions; these
//! helpers emit `rev8` directly. None of them accesses bytes outside its objects.

/// Reverses the bytes of one word. On riscv32 this uses a scoped Zbb `rev8`.
#[inline(always)]
pub fn bswap32(value: u32) -> u32 {
    #[cfg(target_arch = "riscv32")]
    {
        let result: u32;
        // SAFETY: `rev8` only reads the input register and writes the output register.
        unsafe {
            core::arch::asm!(
                ".option push",
                ".option arch, +zbb",
                "rev8 {rd}, {rs1}",
                ".option pop",
                rd = lateout(reg) result,
                rs1 = in(reg) value,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        result
    }

    #[cfg(not(target_arch = "riscv32"))]
    {
        value.swap_bytes()
    }
}

/// Reads the `N`-byte big-endian integer at `src` into `u32` limbs at `dst`, least significant
/// first, the top limb zero-padded. An unaligned source is read as the aligned words inside it,
/// shifted together with the edge bytes. Only whole words are written.
///
/// # Safety
/// `N` must be in `1..=32`. `src` must point to `N` readable bytes. `dst`
/// must be 4-aligned and writable for `4 * N.div_ceil(4)` bytes. Source and
/// destination must not overlap.
// TODO: Investigate the correct approach to avoid warning here
#[allow(dead_code)]
#[inline(always)]
pub unsafe fn read_be_into_words<const N: usize>(src: *const u8, dst: *mut u32) {
    const { assert!(N >= 1 && N <= 32) };

    unsafe {
        let partial = N % 4;
        let full_words = N / 4;
        let words_src = src.add(partial);
        let misalignment = words_src.addr() % 4;
        if misalignment == 0 {
            let src_words = words_src.cast::<u32>();
            for i in 0..full_words {
                dst.add(full_words - 1 - i)
                    .write(bswap32(src_words.add(i).read()));
            }
        } else {
            // the hint must be in this block, not behind the length test, to weight the branch
            core::hint::cold_path();
            if full_words != 0 {
                read_be_words_unaligned::<N>(words_src, dst, misalignment);
            }
        }
        if partial != 0 {
            let mut value = 0u32;
            for i in 0..partial {
                value = (value << 8) | u32::from(src.add(i).read());
            }
            dst.add(full_words).write(value);
        }
    }
}

/// The unaligned path of [`read_be_into_words`]: reads the `N / 4` full words at
/// `src`, which is `misalignment` (1 to 3) bytes past a word boundary.
#[inline(always)]
unsafe fn read_be_words_unaligned<const N: usize>(
    src: *const u8,
    dst: *mut u32,
    misalignment: usize,
) {
    unsafe {
        match misalignment {
            1 => read_be_words_shifted::<N, 1>(src, dst),
            2 => read_be_words_shifted::<N, 2>(src, dst),
            // 3, the only other value of `addr % 4` here
            _ => {
                debug_assert_eq!(misalignment, 3);
                read_be_words_shifted::<N, 3>(src, dst)
            }
        }
    }
}

/// Reads the `W = N / 4` (at least one) big-endian words at `src`, `K` bytes past a word
/// boundary, into `W` words at `dst`: the `W - 1` aligned inner words as words, the `4 - K` head
/// and `K` tail bytes as bytes and aligned halfwords, combined by shifts.
#[inline(always)]
unsafe fn read_be_words_shifted<const N: usize, const K: usize>(src: *const u8, dst: *mut u32) {
    if N / 4 == 0 {
        return;
    }
    const { assert!(K >= 1 && K <= 3) };

    let words = N / 4;
    debug_assert!(words != 0);
    let low_shift = 8 * K as u32;
    let high_shift = 32 - low_shift;
    unsafe {
        let inner = src.add(4 - K).cast::<u32>();
        // Source bytes [0, 4 - K) in little-endian order.
        let head = match K {
            1 => u32::from(src.read()) | (u32::from(src.add(1).cast::<u16>().read()) << 8),
            2 => u32::from(src.cast::<u16>().read()),
            _ => u32::from(src.read()),
        };
        // Source bytes [4 * W - K, 4 * W) in little-endian order, from an aligned address.
        let tail_bytes = src.add(4 * words).sub(K);
        let tail = match K {
            1 => u32::from(tail_bytes.read()),
            2 => u32::from(tail_bytes.cast::<u16>().read()),
            _ => {
                u32::from(tail_bytes.cast::<u16>().read())
                    | (u32::from(tail_bytes.add(2).read()) << 16)
            }
        };
        let mut low = head;
        for i in 0..words {
            let next = if i + 1 < words {
                inner.add(i).read()
            } else {
                tail
            };
            dst.add(words - 1 - i)
                .write(bswap32(low | (next << high_shift)));
            low = next >> low_shift;
        }
    }
}

/// Returns eight aligned source words as 32 big-endian bytes, by value and without
/// branches, so that a local destination can stay in registers.
///
/// # Safety
/// `src` must be 4-aligned and point to eight initialized words.
// TODO: Investigate the correct approach to avoid warning here
#[allow(dead_code)]
#[inline(always)]
pub unsafe fn words_to_be_bytes_32(src: *const u32) -> [u8; 32] {
    let words: [u32; 8] = core::array::from_fn(|i| bswap32(unsafe { src.add(7 - i).read() }));
    // SAFETY: `[u32; 8]` and `[u8; 32]` have the same size, and every bit pattern is valid.
    unsafe { core::mem::transmute::<[u32; 8], [u8; 32]>(words) }
}

/// Reads a big-endian integer into little-endian `u64` limbs.
/// The word path is used when `bytes` is 4-aligned; otherwise it reads byte chunks.
#[inline(always)]
pub fn be_bytes_to_le_limbs(bytes: &[u8; 32]) -> [u64; 4] {
    if bytes.as_ptr().addr().is_multiple_of(4) {
        let words = bytes.as_ptr().cast::<u32>();
        core::array::from_fn(|i| {
            let c = 3 - i;
            // SAFETY: the source is 4-aligned and contains eight readable words.
            let hi = bswap32(unsafe { words.add(2 * c).read() });
            let lo = bswap32(unsafe { words.add(2 * c + 1).read() });
            (u64::from(hi) << 32) | u64::from(lo)
        })
    } else {
        let chunks = bytes.as_chunks::<8>().0;
        core::array::from_fn(|i| u64::from_be_bytes(chunks[3 - i]))
    }
}

/// Writes little-endian `u64` limbs as a big-endian integer.
/// The word path is used when `out` is 4-aligned; otherwise it writes byte chunks.
#[inline(always)]
pub fn le_limbs_to_be_bytes(limbs: &[u64; 4], out: &mut [u8; 32]) {
    if out.as_ptr().addr().is_multiple_of(4) {
        let words = out.as_mut_ptr().cast::<u32>();
        for c in 0..4 {
            let limb = limbs[3 - c];
            // SAFETY: the destination is 4-aligned and contains eight writable words.
            unsafe {
                words.add(2 * c).write(bswap32((limb >> 32) as u32));
                words.add(2 * c + 1).write(bswap32(limb as u32));
            }
        }
    } else {
        for c in 0..4 {
            out[8 * c..8 * c + 8].copy_from_slice(&limbs[3 - c].to_be_bytes());
        }
    }
}

/// Reverses all bytes of `W` words in place by swapping and reversing words.
///
/// # Safety
/// `p` must be 4-aligned and point to `W` initialized, writable words.
// TODO: Investigate the correct approach to avoid warning here
#[allow(dead_code)]
#[inline(always)]
pub unsafe fn bytereverse_words_in_place<const W: usize>(p: *mut u32) {
    unsafe {
        for i in 0..W / 2 {
            let left = p.add(i).read();
            let right = p.add(W - 1 - i).read();
            p.add(i).write(bswap32(right));
            p.add(W - 1 - i).write(bswap32(left));
        }
        if !W.is_multiple_of(2) {
            let middle = p.add(W / 2);
            middle.write(bswap32(middle.read()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bswap_words() {
        assert_eq!(bswap32(0x1234_5678), 0x7856_3412);
    }

    /// A 32-byte-aligned scratch buffer, so that `offset` is also the misalignment.
    #[repr(C, align(32))]
    struct Scratch([u8; 64]);

    fn payloads() -> impl Iterator<Item = [u8; 32]> {
        let mut state = 0x1234_5678u32;
        let patterned = core::array::from_fn(|i| (i * 37 + 11) as u8);
        core::iter::once(patterned)
            .chain(core::iter::once([0xFF; 32]))
            .chain((0..8).map(move |_| {
                core::array::from_fn(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (state >> 24) as u8
                })
            }))
    }

    /// Checks `read_be_into_words::<N>` at every source misalignment against a byte loop. The
    /// source pointer comes from a reference to exactly its `N` bytes, so Miri rejects any read
    /// outside them; sentinels around the destination catch stray writes.
    fn check_read<const N: usize>() {
        let rounded = N.div_ceil(4) * 4;
        for payload in payloads() {
            for offset in 0..=3 {
                for fill in [0x00u8, 0xA5] {
                    let mut src = Scratch([0x5A; 64]);
                    src.0[offset..offset + N].copy_from_slice(&payload[..N]);
                    let source: &[u8] = &src.0[offset..offset + N];
                    assert_eq!(source.as_ptr().addr() % 4, offset);

                    let mut dst_words = [u32::from_ne_bytes([fill; 4]); 16];
                    // The helper may write only the first `rounded` bytes.
                    let window = dst_words[..rounded / 4].as_mut_ptr();
                    // SAFETY: the rounded range is within the destination window.
                    unsafe { window.cast::<u8>().add(N).write_bytes(0, rounded - N) };

                    // SAFETY: `source` has N readable bytes, the window is aligned and has
                    // room for rounded bytes, and they do not overlap.
                    unsafe { read_be_into_words::<N>(source.as_ptr(), window) };

                    let mut expected = [fill; 64];
                    expected[N..rounded].fill(0);
                    for i in 0..N {
                        expected[N - 1 - i] = payload[i];
                    }
                    // SAFETY: `[u32; 16]` and `[u8; 64]` have the same size, any bits are valid.
                    let actual = unsafe { core::mem::transmute::<[u32; 16], [u8; 64]>(dst_words) };
                    assert_eq!(
                        actual, expected,
                        "N = {N}, offset = {offset}, fill = {fill:#x}"
                    );
                    assert!(src.0[..offset].iter().all(|&b| b == 0x5A));
                    assert!(src.0[offset + N..].iter().all(|&b| b == 0x5A));
                }
            }
        }
    }

    /// Every `N % 4`, no full word, one full word, inner words, and the maximum
    #[test]
    fn read_be_into_words_alignment_and_sentinels() {
        check_read::<1>();
        check_read::<2>();
        check_read::<3>();
        check_read::<4>();
        check_read::<5>();
        check_read::<8>();
        check_read::<10>();
        check_read::<31>();
        check_read::<32>();
    }

    #[test]
    fn words_to_be_bytes_32_reverses_the_bytes() {
        for payload in payloads() {
            let mut src = [0u32; 8];
            // SAFETY: `src` is 32 writable bytes, disjoint from `payload`.
            unsafe {
                core::ptr::copy_nonoverlapping(payload.as_ptr(), src.as_mut_ptr().cast(), 32)
            };
            // SAFETY: `src` is aligned and contains eight words.
            let by_value = unsafe { words_to_be_bytes_32(src.as_ptr()) };
            let mut reversed = payload;
            reversed.reverse();
            assert_eq!(by_value, reversed);
        }
    }

    fn check_in_place<const W: usize>() {
        let mut words = core::array::from_fn::<_, W, _>(|i| {
            u32::from_ne_bytes(core::array::from_fn(|j| (i * 4 + j * 37 + 11) as u8))
        });
        let original = words;
        // SAFETY: words is W aligned, initialized, writable u32 values.
        unsafe { bytereverse_words_in_place::<W>(words.as_mut_ptr()) };
        let result = words.as_ptr().cast::<u8>();
        let source = original.as_ptr().cast::<u8>();
        for i in 0..W * 4 {
            // SAFETY: both byte indices are within their arrays.
            assert_eq!(unsafe { result.add(i).read() }, unsafe {
                source.add(W * 4 - 1 - i).read()
            });
        }
    }

    #[test]
    fn bytereverse_words_in_place_even_and_odd() {
        check_in_place::<1>();
        check_in_place::<2>();
        check_in_place::<7>();
        check_in_place::<8>();
    }

    /// Both limb helpers against a reference at every input and output misalignment
    #[test]
    fn limb_helpers_alignment_and_sentinels() {
        #[repr(align(8))]
        struct Scratch([u8; 48]);

        for payload in payloads() {
            let reference = core::array::from_fn::<_, 4, _>(|i| {
                u64::from_be_bytes(payload.as_chunks::<8>().0[3 - i])
            });
            for input_offset in 0..=3 {
                let mut src = Scratch([0xA5; 48]);
                src.0[4 + input_offset..36 + input_offset].copy_from_slice(&payload);
                let input: &[u8; 32] = src.0[4 + input_offset..36 + input_offset]
                    .try_into()
                    .unwrap();
                assert_eq!(be_bytes_to_le_limbs(input), reference);

                for output_offset in 0..=3 {
                    let mut dst = Scratch([0xA5; 48]);
                    let output: &mut [u8; 32] = (&mut dst.0[4 + output_offset..36 + output_offset])
                        .try_into()
                        .unwrap();
                    le_limbs_to_be_bytes(&reference, output);
                    let mut expected = [0xA5; 48];
                    expected[4 + output_offset..36 + output_offset].copy_from_slice(&payload);
                    assert_eq!(dst.0, expected);
                }
            }
        }
    }
}
