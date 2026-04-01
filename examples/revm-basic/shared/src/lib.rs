use revm::{
    bytecode::Bytecode,
    context::TxEnv,
    context_interface::result::ExecutionResult,
    database::CacheDB,
    database_interface::EmptyDB,
    primitives::{Address, Bytes, TxKind, U256},
    state::AccountInfo,
    Context, ExecuteEvm, MainBuilder, MainContext,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunInput {
    pub caller: [u8; 20],
    pub tx_to: [u8; 20],
    pub gas_limit: u64,
}

pub fn run(input: &RunInput) -> Result<u64, String> {
    let caller = Address::from(input.caller);
    let contract_addr = Address::from(input.tx_to);

    let contract_code = Bytecode::new_legacy(Bytes::from(contract_bytecode().to_vec()));

    let mut db = CacheDB::<EmptyDB>::default();
    db.insert_account_info(
        caller,
        AccountInfo {
            balance: U256::from(1_000_000_000u64),
            ..Default::default()
        },
    );
    db.insert_account_info(
        contract_addr,
        AccountInfo {
            nonce: 1,
            code: Some(contract_code),
            ..Default::default()
        },
    );

    let ctx = Context::mainnet().with_db(db);
    let mut evm = ctx.build_mainnet();

    let tx = TxEnv::builder()
        .caller(caller)
        .kind(TxKind::Call(contract_addr))
        .gas_limit(input.gas_limit)
        .value(U256::ZERO)
        .data(Bytes::new())
        .build()
        .map_err(|e| format!("{e:?}"))?;

    let result = evm.transact(tx).map_err(|e| format!("{e:?}"))?;
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

/// Stores 0x42 to memory and returns it (MSTORE + RETURN).
fn contract_bytecode() -> &'static [u8] {
    &[
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x52, // MSTORE
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0xf3, // RETURN
    ]
}
