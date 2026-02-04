/// Execution output captured from simulator or prover results.
#[derive(Clone, Debug)]
pub struct Receipt {
    pub registers: [u32; 32],
    pub output: [u32; 8],
    pub output_extended: [u32; 16],
}

impl Receipt {
    pub fn from_registers(registers: [u32; 32]) -> Self {
        let mut output = [0u32; 8];
        output.copy_from_slice(&registers[10..18]);
        let mut output_extended = [0u32; 16];
        output_extended.copy_from_slice(&registers[10..26]);
        Self {
            registers,
            output,
            output_extended,
        }
    }
}
