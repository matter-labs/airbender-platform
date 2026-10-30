//! The signature exports: status codes, raw x || y keys, recovery IDs and the recorded hints.
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::call;

#[test]
fn secp256k1_verify_and_recover() {
    use crypto::hints::{record_hints, Secp256k1HooksWithOracle};
    use crypto::k256::ecdsa::SigningKey;
    let key = SigningKey::from_bytes(&[7u8; 32].into()).unwrap();
    let msg = Bytes { data: [0xff; 32] };
    let (sig, id) = key.sign_prehash_recoverable(&msg.data).unwrap();
    let sig = Bytes {
        data: sig.to_bytes().into(),
    };
    // x || y, without SEC1's 0x04 prefix
    let pk = Bytes {
        data: key.verifying_key().to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .unwrap(),
    };
    let verify = |initial: bool, msg: &Bytes<32>, sig: &Bytes<64>, pk: &Bytes<64>| {
        call(initial, |out| unsafe {
            zkvm_secp256k1_verify(msg, sig, pk, out)
        })
    };
    let ((status, verified), tape) = verify(false, &msg, &sig, &pk);
    assert_eq!((status, verified, tape.is_empty()), (EOK, true, false));
    let (_, hinted) = record_hints(|o| {
        let hooks = &mut Secp256k1HooksWithOracle::new(o);
        crypto::secp256k1::verify_prehash_with_hooks(&msg.data, &sig.data, &pk.data, hooks)
    });
    assert_eq!(tape, hinted);
    let mut wrong = msg;
    wrong.data[0] ^= 1;
    assert_eq!(verify(true, &wrong, &sig, &pk).0, (EOK, false));

    let unchanged = Bytes { data: [0xa5; 64] };
    let recover = |sig: &Bytes<64>, recid: u8| {
        call(unchanged, |out| unsafe {
            zkvm_secp256k1_ecrecover(&msg, sig, recid, out)
        })
    };
    let ((status, recovered), tape) = recover(&sig, id.to_byte());
    assert_eq!((status, recovered, tape.is_empty()), (EOK, pk, false));

    // malformed inputs fail before any hint, with the output unchanged
    let zero = Bytes { data: [0; 64] };
    let mut off_curve = pk;
    off_curve.data[63] ^= 1;
    assert_eq!(verify(true, &msg, &zero, &pk), ((EFAIL, true), vec![]));
    assert_eq!(
        verify(true, &msg, &sig, &off_curve),
        ((EFAIL, true), vec![])
    );
    assert_eq!(recover(&zero, 0), ((EFAIL, unchanged), vec![]));
    // IDs 2 and 3 need x = r + n below p, which this r is not; there is no ID above 3
    for recid in [2, 3, 4] {
        assert_eq!(recover(&sig, recid).0, (EFAIL, unchanged));
    }
}

#[test]
fn p256_valid_false_and_malformed() {
    unsafe {
        use crypto::p256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};
        let key = SigningKey::from_bytes(&[11u8; 32].into()).unwrap();
        let mut msg = Bytes { data: [13; 32] };
        let sig: Signature = key.sign_prehash(&msg.data).unwrap();
        let sig = Bytes {
            data: sig.to_bytes().into(),
        };
        let pk = Bytes {
            data: key.verifying_key().to_encoded_point(false).as_bytes()[1..]
                .try_into()
                .unwrap(),
        };
        let mut verified = false;
        assert_eq!(zkvm_secp256r1_verify(&msg, &sig, &pk, &mut verified), EOK);
        assert!(verified);
        msg.data[0] ^= 1;
        assert_eq!(zkvm_secp256r1_verify(&msg, &sig, &pk, &mut verified), EOK);
        assert!(!verified);
        verified = true;
        assert_eq!(
            zkvm_secp256r1_verify(&msg, &Bytes { data: [0; 64] }, &pk, &mut verified),
            EFAIL
        );
        assert!(verified);
    }
}
