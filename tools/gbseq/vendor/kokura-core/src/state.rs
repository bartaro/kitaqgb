use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    apu::Apu,
    cartridge::Cartridge,
    cpu::Cpu,
    dma::DmaState,
    error::CoreError,
    interrupt::InterruptState,
    joypad::Joypad,
    memory::Memory,
    ppu::Ppu,
    serial::Serial,
    timer::Timer,
    types::{ClockState, HardwareMode},
    Machine,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateFileHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub rom_title: String,
    pub rom_size: u32,
    pub rom_checksum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateFile {
    pub header: StateFileHeader,
    pub state: Box<MachineState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineState {
    pub version: u32,
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub timer: Timer,
    pub apu: Apu,
    pub interrupt: InterruptState,
    pub joypad: Joypad,
    pub dma: DmaState,
    pub serial: Serial,
    pub cartridge: Cartridge,
    pub memory: Memory,
    pub clocks: ClockState,
    pub mode: HardwareMode,
    pub cgb_double_speed: bool,
    pub cgb_speed_switch_armed: bool,
    pub cgb_speed_switch_freeze_cycles: u32,
}

impl MachineState {
    pub const CURRENT_VERSION: u32 = 7;
    pub const FILE_MAGIC: [u8; 4] = *b"KQS1";

    pub fn from_machine(machine: &Machine) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            cpu: machine.cpu.clone(),
            ppu: machine.ppu.clone(),
            timer: machine.timer.clone(),
            apu: machine.apu.clone(),
            interrupt: machine.interrupt.clone(),
            joypad: machine.joypad.clone(),
            dma: machine.dma.clone(),
            serial: machine.serial.clone(),
            cartridge: machine.cartridge.clone(),
            memory: machine.memory.clone(),
            clocks: machine.clocks.clone(),
            mode: machine.mode,
            cgb_double_speed: machine.cgb_double_speed,
            cgb_speed_switch_armed: machine.cgb_speed_switch_armed,
            cgb_speed_switch_freeze_cycles: machine.cgb_speed_switch_freeze_cycles,
        }
    }

    pub fn apply_to(&self, machine: &mut Machine) {
        machine.cpu = self.cpu.clone();
        machine.ppu = self.ppu.clone();
        machine.timer = self.timer.clone();
        machine.apu = self.apu.clone();
        machine.interrupt = self.interrupt.clone();
        machine.joypad = self.joypad.clone();
        machine.dma = self.dma.clone();
        machine.serial = self.serial.clone();
        machine.cartridge = self.cartridge.clone();
        machine.memory = self.memory.clone();
        machine.clocks = self.clocks.clone();
        machine.mode = self.mode;
        machine.cgb_double_speed = self.cgb_double_speed;
        machine.cgb_speed_switch_armed = self.cgb_speed_switch_armed;
        machine.cgb_speed_switch_freeze_cycles = self.cgb_speed_switch_freeze_cycles;
        machine.active_interrupt_vector = None;
        machine.ppu.restore_runtime_state(&machine.memory);
    }

    pub fn rom_checksum(&self) -> u32 {
        self.cartridge
            .rom
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_mul(16777619) ^ b as u32)
    }

    pub fn file_header(&self) -> StateFileHeader {
        StateFileHeader {
            magic: Self::FILE_MAGIC,
            version: Self::CURRENT_VERSION,
            rom_title: self.cartridge.header.title.clone(),
            rom_size: self.cartridge.rom.len() as u32,
            rom_checksum: self.rom_checksum(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, CoreError> {
        let file = StateFile {
            header: self.file_header(),
            state: Box::new(self.clone()),
        };
        Ok(bincode::serialize(&file)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        let file: StateFile = bincode::deserialize(bytes)?;
        file.validate()?;
        file.state.validate()?;
        Ok(*file.state)
    }

    pub fn save_to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), CoreError> {
        let file = StateFile {
            header: self.file_header(),
            state: Box::new(self.clone()),
        };
        bincode::serialize_into(writer, &file)?;
        Ok(())
    }

    pub fn load_boxed_from_reader<R: std::io::Read>(reader: R) -> Result<Box<Self>, CoreError> {
        let file: StateFile = bincode::deserialize_from(reader)?;
        file.validate()?;
        file.state.validate()?;
        Ok(file.state)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), CoreError> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.save_to_writer(writer)?;
        Ok(())
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        Ok(*Self::load_boxed_from_path(path)?)
    }

    pub fn load_boxed_from_path<P: AsRef<Path>>(path: P) -> Result<Box<Self>, CoreError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::load_boxed_from_reader(reader)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.version != Self::CURRENT_VERSION {
            return Err(CoreError::InvalidState(format!(
                "unsupported state version {} (expected {})",
                self.version,
                Self::CURRENT_VERSION
            )));
        }
        if self.ppu.framebuffer.len() != 160 * 144 {
            return Err(CoreError::InvalidState("framebuffer size mismatch".into()));
        }
        Ok(())
    }

    pub fn validate_header_against_machine(&self, machine: &Machine) -> Result<(), CoreError> {
        let expected_title = machine.cartridge.header.title.clone();
        let expected_size = machine.cartridge.rom.len() as u32;
        let expected_checksum = machine
            .cartridge
            .rom
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_mul(16777619) ^ b as u32);

        let actual_title = self.cartridge.header.title.clone();
        let actual_size = self.cartridge.rom.len() as u32;
        let actual_checksum = self.rom_checksum();

        if actual_title != expected_title
            || actual_size != expected_size
            || actual_checksum != expected_checksum
        {
            return Err(CoreError::InvalidState(format!(
                "state ROM mismatch: state='{}' size={} checksum=0x{:08X}, machine='{}' size={} checksum=0x{:08X}",
                actual_title,
                actual_size,
                actual_checksum,
                expected_title,
                expected_size,
                expected_checksum
            )));
        }
        Ok(())
    }
}

