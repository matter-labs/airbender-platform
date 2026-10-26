use airbender_crypto as crypto;
use airbender_crypto_c::*;

#[test]
fn secp_status_and_raw_public_key_recovery() {
    unsafe {
        use crypto::k256::ecdsa::SigningKey;
        let key = SigningKey::from_bytes(&[7u8; 32].into()).unwrap();
        let msg = Bytes { data: [0xff; 32] };
        let (sig, id) = key.sign_prehash_recoverable(&msg.data).unwrap();
        let encoded = key.verifying_key().to_encoded_point(false);
        let pk = Bytes {
            data: encoded.as_bytes()[1..].try_into().unwrap(),
        };
        let sig = Bytes {
            data: sig.to_bytes().into(),
        };
        let mut verified = false;
        assert_eq!(zkvm_secp256k1_verify(&msg, &sig, &pk, &mut verified), EOK);
        assert!(verified);
        let mut recovered = Bytes { data: [9; 64] };
        assert_eq!(
            zkvm_secp256k1_ecrecover(&msg, &sig, id.to_byte(), &mut recovered),
            EOK
        );
        assert_eq!(recovered, pk);
        let mut wrong = msg;
        wrong.data[0] ^= 1;
        assert_eq!(zkvm_secp256k1_verify(&wrong, &sig, &pk, &mut verified), EOK);
        assert!(!verified);
        assert_eq!(
            zkvm_secp256k1_ecrecover(&msg, &sig, 4, &mut recovered),
            EFAIL
        );
        assert_eq!(recovered, pk);
        let zero = Bytes { data: [0; 64] };
        verified = true;
        assert_eq!(
            zkvm_secp256k1_verify(&msg, &zero, &pk, &mut verified),
            EFAIL
        );
        assert!(verified);
        assert_eq!(
            zkvm_secp256k1_ecrecover(&msg, &zero, 0, &mut recovered),
            EFAIL
        );
        assert_eq!(recovered, pk);
        assert_eq!(
            zkvm_secp256k1_verify(&msg, &sig, &zero, &mut verified),
            EFAIL
        );
        assert!(verified);
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
