//! The streaming hashers: their results and state handling.
use airbender_crypto_c::*;

fn keccak(data: &[u8]) -> [u8; 32] {
    let mut out = Bytes { data: [0; 32] };
    assert_eq!(
        unsafe { zkvm_keccak256(data.as_ptr(), data.len(), &mut out) },
        EOK
    );
    out.data
}

fn hex(text: &str) -> [u8; 32] {
    let mut out = [0; 32];
    for (byte, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    out
}

#[test]
fn streaming_keccak_matches_the_one_shot_digest() {
    let data: Vec<u8> = (0..300).map(|i| (i * 7 + 3) as u8).collect();
    let mut state = KeccakState {
        opaque: [0xa5; 512],
    };
    let mut out = Bytes { data: [0; 32] };
    for len in [0, 1, 3, 4, 135, 136, 137, 271, 272, 273, 300] {
        for split in [0, 1, 2, 5, 64, 135, 136, 137, len] {
            if split > len {
                continue;
            }
            unsafe {
                airbender_keccak256_init(&mut state);
                airbender_keccak256_update(&mut state, data.as_ptr(), split);
                airbender_keccak256_update(&mut state, data[split..].as_ptr(), len - split);
                airbender_keccak256_finalize(&mut state, &mut out);
            }
            assert_eq!(
                out.data,
                keccak(&data[..len]),
                "{len} bytes split at {split}"
            );
        }
    }
    unsafe {
        airbender_keccak256_init(&mut state);
        airbender_keccak256_update(&mut state, b"abc".as_ptr(), 3);
        airbender_keccak256_finalize(&mut state, &mut out);
    }
    assert_eq!(
        out.data,
        hex("4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45")
    );
}

#[test]
fn sha3_pads_differently_and_reset_states_are_reusable() {
    let mut state = KeccakState { opaque: [0; 512] };
    let mut out = Bytes { data: [0; 32] };
    unsafe {
        airbender_sha3_256_init(&mut state);
        airbender_sha3_256_finalize_reset(&mut state, &mut out);
        assert_eq!(
            out.data,
            hex("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a")
        );
        // reset: the next digest starts over
        airbender_sha3_256_update(&mut state, b"abc".as_ptr(), 3);
        airbender_sha3_256_finalize_reset(&mut state, &mut out);
        let abc = hex("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");
        assert_eq!(out.data, abc);
        airbender_sha3_256_update(&mut state, b"abc".as_ptr(), 3);
        airbender_sha3_256_finalize(&mut state, &mut out);
        assert_eq!(out.data, abc);
        // after finalize, init before reuse
        airbender_sha3_256_init(&mut state);
        airbender_sha3_256_update(&mut state, b"ab".as_ptr(), 2);
        airbender_sha3_256_update(&mut state, b"c".as_ptr(), 1);
        airbender_sha3_256_finalize(&mut state, &mut out);
        assert_eq!(out.data, abc);
        // the same for keccak
        airbender_keccak256_init(&mut state);
        airbender_keccak256_finalize_reset(&mut state, &mut out);
        assert_eq!(out.data, keccak(&[]));
        airbender_keccak256_update(&mut state, b"abc".as_ptr(), 3);
        airbender_keccak256_finalize_reset(&mut state, &mut out);
        assert_eq!(out.data, keccak(b"abc"));
    }
}
