use crate::shared::WitnessInput;
use revm::{
    db::InMemoryDB,
    primitives::{
        alloy_primitives::B256, AccountInfo, Address, Bytecode, Bytes, ExecutionResult, SpecId,
        TxKind, U256,
    },
    Evm,
};

pub fn run_witness(input: &WitnessInput) -> Result<u64, String> {
    let contract_code = Bytecode::new_legacy(Bytes::from(contract_bytecode().to_vec()));
    let contract_code_hash = contract_code.hash_slow();

    let mut evm = Evm::builder()
        .with_db(InMemoryDB::default())
        .with_spec_id(SpecId::BERLIN)
        .modify_db(|db| {
            db.insert_account_info(
                Address::from(input.caller),
                AccountInfo::new(
                    U256::from(1_000_000_000u64),
                    0,
                    B256::ZERO,
                    Bytecode::default(),
                ),
            );
            db.insert_account_info(
                Address::from(input.tx_to),
                AccountInfo::new(U256::ZERO, 1, contract_code_hash, contract_code),
            );
        })
        .modify_tx_env(|tx| {
            tx.caller = Address::from(input.caller);
            tx.transact_to = TxKind::Call(Address::from(input.tx_to));
            tx.gas_limit = input.gas_limit;
            tx.value = U256::ZERO;
            tx.data = Bytes::new();
        })
        .build();

    let result = evm.transact().map_err(|err| format!("{err:?}"))?;
    match result.result {
        ExecutionResult::Success { gas_used, .. } => Ok(gas_used),
        ExecutionResult::Revert { output, gas_used } => Err(format!(
            "revm reverted after {gas_used} gas with {} bytes of output",
            output.len()
        )),
        ExecutionResult::Halt { reason, gas_used } => {
            Err(format!("revm halted after {gas_used} gas: {reason:?}"))
        }
    }
}

/// Minimal contract: stack-only arithmetic, no memory.
///
/// We intentionally avoid any memory operations (MSTORE, MLOAD, CALLDATACOPY,
/// RETURN-with-data) because revm's memory expansion gas math compiles to a
/// signed multiply instruction (`mulh`) on RISC-V, and the proving backend
/// rejects it. See the README for the full story.
///
/// Bytecode: PUSH1 1, PUSH1 2, ADD, POP, STOP
fn contract_bytecode() -> &'static [u8] {
    &[
        0x60, 0x01, // PUSH1 0x01
        0x60, 0x02, // PUSH1 0x02
        0x01, // ADD
        0x50, // POP
        0x00, // STOP
    ]
}
