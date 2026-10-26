//! C bindings of the signature operations.
use crate::{commit, crypto, read, with_active_oracle, Bytes};
#[cfg(any(not(target_arch = "riscv32"), feature = "delegation"))]
use crypto::hints::Secp256k1HooksWithOracle;
use crypto::secp256k1::hooks::DefaultSecp256k1Hooks;

#[no_mangle]
/// ECDSA verification over secp256k1, with the hinted hooks of the recovery (scalar inverse,
/// affine multiplication with hinted divisions) when there is an oracle.
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> i32 {
    use crypto::secp256k1::verify_prehash_with_hooks;
    commit(verified, || {
        let (msg, sig, pubkey) = (read(msg).data, read(sig).data, read(pubkey).data);
        hinted!(
            |oracle| verify_prehash_with_hooks(
                &msg,
                &sig,
                &pubkey,
                &mut Secp256k1HooksWithOracle::new(oracle)
            ),
            verify_prehash_with_hooks(&msg, &sig, &pubkey, &mut DefaultSecp256k1Hooks),
        )
    })
}

#[no_mangle]
/// ECDSA verification over P-256, without hints.
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> i32 {
    commit(verified, || {
        crypto::secp256r1::verify_prehash(&read(msg).data, &read(sig).data, &read(pubkey).data)
    })
}

#[no_mangle]
/// Public-key recovery for recovery IDs 0 to 3: the `y` parity, plus 2 for the `x = r + n` of the
/// signature point. 0 and 1 take zksync-os's ecrecover endpoint.
/// # Safety
/// See the crate-level contract.
pub unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    recid: u8,
    output: *mut Bytes<64>,
) -> i32 {
    commit(output, || {
        let msg = read(msg).data;
        let sig = read(sig).data;
        let ([r, s], []) = sig.as_chunks::<32>() else {
            unreachable!("64 bytes are r and s")
        };
        match recid {
            0 | 1 => with_active_oracle(|oracle| {
                crypto::precompiles::secp256k1_ecrecover(&msg, r, s, recid == 1, oracle)
            })
            .ok()
            .map(|key| Bytes { data: key.0 }),
            2 | 3 => recover_x_above_order(&msg, &sig, recid),
            _ => None,
        }
    })
}

/// The recovery of a signature whose point has `x = r + n` (`r` below `p - n`, about 2^-128 of
/// signatures), which the ecrecover of Ethereum does not take
fn recover_x_above_order(msg: &[u8; 32], sig: &[u8; 64], recid: u8) -> Option<Bytes<64>> {
    use crypto::k256::{
        ecdsa::{RecoveryId, Signature},
        elliptic_curve::ops::Reduce,
        Scalar, U256,
    };
    use crypto::secp256k1::recover_with_hooks;
    let signature = Signature::from_slice(sig).ok()?;
    let id = RecoveryId::try_from(recid).ok()?;
    let msg = <Scalar as Reduce<U256>>::reduce_bytes(&(*msg).into());
    let key = hinted!(
        |oracle| recover_with_hooks(
            &msg,
            &signature,
            &id,
            &mut Secp256k1HooksWithOracle::new(oracle)
        ),
        recover_with_hooks(&msg, &signature, &id, &mut DefaultSecp256k1Hooks),
    )
    .ok()?;
    let mut output = Bytes { data: [0; 64] };
    let ([x, y], []) = output.data.as_chunks_mut::<32>() else {
        unreachable!("64 bytes are x and y")
    };
    key.write_coordinates(x, y);
    Some(output)
}
