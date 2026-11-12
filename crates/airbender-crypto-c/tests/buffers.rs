//! Buffer contracts, independent of the arithmetic implementation.
use airbender_crypto_c::*;

#[test]
fn modexp_exact_width_guards_and_overlap() {
    let mut output = [0xa5; 6];
    unsafe {
        assert_eq!(
            zkvm_modexp(
                [2].as_ptr(),
                1,
                [10].as_ptr(),
                1,
                [0, 0, 3, 232].as_ptr(),
                4,
                output.as_mut_ptr().add(1)
            ),
            EOK
        );
    }
    assert_eq!(output, [0xa5, 0, 0, 0, 24, 0xa5]);
    // Output overlaps the modulus, with guards on each side.
    let mut buffer = [0xa5, 0, 0, 3, 232, 0xa5];
    unsafe {
        assert_eq!(
            zkvm_modexp(
                [2].as_ptr(),
                1,
                [10].as_ptr(),
                1,
                buffer.as_ptr().add(1),
                4,
                buffer.as_mut_ptr().add(1)
            ),
            EOK
        );
    }
    assert_eq!(buffer, output);
}

/// EIP-7823: a declared length over 1024 bytes fails on the lengths alone, before the operands
/// are read (the pointers are to one byte), and leaves the output; 1024 is accepted
#[test]
fn modexp_rejects_lengths_over_the_eip_7823_bound() {
    let byte = [1u8];
    let mut output = [0xa5; 2];
    for (base_len, exp_len, mod_len) in [(1025, 1, 1), (1, 1025, 1), (1, 1, 1025)] {
        let status = unsafe {
            zkvm_modexp(
                byte.as_ptr(),
                base_len,
                byte.as_ptr(),
                exp_len,
                byte.as_ptr(),
                mod_len,
                output.as_mut_ptr(),
            )
        };
        assert_eq!((status, output), (EFAIL, [0xa5; 2]));
    }
    // 1024 everywhere: 2^1 mod (2^8184 + 1), the modulus being 1 || zeroes || 1
    let mut exponent = [0u8; 1024];
    exponent[1023] = 1;
    let mut modulus = [0u8; 1024];
    modulus[0] = 1;
    modulus[1023] = 1;
    let mut output = [0xa5u8; 1024];
    let mut two = [0u8; 1024];
    two[1023] = 2;
    let status = unsafe {
        zkvm_modexp(
            two.as_ptr(),
            1024,
            exponent.as_ptr(),
            1024,
            modulus.as_ptr(),
            1024,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, EOK);
    assert_eq!(output, two);
}

#[test]
fn hash_overlap_and_ripemd_padding() {
    let mut output = Bytes { data: [0xa5; 32] };
    unsafe {
        assert_eq!(zkvm_ripemd160(b"abc".as_ptr(), 3, &mut output), EOK);
        assert_eq!(&output.data[..12], &[0; 12]);
        let mut expected = Bytes { data: [0; 32] };
        assert_eq!(zkvm_sha256(b"abc".as_ptr(), 3, &mut expected), EOK);
        output.data[..3].copy_from_slice(b"abc");
        assert_eq!(zkvm_sha256(output.data.as_ptr(), 3, &mut output), EOK);
        assert_eq!(output, expected);
    }
}

#[test]
fn blake2f_invalid_flag_preserves_in_place_state() {
    let mut state = Bytes { data: [0xa5; 64] };
    let saved = state;
    unsafe {
        assert_eq!(
            zkvm_blake2f(
                12,
                &mut state,
                &Bytes { data: [0; 128] },
                &Bytes { data: [0; 16] },
                2
            ),
            EFAIL
        );
    }
    assert_eq!(state, saved);
}
