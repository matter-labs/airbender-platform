#[cfg(not(target_endian = "little"))]
compile_error!("invalid arch - only intended for LE machines");

#[cfg(all(target_arch = "riscv32", feature = "keccak_special5"))]
mod precompile;
#[cfg(all(target_arch = "riscv32", feature = "keccak_special5"))]
pub(crate) use self::precompile::keccak_f1600;

#[cfg(any(
    not(all(target_arch = "riscv32", feature = "keccak_special5")),
    feature = "testing",
))]
mod precompile_logic_simulator;
#[cfg(any(
    not(all(target_arch = "riscv32", feature = "keccak_special5")),
    feature = "testing",
))]
pub(crate) use self::precompile_logic_simulator::keccak_f1600;

use crate::MiniDigest;

use common_constants::delegation_types::keccak_special5::{
    KeccakF1600State, KECCAK_SPECIAL5_STATE_AND_SCRATCH_U64_WORDS,
};

// Use Airbender's ABI type directly so the digest state layout stays coupled to
// the delegation circuit contract exposed by `common_constants`.
pub(crate) type AlignedState = KeccakF1600State;

// NOTE: Sha3 and Keccak differ only in padding, so we can make it generic for free,
// whether we will need it in practice or not. We also do not use a separate buffer for input,
// and instead XOR input directly into the state

const BUFFER_SIZE_U64_WORDS: usize = 17;
const BUFFER_SIZE_U32_WORDS: usize = BUFFER_SIZE_U64_WORDS * 2;
const BUFFER_SIZE_BYTES: usize = 17 * core::mem::size_of::<u64>();

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Keccak256Core<const SHA3: bool = false> {
    state: AlignedState,
    filled_bytes: usize,
}

#[allow(dead_code)]
pub type Keccak256 = Keccak256Core<false>;
#[allow(dead_code)]
pub type Sha3_256 = Keccak256Core<true>;

// Absorbing is XOR of the input into the rate part of the state, and it is organized the way
// `memcpy` of `riscv_common` is: there are no unaligned loads of words on our machine, so we
// consume bytes until the *source* is word-aligned, and then go by words of the source.
// A word of the source lands either exactly on a word of the state, or (if the absorbed length
// is not a multiple of 4 at that moment) on two neighbours, shifted. The rest (less than a word)
// goes by bytes again.

const WORD_SIZE: usize = core::mem::size_of::<u32>();

/// `dst[i] ^= src[i]`, unrolled
///
/// # Safety
/// `dst` and `src` should be valid for `len` words
#[inline(always)]
unsafe fn xor_words(mut dst: *mut u32, mut src: *const u32, mut len: usize) {
    while len >= 4 {
        seq_macro::seq!(N in 0..4 {
            dst.add(N).write(dst.add(N).read() ^ src.add(N).read());
        });
        dst = dst.add(4);
        src = src.add(4);
        len -= 4;
    }
    if len & 2 > 0 {
        seq_macro::seq!(N in 0..2 {
            dst.add(N).write(dst.add(N).read() ^ src.add(N).read());
        });
        dst = dst.add(2);
        src = src.add(2);
    }
    if len & 1 > 0 {
        dst.write(dst.read() ^ src.read());
    }
}

/// XORs `len` words of `src` into `len + 1` words of `dst`, shifted by `SHIFT` bits up, unrolled
///
/// # Safety
/// `dst` should be valid for `len + 1` words, and `src` for `len` words
#[inline(always)]
unsafe fn xor_words_shifted<const SHIFT: u32>(
    mut dst: *mut u32,
    mut src: *const u32,
    mut len: usize,
) {
    let mut carry = 0u32;
    while len >= 2 {
        let word_0 = src.read();
        let word_1 = src.add(1).read();
        dst.write(dst.read() ^ (word_0 << SHIFT | carry));
        dst.add(1)
            .write(dst.add(1).read() ^ (word_1 << SHIFT | word_0 >> (u32::BITS - SHIFT)));
        carry = word_1 >> (u32::BITS - SHIFT);
        dst = dst.add(2);
        src = src.add(2);
        len -= 2;
    }
    if len > 0 {
        let word = src.read();
        dst.write(dst.read() ^ (word << SHIFT | carry));
        carry = word >> (u32::BITS - SHIFT);
        dst = dst.add(1);
    }
    // there is always a place for it: the buffer is a whole number of words
    dst.write(dst.read() ^ carry);
}

