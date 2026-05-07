use airbender_host::{Inputs, Program, Result, Runner};
use std::path::PathBuf;

use airbender_crypto::k256::{
    ecdsa::{hazmat::bits2field, RecoveryId, Signature},
    elliptic_curve::ops::Reduce,
    Scalar, Secp256k1, U256,
};
use airbender_crypto::secp256k1::field::FieldElement;
use airbender_crypto::secp256k1::hooks::{DefaultSecp256k1Hooks, Secp256k1Hooks};
use airbender_crypto::secp256k1::scalars;
use airbender_crypto::secp256k1::recover_with_hooks;

fn main() -> Result<()> {
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;

    let digest: [u8; 32] = [
        56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112, 84, 246,
        106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62,
    ];
    let r = digest;
    let s: [u8; 32] = [
        120, 157, 29, 212, 35, 210, 95, 7, 114, 210, 116, 141, 96, 247, 228, 184, 27, 177, 77, 8,
        110, 186, 142, 142, 142, 251, 109, 207, 248, 164, 174, 2,
    ];
    let rec_id: u8 = 0;

    // --- Precompute hints ---
    //
    // The host runs recovery with CapturingHooks that record the result of
    // each expensive operation. These captured values become the hints that
    // the guest will verify cheaply.
    let signature = Signature::from_scalars(r, s).unwrap();
    let recovery_id = RecoveryId::try_from(rec_id).unwrap();
    let message =
        <Scalar as Reduce<U256>>::reduce_bytes(&bits2field::<Secp256k1>(&digest).unwrap());

    let mut capturing = CapturingHooks::default();
    let pk = recover_with_hooks(&message, &signature, &recovery_id, &mut capturing).unwrap();
    let compressed = pk.to_bytes();
    let expected_output =
        u32::from_be_bytes([compressed[1], compressed[2], compressed[3], compressed[4]]);

    let sqrt_hint = capturing.sqrt_result.expect("sqrt was called");
    let scalar_inv_hint = capturing.scalar_inv_result.expect("scalar_inv was called");
    let fe_inv_hint = capturing.fe_inv_result.expect("fe_inv was called");

    println!("Hints precomputed on host:");
    println!("  sqrt:       {}", hex::encode(sqrt_hint));
    println!("  scalar_inv: {}", hex::encode(scalar_inv_hint));
    println!("  fe_inv:     {}", hex::encode(fe_inv_hint));

    // --- Run guest with hints ---
    let mut inputs = Inputs::new();
    inputs.push(&digest)?;
    inputs.push(&r)?;
    inputs.push(&s)?;
    inputs.push(&rec_id)?;
    inputs.push(&sqrt_hint)?;
    inputs.push(&scalar_inv_hint)?;
    inputs.push(&fe_inv_hint)?;

    let runner = program.transpiler_runner().build()?;
    let execution = runner.run(inputs.words())?;
    let guest_output = execution.receipt.output[0];

    assert_eq!(guest_output, expected_output, "guest/host mismatch");
    println!(
        "\nGuest execution: cycles={}, output=0x{:08x}",
        execution.cycles_executed, guest_output
    );
    println!("Host verification: matches.");

    Ok(())
}

/// Hooks that compute operations normally but capture the results.
///
/// The host uses these to run recovery once and record the intermediate values
/// (sqrt, scalar inverse, field inverse). These become the hints passed to the
/// guest, where a cheaper hooks implementation just verifies them.
#[derive(Default)]
struct CapturingHooks {
    sqrt_result: Option<[u8; 32]>,
    scalar_inv_result: Option<[u8; 32]>,
    fe_inv_result: Option<[u8; 32]>,
}

impl Secp256k1Hooks for CapturingHooks {
    fn fe_sqrt_and_assign(&mut self, fe: &mut FieldElement) -> bool {
        let result = DefaultSecp256k1Hooks.fe_sqrt_and_assign(fe);
        self.sqrt_result = Some(fe.to_bytes().into());
        result
    }

    fn fe_invert_and_assign(&mut self, fe: &mut FieldElement) {
        DefaultSecp256k1Hooks.fe_invert_and_assign(fe);
        self.fe_inv_result = Some(fe.to_bytes().into());
    }

    fn scalar_invert_and_assign(&mut self, scalar: &mut scalars::Scalar) {
        DefaultSecp256k1Hooks.scalar_invert_and_assign(scalar);
        self.scalar_inv_result = Some(scalar.to_repr().into());
    }
}
