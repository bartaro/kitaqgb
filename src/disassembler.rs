use crate::asm_info::{Format, OPCODES};
use std::io::{self, Write};
const BANK_SIZE: usize = 0x4000;
pub fn decode_cb(op: u8) -> String {
    let regs = ["B", "C", "D", "E", "H", "L", "[HL]", "A"];
    let y = (op >> 3) & 7;
    let r = regs[(op & 7) as usize];
    match op >> 6 {
        0 => format!(
            "{} {r}",
            ["RLC", "RRC", "RL", "RR", "SLA", "SRA", "SWAP", "SRL"][y as usize]
        ),
        1 => format!("BIT {y},{r}"),
        2 => format!("RES {y},{r}"),
        _ => format!("SET {y},{r}"),
    }
}
/// Stream the whole ROM while respecting every 16 KiB bank boundary.
pub fn disassemble(rom: &[u8], filename: &str, mut w: impl Write) -> io::Result<()> {
    writeln!(w, "; GB-compatible ROM Disassembly (full) ")?;
    writeln!(w, "; File: {filename}")?;
    writeln!(
        w,
        "; ROM size: {} bytes ({} KB)\n",
        rom.len(),
        rom.len() / 1024
    )?;
    for (bank, bytes) in rom.chunks(BANK_SIZE).enumerate() {
        let start = bank * BANK_SIZE;
        let base = if bank == 0 { 0 } else { BANK_SIZE };
        let mut addr = base;
        let mut off = 0;
        writeln!(
            w,
            "; ------------------------------------------------------------"
        )?;
        writeln!(
            w,
            "; BANK {bank}  file_off=${start:06X}  size=${:04X}  cpu=${base:04X}-{:04X}",
            bytes.len(),
            base + bytes.len() - 1
        )?;
        writeln!(
            w,
            "; ------------------------------------------------------------"
        )?;
        while off < bytes.len() {
            let current = addr;
            let fileoff = start + off;
            let code = bytes[off];
            off += 1;
            if code == 0xcb {
                let cb = if off < bytes.len() {
                    let b = bytes[off];
                    off += 1;
                    b
                } else {
                    0xff
                };
                writeln!(
                    w,
                    "{current:04X}    {code:02X} {cb:02X}    {}    ; file_off=${fileoff:06X}",
                    decode_cb(cb)
                )?;
                addr += 2;
            } else {
                let op = OPCODES[code as usize];
                let size = op.format.size();
                let mut value = 0u16;
                let mut imm0 = 0xff;
                write!(w, "{current:04X}    {code:02X}")?;
                for i in 0..2 {
                    if i < size {
                        let imm = if off < bytes.len() {
                            let b = bytes[off];
                            off += 1;
                            b
                        } else {
                            0xff
                        };
                        if i == 0 {
                            imm0 = imm;
                        }
                        value |= (imm as u16) << (i * 8);
                        write!(w, " {imm:02X}")?;
                    } else {
                        write!(w, "   ")?;
                    }
                }
                write!(w, "    ")?;
                if op.mnemonic == "???" {
                    write!(w, "DB ${code:02X}")?;
                } else if op.format == Format::Relative {
                    let rel = imm0 as i8;
                    let target = current as i32 + 2 + rel as i32;
                    let window = base as i32..=base as i32 + 0x3fff;
                    write!(
                        w,
                        "{} ${:04X} ({rel}){}",
                        op.mnemonic,
                        target & 0xffff,
                        if window.contains(&target) {
                            ""
                        } else {
                            " ; out-of-bank"
                        }
                    )?;
                } else {
                    write!(w, "{}", op.mnemonic)?;
                    match op.format {
                        Format::Invalid => write!(w, " ???")?,
                        Format::Immediate8 => write!(w, " ${value:02X}")?,
                        Format::Immediate16 => write!(w, " ${value:04X}")?,
                        Format::Absolute => write!(w, " [${value:04X}]")?,
                        Format::Indirect => write!(w, " [HL]")?,
                        Format::HighMem => write!(w, " [$FF00+${value:02X}]")?,
                        _ => {}
                    }
                }
                writeln!(w, "    ; file_off=${fileoff:06X}")?;
                addr += 1 + size;
            }
            if addr > base + BANK_SIZE {
                break;
            }
        }
        writeln!(w)?;
    }
    Ok(())
}