impl<const SHA3: bool> Keccak256Core<SHA3> {
    /// Same as `MiniDigest::new`, but usable for statics
    pub const fn const_new() -> Self {
        Self {
            state: AlignedState::zeroed(),
            filled_bytes: 0,
        }
    }

    #[inline(always)]
    fn state_words(&mut self) -> *mut u32 {
        self.state.0.as_mut_ptr().cast::<u32>()
    }

    #[inline(always)]
    unsafe fn permute_if_full(&mut self) {
        if self.filled_bytes == BUFFER_SIZE_BYTES {
            self.filled_bytes = 0;
            keccak_f1600(&mut self.state);
        }
    }

    #[inline(always)]
    unsafe fn absorb_byte(&mut self, byte: u8) {
        debug_assert!(self.filled_bytes < BUFFER_SIZE_BYTES);
        let dst = self.state_words().add(self.filled_bytes / WORD_SIZE);
        // the machine is little-endian (see the top of the file)
        dst.write(dst.read() ^ ((byte as u32) << (8 * (self.filled_bytes % WORD_SIZE))));
        self.filled_bytes += 1;
        self.permute_if_full();
    }

    /// # Safety
    /// `src` should be aligned and valid for `len` words
    #[inline(always)]
    unsafe fn absorb_words(&mut self, mut src: *const u32, mut len: usize) {
        debug_assert!(len == 0 || src.is_aligned());

        while len > 0 {
            debug_assert!(self.filled_bytes < BUFFER_SIZE_BYTES);
            let byte_in_word = self.filled_bytes % WORD_SIZE;
            let mut dst = self.state_words().add(self.filled_bytes / WORD_SIZE);
            // whole words of the source that fit into the buffer
            let to_absorb =
                core::cmp::min(len, (BUFFER_SIZE_BYTES - self.filled_bytes) / WORD_SIZE);
            len -= to_absorb;
            self.filled_bytes += to_absorb * WORD_SIZE;

            if byte_in_word == 0 {
                xor_words(dst, src, to_absorb);
                src = src.add(to_absorb);
                self.permute_if_full();
            } else {
                // a word of the source covers the upper bytes of one word of the state,
                // and the lower bytes of the next one
                let shift = 8 * byte_in_word as u32;
                match byte_in_word {
                    1 => xor_words_shifted::<8>(dst, src, to_absorb),
                    2 => xor_words_shifted::<16>(dst, src, to_absorb),
                    3 => xor_words_shifted::<24>(dst, src, to_absorb),
                    _ => core::hint::unreachable_unchecked(),
                }
                src = src.add(to_absorb);
                dst = dst.add(to_absorb);

                if len > 0 && BUFFER_SIZE_BYTES - self.filled_bytes < WORD_SIZE {
                    // and this one covers the end of the buffer, and the beginning of the next one
                    debug_assert_eq!(dst, self.state_words().add(BUFFER_SIZE_U32_WORDS - 1));
                    let word = src.read();
                    dst.write(dst.read() ^ (word << shift));
                    keccak_f1600(&mut self.state);
                    let dst = self.state_words();
                    dst.write(dst.read() ^ (word >> (u32::BITS - shift)));
                    self.filled_bytes = byte_in_word;
                    src = src.add(1);
                    len -= 1;
                }
            }
        }
    }
}

impl<const SHA3: bool> MiniDigest for Keccak256Core<SHA3> {
    type HashOutput = crate::Bytes32;