impl StateFile {
    fn validate(&self) -> Result<(), CoreError> {
        if self.header.magic != MachineState::FILE_MAGIC {
            let magic = self.header.magic;
            return Err(CoreError::InvalidState(format!(
                "invalid state magic {:02X}{:02X}{:02X}{:02X}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }
        if self.header.version != MachineState::CURRENT_VERSION {
            return Err(CoreError::InvalidState(format!(
                "unsupported file header version {} (expected {})",
                self.header.version,
                MachineState::CURRENT_VERSION
            )));
        }

        let state = &self.state;
        let state_title = state.cartridge.header.title.clone();
        let state_size = state.cartridge.rom.len() as u32;
        let state_checksum = state.rom_checksum();

        if self.header.rom_title != state_title
            || self.header.rom_size != state_size
            || self.header.rom_checksum != state_checksum
        {
            return Err(CoreError::InvalidState(format!(
                "state header mismatch: header='{}' size={} checksum=0x{:08X}, state='{}' size={} checksum=0x{:08X}",
                self.header.rom_title,
                self.header.rom_size,
                self.header.rom_checksum,
                state_title,
                state_size,
                state_checksum
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MachineState;
    use crate::Machine;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn make_test_rom(program: &[u8]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        rom[0x0100..0x0100 + program.len()].copy_from_slice(program);
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom[0x0149] = 0x00;
        rom
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("kokura_{name}_{nanos}.kqs"))
    }

    #[test]
    fn boxed_state_roundtrip_from_path() {
        let mut machine = Machine::new();
        machine.load_rom(make_test_rom(&[0x00])).unwrap();
        let state = machine.save_state();
        let path = unique_temp_path("state_roundtrip");

        state.save_to_path(&path).unwrap();
        let loaded = MachineState::load_boxed_from_path(&path).unwrap();

        assert_eq!(loaded.cpu.pc, state.cpu.pc);
        assert_eq!(loaded.ppu.ly, state.ppu.ly);
        assert_eq!(loaded.memory.read8(0xFF40), state.memory.read8(0xFF40));

        fs::remove_file(path).unwrap();
    }
}
