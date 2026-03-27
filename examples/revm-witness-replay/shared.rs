use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WitnessInput {
    pub caller: [u8; 20],
    pub tx_to: [u8; 20],
    pub gas_limit: u64,
}