    #[inline(always)]
    fn new() -> Self {
        Self {
            state: AlignedState::zeroed(),
            filled_bytes: 0,
        }
    }

    // #[inline(always)]
    #[inline(never)]
    fn update(&mut self, input: impl AsRef<[u8]>) {
        let mut input = input.as_ref();

        if input.is_empty() {
            return;
        }

        unsafe {
            // align the source
            while let Some((byte, rest)) = input.split_first() {
                if input.as_ptr().cast::<u32>().is_aligned() {
                    break;
                }
                self.absorb_byte(*byte);
                input = rest;
            }

            let (words, tail) = input.as_chunks::<WORD_SIZE>();
            self.absorb_words(words.as_ptr().cast::<u32>(), words.len());

            for byte in tail.iter() {
                self.absorb_byte(*byte);
            }

            debug_assert_ne!(self.filled_bytes, BUFFER_SIZE_BYTES);
        };
    }

    #[inline(always)]
    fn finalize(mut self) -> Self::HashOutput {
        keccak_pad::<SHA3>(self.state.as_words_mut(), self.filled_bytes);
        keccak_f1600(&mut self.state);
        // state is overaligned, and the output is word aligned: copied by words
        unsafe { self.state.0.as_ptr().cast::<crate::Bytes32>().read() }
    }

    #[inline(always)]
    fn finalize_reset(&mut self) -> Self::HashOutput {
        keccak_pad::<SHA3>(self.state.as_words_mut(), self.filled_bytes);
        keccak_f1600(&mut self.state);
        let output = unsafe { self.state.0.as_ptr().cast::<crate::Bytes32>().read() };
        self.reset_state();

        output
    }

    #[inline(always)]
    fn digest(input: impl AsRef<[u8]>) -> Self::HashOutput {
        let mut hasher = Self::new();
        hasher.update(input);
        hasher.finalize()
    }

    #[inline(always)]
    fn finalize_reset_with_closure<FN: FnOnce(&Self::HashOutput) -> ()>(&mut self, closure: FN) {
        keccak_pad::<SHA3>(self.state.as_words_mut(), self.filled_bytes);
        keccak_f1600(&mut self.state);
        let output = unsafe {
            self.state
                .0
                .as_ptr()
                .cast::<crate::Bytes32>()
                .as_ref_unchecked()
        };
        (closure)(output);
        self.reset_state();
    }
}

#[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
static ZERO_WORD: crate::BigInt<4> = crate::BigInt::<4>::zero();

impl<const SHA3: bool> Keccak256Core<SHA3> {
    #[cfg(all(target_arch = "riscv32", feature = "bigint_ops"))]
    #[inline(always)]
    fn reset_state(&mut self) {
        // We know that our state is overaligned, so we can do memcpy via precompile.
        // In total we can zero out all padded buffer - 32xu64 words, meaning 8 precompile calls

        unsafe {
            let mut ptr = self.state.0.as_mut_ptr();
            let src = core::ptr::addr_of!(ZERO_WORD).cast();
            seq_macro::seq!(N in 0..8 {
                let _ = crate::bigint_op_delegation_raw(ptr.cast(), src, crate::BigIntOps::MemCpy);
                ptr = ptr.add(4); // 4xu64 = 32 bytes
            });
        }
        self.filled_bytes = 0;
    }

    /// Zero the state with inline stores: a plain loop (or `fill`) is recognized as `memset`
    /// and becomes a call, that costs more than the stores themselves
    #[cfg(not(all(target_arch = "riscv32", feature = "bigint_ops")))]
    #[inline(always)]
    fn reset_state(&mut self) {
        for dst in self.state.0.iter_mut() {
            unsafe {
                core::ptr::write_volatile(dst, 0);
            }
        }
        self.filled_bytes = 0;
    }
}

