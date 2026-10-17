use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmaState {
    pub active: bool,
    pub source: u16,
    #[serde(skip)]
    pub bytes_copied: u16,
    #[serde(skip)]
    pub cycle_accum: u8,
    #[serde(skip)]
    pub start_delay_cycles: u8,
    pub ff46: u8,
    pub hdma1: u8,
    pub hdma2: u8,
    pub hdma3: u8,
    pub hdma4: u8,
    pub hdma5: u8,
    pub hdma_source: u16,
    pub hdma_dest: u16,
    pub hdma_blocks_remaining: u8,
    pub hdma_total_blocks: u8,
    pub hdma_hblank_mode: bool,
    pub hdma_active: bool,
}

impl Default for DmaState {
    fn default() -> Self {
        Self {
            active: false,
            source: 0,
            bytes_copied: 0,
            cycle_accum: 0,
            start_delay_cycles: 0,
            ff46: 0xFF,
            hdma1: 0xFF,
            hdma2: 0xFF,
            hdma3: 0xFF,
            hdma4: 0xFF,
            hdma5: 0xFF,
            hdma_source: 0,
            hdma_dest: 0x8000,
            hdma_blocks_remaining: 0,
            hdma_total_blocks: 0,
            hdma_hblank_mode: false,
            hdma_active: false,
        }
    }
}
