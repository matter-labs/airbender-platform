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