#[allow(dead_code)]
#[inline(always)]
fn keccak_pad<const SHA3: bool>(
    state: &mut [u64; KECCAK_SPECIAL5_STATE_AND_SCRATCH_U64_WORDS],
    len_filled_bytes: usize,
) {
    // 32-bit words: a 64-bit shift by a variable amount is expensive on 32-bit targets.
    // Lanes are little-endian, so the high half of lane `i` is word `2 * i + 1`
    const NUM_WORDS: usize = 2 * KECCAK_SPECIAL5_STATE_AND_SCRATCH_U64_WORDS;
    let words: &mut [u32; NUM_WORDS] =
        unsafe { &mut *state.as_mut_ptr().cast::<[u32; NUM_WORDS]>() };
    let padding_start = (if SHA3 { 0x06u32 } else { 0x01u32 }) << ((len_filled_bytes % 4) * 8);
    words[len_filled_bytes / 4] ^= padding_start;
    words[2 * 16 + 1] ^= 0x80000000; // last bit is always there
}

#[cfg(any(test, feature = "sha3_tests"))]
pub mod tests {
    #[test]
    fn keccak_f1600() {
        keccak_f1600_test();
    }

    #[test]
    #[should_panic]
    fn bad_keccak_f1600() {
        bad_keccak_f1600_test();
    }

    #[test]
    fn mini_digest() {
        mini_digest_test();
    }

    #[test]
    fn hash_chain() {
        hash_chain_test();
    }

    #[allow(dead_code)]
    pub fn bad_keccak_f1600_test() {
        let state_first = [
            0xF1258F7940E1DDE7,
            0x84D5CCF933C0478A,
            0xD598261EA65AA9EE,
            0xBD1547306F80494D,
            0x8B284E056253D057,
            0xFF97A42D7F8E6FD4,
            0x90FEE5A0A44647C4,
            0x8C5BDA0CD6192E76,
            0xAD30A6F71B19059C,
            0x30935AB7D08FFC64,
            0xEB5AA93F2317D635,
            0xA9A6E6260D712103,
            0x81A57C16DBCF555F,
            0x43B831CD0347C826,
            0x01F22F1A11A5569F,
            0x05E5635A21D9AE61,
            0x64BEFEF28CC970F2,
            0x613670957BC46611,
            0xB87C5A554FD00ECB,
            0x8C3EE88A1CCF32C8,
            0x940C7922AE3A2614,
            0x1841F924A2C509E4,
            0x16F53526E70465C2,
            0x75F644E97F30A13B,
            0xEAF1FF7B5CECA249,
        ];
        let state_second = [
            0x2D5C954DF96ECB3C,
            0x6A332CD07057B56D,
            0x093D8D1270D76B6C,
            0x8A20D9B25569D094,
            0x4F9C4F99E5E7F156,
            0xF957B9A2DA65FB38,
            0x85773DAE1275AF0D,
            0xFAF4F247C3D810F7,
            0x1F1B9EE6F79A8759,
            0xE4FECC0FEE98B425,
            0x68CE61B6B9CE68A1,
            0xDEEA66C4BA8F974F,
            0x33C43D836EAFB1F5,
            0xE00654042719DBD9,
            0x7CF8A9F009831265,
            0xFD5449A6BF174743,
            0x97DDAD33D8994B40,
            0x48EAD5FC5D0BE774,
            0xE3B8C8EE55B7B03C,
            0x91A0226E649E42E9,
            0x900E3129E7BADD7B,
            0x202A9EC5FAA3CCE8,
            0x5B3402464E1C3DB6,
            0x609F4E62A44C1059,
            0x1, //0x20D06CD26A8FBF5C,
        ];

        let mut state = super::AlignedState::zeroed();
        state.0[..25].copy_from_slice(&state_first);
        super::keccak_f1600(&mut state);
        assert!(state.0[..25] == state_second);
    }

