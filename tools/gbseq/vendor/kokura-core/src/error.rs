use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("ROM too small: {0} bytes")]
    RomTooSmall(usize),
    #[error("unsupported cartridge type 0x{0:02X}")]
    UnsupportedCartridgeType(u8),
    #[error("unsupported opcode 0x{opcode:02X} at PC=0x{pc:04X}")]
    UnsupportedOpcode { opcode: u8, pc: u16 },
    #[error("invalid machine state: {0}")]
    InvalidState(String),
    #[error("state I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("state serialization error: {0}")]
    Bincode(#[from] bincode::Error),
}
