#[cfg(feature = "secp256k1-static-context")]
#[test]
fn signature_endpoints_match_reference() {
    use crate::{k256, p256};
    use k256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
    for n in 1..=12u8 {
        let digest = [n.wrapping_mul(19); 32];
        let k = k256::ecdsa::SigningKey::from_bytes(&[n; 32].into()).unwrap();
        let p = p256::ecdsa::SigningKey::from_bytes(&[n; 32].into()).unwrap();
        let sig: k256::ecdsa::Signature = k.sign_prehash(&digest).unwrap();
        let psig: p256::ecdsa::Signature = p.sign_prehash(&digest).unwrap();
        let key: [u8; 64] = k.verifying_key().to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .unwrap();
        let pkey: [u8; 64] = p.verifying_key().to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .unwrap();
        for message in [digest, [0; 32]] {
            let expected = k.verifying_key().verify_prehash(&message, &sig).is_ok();
            let pexpected = p.verifying_key().verify_prehash(&message, &psig).is_ok();
            assert_eq!(
                crate::secp256k1::verify_prehash(&message, &sig.to_bytes().into(), &key),
                Some(expected)
            );
            assert_eq!(
                crate::secp256r1::verify_prehash(&message, &psig.to_bytes().into(), &pkey),
                Some(pexpected)
            );
        }
    }
}