    #[allow(dead_code)]
    pub fn keccak_f1600_test() {
        let state_first = [
            0xF1258F7940E1DDE7,
            0x84D5CCF933C0478A,
            0xD598261EA65AA9EE,
            0xBD1547306F80494D,
            0x8B284E056253D057,
            0xFF97A42D7F8E6FD4,
            0x90FEE5A0A44647C4,
            0x8C5BDA0CD6192E76,
            0xAD30A6F71B19059C,
            0x30935AB7D08FFC64,
            0xEB5AA93F2317D635,
            0xA9A6E6260D712103,
            0x81A57C16DBCF555F,
            0x43B831CD0347C826,
            0x01F22F1A11A5569F,
            0x05E5635A21D9AE61,
            0x64BEFEF28CC970F2,
            0x613670957BC46611,
            0xB87C5A554FD00ECB,
            0x8C3EE88A1CCF32C8,
            0x940C7922AE3A2614,
            0x1841F924A2C509E4,
            0x16F53526E70465C2,
            0x75F644E97F30A13B,
            0xEAF1FF7B5CECA249,
        ];
        let state_second = [
            0x2D5C954DF96ECB3C,
            0x6A332CD07057B56D,
            0x093D8D1270D76B6C,
            0x8A20D9B25569D094,
            0x4F9C4F99E5E7F156,
            0xF957B9A2DA65FB38,
            0x85773DAE1275AF0D,
            0xFAF4F247C3D810F7,
            0x1F1B9EE6F79A8759,
            0xE4FECC0FEE98B425,
            0x68CE61B6B9CE68A1,
            0xDEEA66C4BA8F974F,
            0x33C43D836EAFB1F5,
            0xE00654042719DBD9,
            0x7CF8A9F009831265,
            0xFD5449A6BF174743,
            0x97DDAD33D8994B40,
            0x48EAD5FC5D0BE774,
            0xE3B8C8EE55B7B03C,
            0x91A0226E649E42E9,
            0x900E3129E7BADD7B,
            0x202A9EC5FAA3CCE8,
            0x5B3402464E1C3DB6,
            0x609F4E62A44C1059,
            0x20D06CD26A8FBF5C,
        ];

        let mut state = super::AlignedState::zeroed();
        state.0[..25].copy_from_slice(&state_first);
        super::keccak_f1600(&mut state);
        assert!(state.0[..25] == state_second);
    }

    #[allow(dead_code)]
    pub fn mini_digest_test() {
        use super::*;
        use ark_std::rand::Rng;
        let mut rng = ark_std::test_rng();
        let mut formal_keccak256 = <sha3::Keccak256 as sha3::Digest>::new();
        let mut formal_sha3 = <sha3::Sha3_256 as sha3::Digest>::new();
        let mut my_keccak256 = Keccak256::new();
        let mut my_sha3 = Sha3_256::new();
        let mut msg = [0; u8::MAX as usize];

        for _try in 0..1 << 10 {
            let num_chunks = rng.r#gen::<u8>();
            for _chunk in 0..num_chunks {
                let len = rng.r#gen::<u8>() as usize;
                for byte in msg.iter_mut().take(len) {
                    *byte = rng.r#gen::<u8>();
                }
                // inputs at all the alignments
                let start = core::cmp::min(rng.r#gen::<u8>() as usize % 8, len);
                sha3::Digest::update(&mut formal_keccak256, &msg[start..len]);
                sha3::Digest::update(&mut formal_sha3, &msg[start..len]);
                my_keccak256.update(&msg[start..len]);
                my_sha3.update(&msg[start..len]);
            }
            assert!(
                sha3::Digest::finalize_reset(&mut formal_keccak256)[..]
                    == my_keccak256.finalize_reset()
            );
            assert!(sha3::Digest::finalize_reset(&mut formal_sha3)[..] == my_sha3.finalize_reset());
        }
    }

    #[allow(dead_code)]
    pub fn hash_chain_test() {
        use super::*;
        let mut state = AlignedState::zeroed();
        for _ in 0..2000 {
            super::keccak_f1600(&mut state);
        }
        core::hint::black_box(state);
    }
}
