//! The oracle of a guest that takes hints: a `QueryDispatcher` with airbender-crypto's responder
//! for its field operations and ModExp advice (`CryptoHints`), serving a native run of the same
//! computation as its `MemoryOracle`, and the RISC-V guest as its non-determinism CSR source.
use airbender_crypto::hints::{CryptoHints, HintError, NoHints};
use airbender_crypto::modexp::modexp;
use airbender_crypto::precompiles::secp256k1_ecrecover;
use airbender_host::{Inputs, Program, Result};
use airbender_oracle::processor::{NonDeterminismCSRSource, QueryDispatcher, RamPeek, RunMode};
use std::collections::VecDeque;
use std::path::PathBuf;

/// The oracle for a run in `mode`, with the responder for the hints: the field operations
/// (inverses, square roots, quotients, pairing witnesses) and the ModExp advice.
fn hint_oracle(mode: RunMode) -> QueryDispatcher<HintError> {
    let mut oracle = QueryDispatcher::new(mode);
    oracle.add_external_processor(CryptoHints);
    oracle
}

/// The CSR source of the guest: its input words, then the oracle. The guest reads its inputs
/// before its first query, and after a query it reads the words of the answer.
struct GuestSource {
    inputs: VecDeque<u32>,
    oracle: QueryDispatcher<HintError>,
}

impl NonDeterminismCSRSource for GuestSource {
    fn read(&mut self) -> u32 {
        if self.oracle.has_pending_response() {
            return self.oracle.read();
        }
        match self.inputs.pop_front() {
            Some(word) => word,
            None => self.oracle.read(),
        }
    }

    fn write_with_memory_access<R: RamPeek + ?Sized>(&mut self, ram: &R, value: u32) {
        self.oracle.write_with_memory_access(ram, value);
    }

    fn write_with_memory_access_raw(&mut self, ram: &[u32], value: u32) {
        self.oracle.write_with_memory_access_raw(ram, value);
    }

    fn write_with_memory_access_dyn(&mut self, ram: &dyn RamPeek, value: u32) {
        self.oracle.write_with_memory_access_dyn(ram, value);
    }
}

/// 32 bytes from a hex literal
macro_rules! hex {
    ($literal:literal) => {{
        let bytes: [u8; 32] = hex::decode($literal)
            .expect("hex")
            .try_into()
            .expect("32 bytes");
        bytes
    }};
}

fn main() -> Result<()> {
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../guest/dist/app");
    let program = Program::load(&dist_dir)?;

    // ModExp: base^exp mod modulus, 32 bytes each
    let base = hex!("0000000000000000000000000000000000000000000000000000000000c0ffee");
    let exp = hex!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    let modulus = hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    // ecrecover: the digest, r, s and y parity of a signature
    let digest = hex!("38d18acb67d25c8bb9942764b62f18e17054f66a817bd4295423adf9ed98873e");
    let r = digest;
    let s = hex!("789d1dd423d25f0772d2748d60f7e4b81bb14d086eba8e8e8efb6dcff8a4ae02");
    let y_is_odd = false;

    // --- A native run, with the oracle as its MemoryOracle ---
    //
    // The same hinted computation in this process: the dispatcher answers each query from the
    // request the computation points to, and the computation checks every answer.
    let mut oracle = hint_oracle(RunMode::NativeRunOnly);
    let power = modexp(&base, &exp, &modulus, Some(&mut oracle));
    let key = secp256k1_ecrecover(&digest, &r, &s, y_is_odd, Some(&mut oracle))
        .expect("the signature must recover");
    let expected = fold(&power, &key.0);

    // the software paths, without hints, agree
    let plain_power = modexp(&base, &exp, &modulus, None::<&mut NoHints>);
    let plain_key = secp256k1_ecrecover(&digest, &r, &s, y_is_odd, None::<&mut NoHints>)
        .expect("the signature must recover");
    assert_eq!(fold(&plain_power, &plain_key.0), expected, "hinted/plain mismatch");
    println!("Native run with hints:");
    println!("  power: {}", hex::encode(&*power));
    println!("  key:   {}", hex::encode(key.0));

    // --- The guest run, with the oracle as its non-determinism CSR source ---
    //
    // The guest sends the same queries, with the addresses of its requests in its own memory,
    // which the dispatcher reads through the simulator while the guest waits.
    let mut inputs = Inputs::new();
    inputs.push(&base)?;
    inputs.push(&exp)?;
    inputs.push(&modulus)?;
    inputs.push(&digest)?;
    inputs.push(&r)?;
    inputs.push(&s)?;
    inputs.push(&y_is_odd)?;

    let runner = program.transpiler_runner().build()?;
    let oracle = QueryDispatcher::new(RunMode::RiscVRun);
    oracle.add_external_processor(CryptoHints);
    let source = GuestSource {
        inputs: inputs.words().iter().copied().collect(),
        oracle,
    };
    let execution = runner.run_with_source(source)?;
    let guest_output = execution.receipt.output[0];

    assert_eq!(guest_output, expected, "guest/host mismatch");
    println!(
        "\nGuest execution: cycles={}, output=0x{:08x}",
        execution.cycles_executed, guest_output
    );
    println!("Host verification: matches.");

    Ok(())
}

/// A word of both results, as the guest computes it: FNV-1a over the power as 32 big-endian
/// bytes, then the key
fn fold(power: &[u8], key: &[u8; 64]) -> u32 {
    let mut padded = [0u8; 32];
    padded[32 - power.len()..].copy_from_slice(power);
    let mut digest = 0x811c_9dc5u32;
    for byte in padded.iter().chain(key) {
        digest ^= u32::from(*byte);
        digest = digest.wrapping_mul(16_777_619);
    }
    digest
}

