using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

public static class AsmInfo
{
    public static string[] Mnemonics = new string[256];
    public static int[] OperandFormats = new int[256];
    public static int[] OperandSizes = new int[10];
    public static string[] OperandFormatStrings = new string[10];

    public const int INV = 0;
    public const int IMP = 1;
    public const int IMM8 = 2;
    public const int IMM16 = 3;
    public const int ABS = 4;
    public const int REL = 5;
    public const int IND = 6;
    public const int LDH = 7;

    public const int IMM = IMM8;
    public const int ZPG = LDH;

    public const int ZPX = INV;
    public const int ABX = INV;
    public const int ABY = INV;
    public const int INV_MODE = INV;
    public const int ZXI = INV;
    public const int ZYI = INV;

    static AsmInfo()
    {
        for (int i = 0; i < 256; i++) { Mnemonics[i] = "???"; OperandFormats[i] = INV; }

        OperandSizes[INV] = 0;
        OperandSizes[IMP] = 0;
        OperandSizes[IMM8] = 1;
        OperandSizes[IMM16] = 2;
        OperandSizes[ABS] = 2;
        OperandSizes[REL] = 1;
        OperandSizes[IND] = 0;
        OperandSizes[LDH] = 1;

        OperandFormatStrings[INV] = " ???";
        OperandFormatStrings[IMP] = "";
        OperandFormatStrings[IMM8] = " ${0:X2}";
        OperandFormatStrings[IMM16] = " ${0:X4}";
        OperandFormatStrings[ABS] = " [${0:X4}]";
        OperandFormatStrings[REL] = " ${0:X2}";
        OperandFormatStrings[IND] = " [HL]";
        OperandFormatStrings[LDH] = " [$FF00+${0:X2}]";

        Def(0x00, "NOP", IMP);
        Def(0xF3, "DI", IMP);
        Def(0xFB, "EI", IMP);
        Def(0x76, "HALT", IMP);
        Def(0x10, "STOP", IMM8);
        Def(0xC9, "RET", IMP);
        Def(0xD9, "RETI", IMP);
        Def(0xC3, "JP", ABS);
        Def(0xE9, "JP_HL", IMP);

        Def(0x18, "JR", REL);
        Def(0x20, "JR_NZ", REL);
        Def(0x28, "JR_Z", REL);
        Def(0x30, "JR_NC", REL);
        Def(0x38, "JR_C", REL);

        Def(0xCD, "CALL", ABS);
        Def(0xC4, "CALL_NZ", ABS);
        Def(0xCC, "CALL_Z", ABS);
        Def(0xD4, "CALL_NC", ABS);
        Def(0xDC, "CALL_C", ABS);

        Def(0xC2, "JP_NZ", ABS);
        Def(0xCA, "JP_Z", ABS);
        Def(0xD2, "JP_NC", ABS);
        Def(0xDA, "JP_C", ABS);

        // RST vectors (call-like, 1 byte)
        Def(0xC7, "RST_00", IMP);
        Def(0xCF, "RST_08", IMP);
        Def(0xD7, "RST_10", IMP);
        Def(0xDF, "RST_18", IMP);
        Def(0xE7, "RST_20", IMP);
        Def(0xEF, "RST_28", IMP);
        Def(0xF7, "RST_30", IMP);
        Def(0xFF, "RST_38", IMP);

        Def(0x3E, "LD_A_IMM", IMM8);
        Def(0x06, "LD_B_IMM", IMM8);
        Def(0x0E, "LD_C_IMM", IMM8);
        Def(0x16, "LD_D_IMM", IMM8);
        Def(0x1E, "LD_E_IMM", IMM8);
        Def(0x26, "LD_H_IMM", IMM8);
        Def(0x2E, "LD_L_IMM", IMM8);

        Def(0x7F, "LD_A_A", IMP); Def(0x78, "LD_A_B", IMP); Def(0x79, "LD_A_C", IMP); Def(0x7A, "LD_A_D", IMP); Def(0x7B, "LD_A_E", IMP); Def(0x7C, "LD_A_H", IMP); Def(0x7D, "LD_A_L", IMP);
        Def(0x47, "LD_B_A", IMP); Def(0x40, "LD_B_B", IMP); Def(0x41, "LD_B_C", IMP); Def(0x42, "LD_B_D", IMP); Def(0x43, "LD_B_E", IMP); Def(0x44, "LD_B_H", IMP); Def(0x45, "LD_B_L", IMP);
        Def(0x4F, "LD_C_A", IMP); Def(0x48, "LD_C_B", IMP); Def(0x49, "LD_C_C", IMP); Def(0x4A, "LD_C_D", IMP); Def(0x4B, "LD_C_E", IMP); Def(0x4C, "LD_C_H", IMP); Def(0x4D, "LD_C_L", IMP);
        Def(0x57, "LD_D_A", IMP); Def(0x50, "LD_D_B", IMP); Def(0x51, "LD_D_C", IMP); Def(0x52, "LD_D_D", IMP); Def(0x53, "LD_D_E", IMP); Def(0x54, "LD_D_H", IMP); Def(0x55, "LD_D_L", IMP);
        Def(0x5F, "LD_E_A", IMP); Def(0x58, "LD_E_B", IMP); Def(0x59, "LD_E_C", IMP); Def(0x5A, "LD_E_D", IMP); Def(0x5B, "LD_E_E", IMP); Def(0x5C, "LD_E_H", IMP); Def(0x5D, "LD_E_L", IMP);
        Def(0x67, "LD_H_A", IMP); Def(0x60, "LD_H_B", IMP); Def(0x61, "LD_H_C", IMP); Def(0x62, "LD_H_D", IMP); Def(0x63, "LD_H_E", IMP); Def(0x64, "LD_H_H", IMP); Def(0x65, "LD_H_L", IMP);
        Def(0x6F, "LD_L_A", IMP); Def(0x68, "LD_L_B", IMP); Def(0x69, "LD_L_C", IMP); Def(0x6A, "LD_L_D", IMP); Def(0x6B, "LD_L_E", IMP); Def(0x6C, "LD_L_H", IMP); Def(0x6D, "LD_L_L", IMP);

        Def(0xEA, "LD_MEM_A", ABS);
        Def(0xFA, "LD_A_MEM", ABS);

        Def(0xE0, "LDH_MEM_A", LDH);
        Def(0xF0, "LDH_A_MEM", LDH);
        Def(0xE2, "LD_C_MEM_A", IMP);
        Def(0xF2, "LD_A_MEM_C", IMP);

        Def(0x77, "LD_HL_A", IMP);
        Def(0x7E, "LD_A_HL", IMP);
        Def(0x70, "LD_HL_B", IMP); Def(0x71, "LD_HL_C", IMP); Def(0x72, "LD_HL_D", IMP); Def(0x73, "LD_HL_E", IMP); Def(0x74, "LD_HL_H", IMP); Def(0x75, "LD_HL_L", IMP);

        Def(0x36, "LD_HL_REF_IMM", IMM8);

        Def(0x12, "LD_DE_A", IMP);
        Def(0x1A, "LD_A_DE", IMP);
        Def(0x02, "LD_BC_A", IMP);
        Def(0x0A, "LD_A_BC", IMP);

        Def(0x22, "LDI_HL_A", IMP);
        Def(0x2A, "LDI_A_HL", IMP);
        Def(0x32, "LDD_HL_A", IMP);
        Def(0x3A, "LDD_A_HL", IMP);

        Def(0x01, "LD_BC_IMM", IMM16);
        Def(0x11, "LD_DE_IMM", IMM16);

        Def(0x21, "LD_HL_IMM", IMM16);

        Def(0x31, "LD_SP_IMM", IMM16);
        Def(0xF9, "LD_SP_HL", IMP);
        Def(0xF8, "LD_HL_SP_IMM", REL);
        Def(0xE8, "ADD_SP_IMM", REL);
        Def(0x08, "LD_MEM_SP", IMM16);

        Def(0xC5, "PUSH_BC", IMP); Def(0xC1, "POP_BC", IMP);
        Def(0xD5, "PUSH_DE", IMP); Def(0xD1, "POP_DE", IMP);
        Def(0xE5, "PUSH_HL", IMP); Def(0xE1, "POP_HL", IMP);
        Def(0xF5, "PUSH_AF", IMP); Def(0xF1, "POP_AF", IMP);

        Def(0x3C, "INC_A", IMP);
        Def(0x04, "INC_B", IMP); Def(0x0C, "INC_C", IMP);
        Def(0x14, "INC_D", IMP); Def(0x1C, "INC_E", IMP);
        Def(0x24, "INC_H", IMP); Def(0x2C, "INC_L", IMP);
        Def(0x34, "INC_HL_REF", IMP);

        Def(0x3D, "DEC_A", IMP);
        Def(0x05, "DEC_B", IMP); Def(0x0D, "DEC_C", IMP);
        Def(0x15, "DEC_D", IMP); Def(0x1D, "DEC_E", IMP);
        Def(0x25, "DEC_H", IMP); Def(0x2D, "DEC_L", IMP);
        Def(0x35, "DEC_HL_REF", IMP);

        Def(0x87, "ADD_A", IMP);
        Def(0x80, "ADD_B", IMP); Def(0x81, "ADD_C", IMP);
        Def(0x82, "ADD_D", IMP); Def(0x83, "ADD_E", IMP);
        Def(0x84, "ADD_H", IMP); Def(0x85, "ADD_L", IMP);
        Def(0x86, "ADD_HL_REF", IMP);
        Def(0xC6, "ADD_A_IMM", IMM8);

        Def(0x8F, "ADC_A", IMP);
        Def(0x88, "ADC_B", IMP); Def(0x89, "ADC_C", IMP);
        Def(0x8A, "ADC_D", IMP); Def(0x8B, "ADC_E", IMP);
        Def(0x8C, "ADC_H", IMP); Def(0x8D, "ADC_L", IMP);
        Def(0x8E, "ADC_HL_REF", IMP);
        Def(0xCE, "ADC_IMM", IMM8);

        Def(0x97, "SUB_A", IMP);
        Def(0x90, "SUB_B", IMP); Def(0x91, "SUB_C", IMP);
        Def(0x92, "SUB_D", IMP); Def(0x93, "SUB_E", IMP);
        Def(0x94, "SUB_H", IMP); Def(0x95, "SUB_L", IMP);
        Def(0x96, "SUB_HL_REF", IMP);
        Def(0xD6, "SUB_IMM", IMM8);

        Def(0x9F, "SBC_A", IMP);
        Def(0x98, "SBC_B", IMP); Def(0x99, "SBC_C", IMP);
        Def(0x9A, "SBC_D", IMP); Def(0x9B, "SBC_E", IMP);
        Def(0x9C, "SBC_H", IMP); Def(0x9D, "SBC_L", IMP);
        Def(0x9E, "SBC_HL_REF", IMP);
        Def(0xDE, "SBC_IMM", IMM8);

        Def(0xA7, "AND_A", IMP);
        Def(0xA0, "AND_B", IMP); Def(0xA1, "AND_C", IMP);
        Def(0xA2, "AND_D", IMP); Def(0xA3, "AND_E", IMP);
        Def(0xA4, "AND_H", IMP); Def(0xA5, "AND_L", IMP);
        Def(0xA6, "AND_HL_REF", IMP);
        Def(0xE6, "AND_IMM", IMM8);

        Def(0xB7, "OR_A", IMP);
        Def(0xB0, "OR_B", IMP); Def(0xB1, "OR_C", IMP);
        Def(0xB2, "OR_D", IMP); Def(0xB3, "OR_E", IMP);
        Def(0xB4, "OR_H", IMP); Def(0xB5, "OR_L", IMP);
        Def(0xB6, "OR_HL_REF", IMP);
        Def(0xF6, "OR_IMM", IMM8);

        Def(0xAF, "XOR_A", IMP);
        Def(0xA8, "XOR_B", IMP); Def(0xA9, "XOR_C", IMP);
        Def(0xAA, "XOR_D", IMP); Def(0xAB, "XOR_E", IMP);
        Def(0xAC, "XOR_H", IMP); Def(0xAD, "XOR_L", IMP);
        Def(0xAE, "XOR_HL_REF", IMP);
        Def(0xEE, "XOR_IMM", IMM8);

        Def(0xBF, "CP_A", IMP);
        Def(0xB8, "CP_B", IMP); Def(0xB9, "CP_C", IMP);
        Def(0xBA, "CP_D", IMP); Def(0xBB, "CP_E", IMP);
        Def(0xBC, "CP_H", IMP); Def(0xBD, "CP_L", IMP);
        Def(0xBE, "CP_HL_REF", IMP);
        Def(0xFE, "CP_IMM", IMM8);

        Def(0x03, "INC_BC", IMP); Def(0x0B, "DEC_BC", IMP);
        Def(0x13, "INC_DE", IMP); Def(0x1B, "DEC_DE", IMP);
        Def(0x23, "INC_HL", IMP); Def(0x2B, "DEC_HL", IMP);
        Def(0x33, "INC_SP", IMP); Def(0x3B, "DEC_SP", IMP);

        Def(0x09, "ADD_HL_BC", IMP);
        Def(0x19, "ADD_HL_DE", IMP);
        Def(0x29, "ADD_HL_HL", IMP);
        Def(0x39, "ADD_HL_SP", IMP);

        Def(0x07, "RLCA", IMP); Def(0x17, "RLA", IMP);
        Def(0x0F, "RRCA", IMP); Def(0x1F, "RRA", IMP);
        Def(0xCB, "PREFIX_CB", IMP);

        Def(0x2F, "CPL", IMP);
        Def(0x3F, "CCF", IMP);
        Def(0x37, "SCF", IMP);
        Def(0x27, "DAA", IMP);
    }

    static void Def(int opcode, string mnemonic, int format)
    {
        Mnemonics[opcode] = mnemonic;
        OperandFormats[opcode] = format;
    }

    public static readonly string[] ShortJumpInstructions = new string[]
    {
        "JR", "JR_NZ", "JR_Z", "JR_NC", "JR_C"
    };
}