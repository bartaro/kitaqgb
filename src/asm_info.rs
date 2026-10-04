// Derived from the verified local AsmInfo.cs opcode definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Invalid,
    Implicit,
    Immediate8,
    Immediate16,
    Absolute,
    Relative,
    Indirect,
    HighMem,
}
impl Format {
    pub const fn size(self) -> usize {
        match self {
            Self::Immediate8 | Self::Relative | Self::HighMem => 1,
            Self::Immediate16 | Self::Absolute => 2,
            _ => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcode {
    pub mnemonic: &'static str,
    pub format: Format,
}
pub const OPCODES: [Opcode; 256] = [
    Opcode {
        mnemonic: "NOP",
        format: Format::Implicit,
    }, // 0x00
    Opcode {
        mnemonic: "LD_BC_IMM",
        format: Format::Immediate16,
    }, // 0x01
    Opcode {
        mnemonic: "LD_BC_A",
        format: Format::Implicit,
    }, // 0x02
    Opcode {
        mnemonic: "INC_BC",
        format: Format::Implicit,
    }, // 0x03
    Opcode {
        mnemonic: "INC_B",
        format: Format::Implicit,
    }, // 0x04
    Opcode {
        mnemonic: "DEC_B",
        format: Format::Implicit,
    }, // 0x05
    Opcode {
        mnemonic: "LD_B_IMM",
        format: Format::Immediate8,
    }, // 0x06
    Opcode {
        mnemonic: "RLCA",
        format: Format::Implicit,
    }, // 0x07
    Opcode {
        mnemonic: "LD_MEM_SP",
        format: Format::Immediate16,
    }, // 0x08
    Opcode {
        mnemonic: "ADD_HL_BC",
        format: Format::Implicit,
    }, // 0x09
    Opcode {
        mnemonic: "LD_A_BC",
        format: Format::Implicit,
    }, // 0x0A
    Opcode {
        mnemonic: "DEC_BC",
        format: Format::Implicit,
    }, // 0x0B
    Opcode {
        mnemonic: "INC_C",
        format: Format::Implicit,
    }, // 0x0C
    Opcode {
        mnemonic: "DEC_C",
        format: Format::Implicit,
    }, // 0x0D
    Opcode {
        mnemonic: "LD_C_IMM",
        format: Format::Immediate8,
    }, // 0x0E
    Opcode {
        mnemonic: "RRCA",
        format: Format::Implicit,
    }, // 0x0F
    Opcode {
        mnemonic: "STOP",
        format: Format::Immediate8,
    }, // 0x10
    Opcode {
        mnemonic: "LD_DE_IMM",
        format: Format::Immediate16,
    }, // 0x11
    Opcode {
        mnemonic: "LD_DE_A",
        format: Format::Implicit,
    }, // 0x12
    Opcode {
        mnemonic: "INC_DE",
        format: Format::Implicit,
    }, // 0x13
    Opcode {
        mnemonic: "INC_D",
        format: Format::Implicit,
    }, // 0x14
    Opcode {
        mnemonic: "DEC_D",
        format: Format::Implicit,
    }, // 0x15
    Opcode {
        mnemonic: "LD_D_IMM",
        format: Format::Immediate8,
    }, // 0x16
    Opcode {
        mnemonic: "RLA",
        format: Format::Implicit,
    }, // 0x17
    Opcode {
        mnemonic: "JR",
        format: Format::Relative,
    }, // 0x18
    Opcode {
        mnemonic: "ADD_HL_DE",
        format: Format::Implicit,
    }, // 0x19
    Opcode {
        mnemonic: "LD_A_DE",
        format: Format::Implicit,
    }, // 0x1A
    Opcode {
        mnemonic: "DEC_DE",
        format: Format::Implicit,
    }, // 0x1B
    Opcode {
        mnemonic: "INC_E",
        format: Format::Implicit,
    }, // 0x1C
    Opcode {
        mnemonic: "DEC_E",
        format: Format::Implicit,
    }, // 0x1D
    Opcode {
        mnemonic: "LD_E_IMM",
        format: Format::Immediate8,
    }, // 0x1E
    Opcode {
        mnemonic: "RRA",
        format: Format::Implicit,
    }, // 0x1F
    Opcode {
        mnemonic: "JR_NZ",
        format: Format::Relative,
    }, // 0x20
    Opcode {
        mnemonic: "LD_HL_IMM",
        format: Format::Immediate16,
    }, // 0x21
    Opcode {
        mnemonic: "LDI_HL_A",
        format: Format::Implicit,
    }, // 0x22
    Opcode {
        mnemonic: "INC_HL",
        format: Format::Implicit,
    }, // 0x23
    Opcode {
        mnemonic: "INC_H",
        format: Format::Implicit,
    }, // 0x24
    Opcode {
        mnemonic: "DEC_H",
        format: Format::Implicit,
    }, // 0x25
    Opcode {
        mnemonic: "LD_H_IMM",
        format: Format::Immediate8,
    }, // 0x26
    Opcode {
        mnemonic: "DAA",
        format: Format::Implicit,
    }, // 0x27
    Opcode {
        mnemonic: "JR_Z",
        format: Format::Relative,
    }, // 0x28
    Opcode {
        mnemonic: "ADD_HL_HL",
        format: Format::Implicit,
    }, // 0x29
    Opcode {
        mnemonic: "LDI_A_HL",
        format: Format::Implicit,
    }, // 0x2A
    Opcode {
        mnemonic: "DEC_HL",
        format: Format::Implicit,
    }, // 0x2B
    Opcode {
        mnemonic: "INC_L",
        format: Format::Implicit,
    }, // 0x2C
    Opcode {
        mnemonic: "DEC_L",
        format: Format::Implicit,
    }, // 0x2D
    Opcode {
        mnemonic: "LD_L_IMM",
        format: Format::Immediate8,
    }, // 0x2E
    Opcode {
        mnemonic: "CPL",
        format: Format::Implicit,
    }, // 0x2F
    Opcode {
        mnemonic: "JR_NC",
        format: Format::Relative,
    }, // 0x30
    Opcode {
        mnemonic: "LD_SP_IMM",
        format: Format::Immediate16,
    }, // 0x31
    Opcode {
        mnemonic: "LDD_HL_A",
        format: Format::Implicit,
    }, // 0x32
    Opcode {
        mnemonic: "INC_SP",
        format: Format::Implicit,
    }, // 0x33
    Opcode {
        mnemonic: "INC_HL_REF",
        format: Format::Implicit,
    }, // 0x34
    Opcode {
        mnemonic: "DEC_HL_REF",
        format: Format::Implicit,
    }, // 0x35
    Opcode {
        mnemonic: "LD_HL_REF_IMM",
        format: Format::Immediate8,
    }, // 0x36
    Opcode {
        mnemonic: "SCF",
        format: Format::Implicit,
    }, // 0x37
    Opcode {
        mnemonic: "JR_C",
        format: Format::Relative,
    }, // 0x38
    Opcode {
        mnemonic: "ADD_HL_SP",
        format: Format::Implicit,
    }, // 0x39
    Opcode {
        mnemonic: "LDD_A_HL",
        format: Format::Implicit,
    }, // 0x3A
    Opcode {
        mnemonic: "DEC_SP",
        format: Format::Implicit,
    }, // 0x3B
    Opcode {
        mnemonic: "INC_A",
        format: Format::Implicit,
    }, // 0x3C
    Opcode {
        mnemonic: "DEC_A",
        format: Format::Implicit,
    }, // 0x3D
    Opcode {
        mnemonic: "LD_A_IMM",
        format: Format::Immediate8,
    }, // 0x3E
    Opcode {
        mnemonic: "CCF",
        format: Format::Implicit,
    }, // 0x3F
    Opcode {
        mnemonic: "LD_B_B",
        format: Format::Implicit,
    }, // 0x40
    Opcode {
        mnemonic: "LD_B_C",
        format: Format::Implicit,
    }, // 0x41
    Opcode {
        mnemonic: "LD_B_D",
        format: Format::Implicit,
    }, // 0x42
    Opcode {
        mnemonic: "LD_B_E",
        format: Format::Implicit,
    }, // 0x43
    Opcode {
        mnemonic: "LD_B_H",
        format: Format::Implicit,
    }, // 0x44
    Opcode {
        mnemonic: "LD_B_L",
        format: Format::Implicit,
    }, // 0x45
    Opcode {
        mnemonic: "LD_B_HL",
        format: Format::Implicit,
    }, // 0x46
    Opcode {
        mnemonic: "LD_B_A",
        format: Format::Implicit,
    }, // 0x47
    Opcode {
        mnemonic: "LD_C_B",
        format: Format::Implicit,
    }, // 0x48
    Opcode {
        mnemonic: "LD_C_C",
        format: Format::Implicit,
    }, // 0x49
    Opcode {
        mnemonic: "LD_C_D",
        format: Format::Implicit,
    }, // 0x4A
    Opcode {
        mnemonic: "LD_C_E",
        format: Format::Implicit,
    }, // 0x4B
    Opcode {
        mnemonic: "LD_C_H",
        format: Format::Implicit,
    }, // 0x4C
    Opcode {
        mnemonic: "LD_C_L",
        format: Format::Implicit,
    }, // 0x4D
    Opcode {
        mnemonic: "LD_C_HL",
        format: Format::Implicit,
    }, // 0x4E
    Opcode {
        mnemonic: "LD_C_A",
        format: Format::Implicit,
    }, // 0x4F
    Opcode {
        mnemonic: "LD_D_B",
        format: Format::Implicit,
    }, // 0x50
    Opcode {
        mnemonic: "LD_D_C",
        format: Format::Implicit,
    }, // 0x51
    Opcode {
        mnemonic: "LD_D_D",
        format: Format::Implicit,
    }, // 0x52
    Opcode {
        mnemonic: "LD_D_E",
        format: Format::Implicit,
    }, // 0x53
    Opcode {
        mnemonic: "LD_D_H",
        format: Format::Implicit,
    }, // 0x54
    Opcode {
        mnemonic: "LD_D_L",
        format: Format::Implicit,
    }, // 0x55
    Opcode {
        mnemonic: "LD_D_HL",
        format: Format::Implicit,
    }, // 0x56
    Opcode {
        mnemonic: "LD_D_A",
        format: Format::Implicit,
    }, // 0x57
    Opcode {
        mnemonic: "LD_E_B",
        format: Format::Implicit,
    }, // 0x58
    Opcode {
        mnemonic: "LD_E_C",
        format: Format::Implicit,
    }, // 0x59
    Opcode {
        mnemonic: "LD_E_D",
        format: Format::Implicit,
    }, // 0x5A
    Opcode {
        mnemonic: "LD_E_E",
        format: Format::Implicit,
    }, // 0x5B
    Opcode {
        mnemonic: "LD_E_H",
        format: Format::Implicit,
    }, // 0x5C
    Opcode {
        mnemonic: "LD_E_L",
        format: Format::Implicit,
    }, // 0x5D
    Opcode {
        mnemonic: "LD_E_HL",
        format: Format::Implicit,
    }, // 0x5E
    Opcode {
        mnemonic: "LD_E_A",
        format: Format::Implicit,
    }, // 0x5F
    Opcode {
        mnemonic: "LD_H_B",
        format: Format::Implicit,
    }, // 0x60
    Opcode {
        mnemonic: "LD_H_C",
        format: Format::Implicit,
    }, // 0x61
    Opcode {
        mnemonic: "LD_H_D",
        format: Format::Implicit,
    }, // 0x62
    Opcode {
        mnemonic: "LD_H_E",
        format: Format::Implicit,
    }, // 0x63
    Opcode {
        mnemonic: "LD_H_H",
        format: Format::Implicit,
    }, // 0x64
    Opcode {
        mnemonic: "LD_H_L",
        format: Format::Implicit,
    }, // 0x65
    Opcode {
        mnemonic: "LD_H_HL",
        format: Format::Implicit,
    }, // 0x66
    Opcode {
        mnemonic: "LD_H_A",
        format: Format::Implicit,
    }, // 0x67
    Opcode {
        mnemonic: "LD_L_B",
        format: Format::Implicit,
    }, // 0x68
    Opcode {
        mnemonic: "LD_L_C",
        format: Format::Implicit,
    }, // 0x69
    Opcode {
        mnemonic: "LD_L_D",
        format: Format::Implicit,
    }, // 0x6A
    Opcode {
        mnemonic: "LD_L_E",
        format: Format::Implicit,
    }, // 0x6B
    Opcode {
        mnemonic: "LD_L_H",
        format: Format::Implicit,
    }, // 0x6C
    Opcode {
        mnemonic: "LD_L_L",
        format: Format::Implicit,
    }, // 0x6D
    Opcode {
        mnemonic: "LD_L_HL",
        format: Format::Implicit,
    }, // 0x6E
    Opcode {
        mnemonic: "LD_L_A",
        format: Format::Implicit,
    }, // 0x6F
    Opcode {
        mnemonic: "LD_HL_B",
        format: Format::Implicit,
    }, // 0x70
    Opcode {
        mnemonic: "LD_HL_C",
        format: Format::Implicit,
    }, // 0x71
    Opcode {
        mnemonic: "LD_HL_D",
        format: Format::Implicit,
    }, // 0x72
    Opcode {
        mnemonic: "LD_HL_E",
        format: Format::Implicit,
    }, // 0x73
    Opcode {
        mnemonic: "LD_HL_H",
        format: Format::Implicit,
    }, // 0x74
    Opcode {
        mnemonic: "LD_HL_L",
        format: Format::Implicit,
    }, // 0x75
    Opcode {
        mnemonic: "HALT",
        format: Format::Implicit,
    }, // 0x76
    Opcode {
        mnemonic: "LD_HL_A",
        format: Format::Implicit,
    }, // 0x77
    Opcode {
        mnemonic: "LD_A_B",
        format: Format::Implicit,
    }, // 0x78
    Opcode {
        mnemonic: "LD_A_C",
        format: Format::Implicit,
    }, // 0x79
    Opcode {
        mnemonic: "LD_A_D",
        format: Format::Implicit,
    }, // 0x7A
    Opcode {
        mnemonic: "LD_A_E",
        format: Format::Implicit,
    }, // 0x7B
    Opcode {
        mnemonic: "LD_A_H",
        format: Format::Implicit,
    }, // 0x7C
    Opcode {
        mnemonic: "LD_A_L",
        format: Format::Implicit,
    }, // 0x7D
    Opcode {
        mnemonic: "LD_A_HL",
        format: Format::Implicit,
    }, // 0x7E
    Opcode {
        mnemonic: "LD_A_A",
        format: Format::Implicit,
    }, // 0x7F
    Opcode {
        mnemonic: "ADD_B",
        format: Format::Implicit,
    }, // 0x80
    Opcode {
        mnemonic: "ADD_C",
        format: Format::Implicit,
    }, // 0x81
    Opcode {
        mnemonic: "ADD_D",
        format: Format::Implicit,
    }, // 0x82
    Opcode {
        mnemonic: "ADD_E",
        format: Format::Implicit,
    }, // 0x83
    Opcode {
        mnemonic: "ADD_H",
        format: Format::Implicit,
    }, // 0x84
    Opcode {
        mnemonic: "ADD_L",
        format: Format::Implicit,
    }, // 0x85
    Opcode {
        mnemonic: "ADD_HL_REF",
        format: Format::Implicit,
    }, // 0x86
    Opcode {
        mnemonic: "ADD_A",
        format: Format::Implicit,
    }, // 0x87
    Opcode {
        mnemonic: "ADC_B",
        format: Format::Implicit,
    }, // 0x88
    Opcode {
        mnemonic: "ADC_C",
        format: Format::Implicit,
    }, // 0x89
    Opcode {
        mnemonic: "ADC_D",
        format: Format::Implicit,
    }, // 0x8A
    Opcode {
        mnemonic: "ADC_E",
        format: Format::Implicit,
    }, // 0x8B
    Opcode {
        mnemonic: "ADC_H",
        format: Format::Implicit,
    }, // 0x8C
    Opcode {
        mnemonic: "ADC_L",
        format: Format::Implicit,
    }, // 0x8D
    Opcode {
        mnemonic: "ADC_HL_REF",
        format: Format::Implicit,
    }, // 0x8E
    Opcode {
        mnemonic: "ADC_A",
        format: Format::Implicit,
    }, // 0x8F
    Opcode {
        mnemonic: "SUB_B",
        format: Format::Implicit,
    }, // 0x90
    Opcode {
        mnemonic: "SUB_C",
        format: Format::Implicit,
    }, // 0x91
    Opcode {
        mnemonic: "SUB_D",
        format: Format::Implicit,
    }, // 0x92
    Opcode {
        mnemonic: "SUB_E",
        format: Format::Implicit,
    }, // 0x93
    Opcode {
        mnemonic: "SUB_H",
        format: Format::Implicit,
    }, // 0x94
    Opcode {
        mnemonic: "SUB_L",
        format: Format::Implicit,
    }, // 0x95
    Opcode {
        mnemonic: "SUB_HL_REF",
        format: Format::Implicit,
    }, // 0x96
    Opcode {
        mnemonic: "SUB_A",
        format: Format::Implicit,
    }, // 0x97
    Opcode {
        mnemonic: "SBC_B",
        format: Format::Implicit,
    }, // 0x98
    Opcode {
        mnemonic: "SBC_C",
        format: Format::Implicit,
    }, // 0x99
    Opcode {
        mnemonic: "SBC_D",
        format: Format::Implicit,
    }, // 0x9A
    Opcode {
        mnemonic: "SBC_E",
        format: Format::Implicit,
    }, // 0x9B
    Opcode {
        mnemonic: "SBC_H",
        format: Format::Implicit,
    }, // 0x9C
    Opcode {
        mnemonic: "SBC_L",
        format: Format::Implicit,
    }, // 0x9D
    Opcode {
        mnemonic: "SBC_HL_REF",
        format: Format::Implicit,
    }, // 0x9E
    Opcode {
        mnemonic: "SBC_A",
        format: Format::Implicit,
    }, // 0x9F
    Opcode {
        mnemonic: "AND_B",
        format: Format::Implicit,
    }, // 0xA0
    Opcode {
        mnemonic: "AND_C",
        format: Format::Implicit,
    }, // 0xA1
    Opcode {
        mnemonic: "AND_D",
        format: Format::Implicit,
    }, // 0xA2
    Opcode {
        mnemonic: "AND_E",
        format: Format::Implicit,
    }, // 0xA3
    Opcode {
        mnemonic: "AND_H",
        format: Format::Implicit,
    }, // 0xA4
    Opcode {
        mnemonic: "AND_L",
        format: Format::Implicit,
    }, // 0xA5
    Opcode {
        mnemonic: "AND_HL_REF",
        format: Format::Implicit,
    }, // 0xA6
    Opcode {
        mnemonic: "AND_A",
        format: Format::Implicit,
    }, // 0xA7
    Opcode {
        mnemonic: "XOR_B",
        format: Format::Implicit,
    }, // 0xA8
    Opcode {
        mnemonic: "XOR_C",
        format: Format::Implicit,
    }, // 0xA9
    Opcode {
        mnemonic: "XOR_D",
        format: Format::Implicit,
    }, // 0xAA
    Opcode {
        mnemonic: "XOR_E",
        format: Format::Implicit,
    }, // 0xAB
    Opcode {
        mnemonic: "XOR_H",
        format: Format::Implicit,
    }, // 0xAC
    Opcode {
        mnemonic: "XOR_L",
        format: Format::Implicit,
    }, // 0xAD
    Opcode {
        mnemonic: "XOR_HL_REF",
        format: Format::Implicit,
    }, // 0xAE
    Opcode {
        mnemonic: "XOR_A",
        format: Format::Implicit,
    }, // 0xAF
    Opcode {
        mnemonic: "OR_B",
        format: Format::Implicit,
    }, // 0xB0
    Opcode {
        mnemonic: "OR_C",
        format: Format::Implicit,
    }, // 0xB1
    Opcode {
        mnemonic: "OR_D",
        format: Format::Implicit,
    }, // 0xB2
    Opcode {
        mnemonic: "OR_E",
        format: Format::Implicit,
    }, // 0xB3
    Opcode {
        mnemonic: "OR_H",
        format: Format::Implicit,
    }, // 0xB4
    Opcode {
        mnemonic: "OR_L",
        format: Format::Implicit,
    }, // 0xB5
    Opcode {
        mnemonic: "OR_HL_REF",
        format: Format::Implicit,
    }, // 0xB6
    Opcode {
        mnemonic: "OR_A",
        format: Format::Implicit,
    }, // 0xB7
    Opcode {
        mnemonic: "CP_B",
        format: Format::Implicit,
    }, // 0xB8
    Opcode {
        mnemonic: "CP_C",
        format: Format::Implicit,
    }, // 0xB9
    Opcode {
        mnemonic: "CP_D",
        format: Format::Implicit,
    }, // 0xBA
    Opcode {
        mnemonic: "CP_E",
        format: Format::Implicit,
    }, // 0xBB
    Opcode {
        mnemonic: "CP_H",
        format: Format::Implicit,
    }, // 0xBC
    Opcode {
        mnemonic: "CP_L",
        format: Format::Implicit,
    }, // 0xBD
    Opcode {
        mnemonic: "CP_HL_REF",
        format: Format::Implicit,
    }, // 0xBE
    Opcode {
        mnemonic: "CP_A",
        format: Format::Implicit,
    }, // 0xBF
    Opcode {
        mnemonic: "RET_NZ",
        format: Format::Implicit,
    }, // 0xC0
    Opcode {
        mnemonic: "POP_BC",
        format: Format::Implicit,
    }, // 0xC1
    Opcode {
        mnemonic: "JP_NZ",
        format: Format::Absolute,
    }, // 0xC2
    Opcode {
        mnemonic: "JP",
        format: Format::Absolute,
    }, // 0xC3
    Opcode {
        mnemonic: "CALL_NZ",
        format: Format::Absolute,
    }, // 0xC4
    Opcode {
        mnemonic: "PUSH_BC",
        format: Format::Implicit,
    }, // 0xC5
    Opcode {
        mnemonic: "ADD_A_IMM",
        format: Format::Immediate8,
    }, // 0xC6
    Opcode {
        mnemonic: "RST_00",
        format: Format::Implicit,
    }, // 0xC7
    Opcode {
        mnemonic: "RET_Z",
        format: Format::Implicit,
    }, // 0xC8
    Opcode {
        mnemonic: "RET",
        format: Format::Implicit,
    }, // 0xC9
    Opcode {
        mnemonic: "JP_Z",
        format: Format::Absolute,
    }, // 0xCA
    Opcode {
        mnemonic: "PREFIX_CB",
        format: Format::Implicit,
    }, // 0xCB
    Opcode {
        mnemonic: "CALL_Z",
        format: Format::Absolute,
    }, // 0xCC
    Opcode {
        mnemonic: "CALL",
        format: Format::Absolute,
    }, // 0xCD
    Opcode {
        mnemonic: "ADC_IMM",
        format: Format::Immediate8,
    }, // 0xCE
    Opcode {
        mnemonic: "RST_08",
        format: Format::Implicit,
    }, // 0xCF
    Opcode {
        mnemonic: "RET_NC",
        format: Format::Implicit,
    }, // 0xD0
    Opcode {
        mnemonic: "POP_DE",
        format: Format::Implicit,
    }, // 0xD1
    Opcode {
        mnemonic: "JP_NC",
        format: Format::Absolute,
    }, // 0xD2
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xD3
    Opcode {
        mnemonic: "CALL_NC",
        format: Format::Absolute,
    }, // 0xD4
    Opcode {
        mnemonic: "PUSH_DE",
        format: Format::Implicit,
    }, // 0xD5
    Opcode {
        mnemonic: "SUB_IMM",
        format: Format::Immediate8,
    }, // 0xD6
    Opcode {
        mnemonic: "RST_10",
        format: Format::Implicit,
    }, // 0xD7
    Opcode {
        mnemonic: "RET_C",
        format: Format::Implicit,
    }, // 0xD8
    Opcode {
        mnemonic: "RETI",
        format: Format::Implicit,
    }, // 0xD9
    Opcode {
        mnemonic: "JP_C",
        format: Format::Absolute,
    }, // 0xDA
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xDB
    Opcode {
        mnemonic: "CALL_C",
        format: Format::Absolute,
    }, // 0xDC
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xDD
    Opcode {
        mnemonic: "SBC_IMM",
        format: Format::Immediate8,
    }, // 0xDE
    Opcode {
        mnemonic: "RST_18",
        format: Format::Implicit,
    }, // 0xDF
    Opcode {
        mnemonic: "LDH_MEM_A",
        format: Format::HighMem,
    }, // 0xE0
    Opcode {
        mnemonic: "POP_HL",
        format: Format::Implicit,
    }, // 0xE1
    Opcode {
        mnemonic: "LD_C_MEM_A",
        format: Format::Implicit,
    }, // 0xE2
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xE3
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xE4
    Opcode {
        mnemonic: "PUSH_HL",
        format: Format::Implicit,
    }, // 0xE5
    Opcode {
        mnemonic: "AND_IMM",
        format: Format::Immediate8,
    }, // 0xE6
    Opcode {
        mnemonic: "RST_20",
        format: Format::Implicit,
    }, // 0xE7
    Opcode {
        mnemonic: "ADD_SP_IMM",
        format: Format::Relative,
    }, // 0xE8
    Opcode {
        mnemonic: "JP_HL",
        format: Format::Implicit,
    }, // 0xE9
    Opcode {
        mnemonic: "LD_MEM_A",
        format: Format::Absolute,
    }, // 0xEA
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xEB
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xEC
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xED
    Opcode {
        mnemonic: "XOR_IMM",
        format: Format::Immediate8,
    }, // 0xEE
    Opcode {
        mnemonic: "RST_28",
        format: Format::Implicit,
    }, // 0xEF
    Opcode {
        mnemonic: "LDH_A_MEM",
        format: Format::HighMem,
    }, // 0xF0
    Opcode {
        mnemonic: "POP_AF",
        format: Format::Implicit,
    }, // 0xF1
    Opcode {
        mnemonic: "LD_A_MEM_C",
        format: Format::Implicit,
    }, // 0xF2
    Opcode {
        mnemonic: "DI",
        format: Format::Implicit,
    }, // 0xF3
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xF4
    Opcode {
        mnemonic: "PUSH_AF",
        format: Format::Implicit,
    }, // 0xF5
    Opcode {
        mnemonic: "OR_IMM",
        format: Format::Immediate8,
    }, // 0xF6
    Opcode {
        mnemonic: "RST_30",
        format: Format::Implicit,
    }, // 0xF7
    Opcode {
        mnemonic: "LD_HL_SP_IMM",
        format: Format::Relative,
    }, // 0xF8
    Opcode {
        mnemonic: "LD_SP_HL",
        format: Format::Implicit,
    }, // 0xF9
    Opcode {
        mnemonic: "LD_A_MEM",
        format: Format::Absolute,
    }, // 0xFA
    Opcode {
        mnemonic: "EI",
        format: Format::Implicit,
    }, // 0xFB
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xFC
    Opcode {
        mnemonic: "???",
        format: Format::Invalid,
    }, // 0xFD
    Opcode {
        mnemonic: "CP_IMM",
        format: Format::Immediate8,
    }, // 0xFE
    Opcode {
        mnemonic: "RST_38",
        format: Format::Implicit,
    }, // 0xFF
];
pub const SHORT_JUMPS: [&str; 5] = ["JR", "JR_NZ", "JR_Z", "JR_NC", "JR_C"];
pub fn find(mnemonic: &str) -> Option<(u8, Opcode)> {
    OPCODES
        .iter()
        .copied()
        .enumerate()
        .find(|(_, o)| o.mnemonic == mnemonic && o.format != Format::Invalid)
        .map(|(i, o)| (i as u8, o))
}
