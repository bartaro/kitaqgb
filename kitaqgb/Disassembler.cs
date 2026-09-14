// Disassembler.cs (bank-aware / full ROM / streaming)
using System;
using System.IO;
using System.Text;

// Produce a linear SM83 byte listing in independent 16 KiB bank windows.
// No code/data discovery, symbol recovery or runtime bank mapping is performed.
static class Disassembler
{
    // GB-compatible ROM banks are 16KB.
    const int BankSize = 0x4000;
    static readonly string[] CbRegs = new string[] { "B", "C", "D", "E", "H", "L", "[HL]", "A" };

    // Split the extended opcode into operation group, bit/rotate selector and register index.
    static string DecodeCbMnemonic(byte cbOpcode)
    {
        int group = (cbOpcode >> 6) & 0x03;
        int y = (cbOpcode >> 3) & 0x07;
        int z = cbOpcode & 0x07;
        string r = CbRegs[z];

        if (group == 0)
        {
            switch (y)
            {
                case 0: return "RLC " + r;
                case 1: return "RRC " + r;
                case 2: return "RL " + r;
                case 3: return "RR " + r;
                case 4: return "SLA " + r;
                case 5: return "SRA " + r;
                case 6: return "SWAP " + r;
                default: return "SRL " + r;
            }
        }

        if (group == 1) return string.Format("BIT {0},{1}", y, r);
        if (group == 2) return string.Format("RES {0},{1}", y, r);
        return string.Format("SET {0},{1}", y, r);
    }

    // Read the ROM into memory and stream the listing to dis.s in the configured debug directory.
    // I/O failures propagate to the caller; existing output is replaced.
    public static void Disassemble(string programPath)
    {
        byte[] rom = File.ReadAllBytes(programPath);

        // Always write disassembly as a stream (avoids huge StringBuilder and truncation/OOM).
        Directory.CreateDirectory(Program.DebugOutputPath);
        string outPath = Path.Combine(Program.DebugOutputPath, "dis.s");

        // UTF-8 without BOM, so it's diff-friendly.
        using (var sw = new StreamWriter(outPath, false, new UTF8Encoding(false)))
        {
            sw.WriteLine("; GB-compatible ROM Disassembly (full) ");
            sw.WriteLine("; File: " + Path.GetFileName(programPath));
            sw.WriteLine("; ROM size: " + rom.Length + " bytes (" + (rom.Length / 1024) + " KB)");
            sw.WriteLine();

            int bankCount = (rom.Length + (BankSize - 1)) / BankSize;

            for (int bank = 0; bank < bankCount; bank++)
            {
                int bankStart = bank * BankSize;
                int bankEnd = Math.Min(bankStart + BankSize, rom.Length);

                // Bank zero starts at CPU address zero; every later file bank is displayed at 0x4000 independently.
                int cpuBase = (bank == 0) ? 0x0000 : 0x4000;
                int cpuAddr = cpuBase;

                sw.WriteLine("; ------------------------------------------------------------");
                sw.WriteLine(string.Format("; BANK {0}  file_off=${1:X6}  size=${2:X4}  cpu=${3:X4}-{4:X4}",
                    bank,
                    bankStart,
                    (bankEnd - bankStart),
                    cpuBase,
                    cpuBase + (bankEnd - bankStart) - 1));
                sw.WriteLine("; ------------------------------------------------------------");

                int off = bankStart;
                while (off < bankEnd)
                {
                    int currentCpuAddr = cpuAddr;
                    int currentOff = off;

                    byte opcode = rom[off++];

                    if (opcode == 0xCB)
                    {
                        // A missing extended opcode is displayed as FF, so the listing can include a padded byte absent from the input.
                        byte cbOpcode = 0xFF;
                        if (off < bankEnd)
                            cbOpcode = rom[off++];

                        sw.Write(string.Format("{0:X4}    {1:X2} {2:X2}    ", currentCpuAddr, opcode, cbOpcode));
                        sw.Write(DecodeCbMnemonic(cbOpcode));
                        sw.Write(string.Format("    ; file_off=${0:X6}", currentOff));
                        sw.WriteLine();

                        cpuAddr += 2;

                        if (bank == 0 && cpuAddr > 0x4000) break;
                        if (bank != 0 && cpuAddr > 0x8000) break;
                        continue;
                    }

                    string mnem = AsmInfo.Mnemonics[opcode];
                    int format = AsmInfo.OperandFormats[opcode];
                    int operandSize = AsmInfo.OperandSizes[format];
                    string formatStr = AsmInfo.OperandFormatStrings[format];

                    // Print: CPUADDR OP [IMM..] MNEMONIC...
                    // Also print file offset as a comment for banked debugging.
                    sw.Write(string.Format("{0:X4}    {1:X2}", currentCpuAddr, opcode));

                    // Read operands (do not cross the bank boundary; pad with FF if missing).
                    int operandVal = 0;
                    byte imm0 = 0xFF;
                    byte imm1 = 0xFF;

                    for (int i = 0; i < 2; i++)
                    {
                        if (i < operandSize)
                        {
                            byte imm;
                            if (off < bankEnd)
                            {
                                imm = rom[off++];
                            }
                            else
                            {
                                imm = 0xFF;
                            }

                            if (i == 0) { imm0 = imm; operandVal = imm; }
                            else { imm1 = imm; operandVal |= (imm << 8); }

                            sw.Write(string.Format(" {0:X2}", imm));
                        }
                        else
                        {
                            sw.Write("   ");
                        }
                    }

                    sw.Write("    ");

                    if (mnem == "???")
                    {
                        sw.Write("DB $" + opcode.ToString("X2"));
                    }
                    else
                    {
                        // The current formatter treats every REL entry as PC-relative, including the table's signed SP-offset forms.
                        // For those forms this display is not a faithful operand; assembly is handled separately.
                        if (format == AsmInfo.REL)
                        {
                            // JR target: PC after instruction (opcode+imm) + rel
                            sbyte rel = (sbyte)imm0;
                            int target = currentCpuAddr + 2 + rel;

                            // Mark if the target leaves the current bank mapping window.
                            bool outOfWindow = false;
                            if (bank == 0)
                            {
                                if (target < 0x0000 || target > 0x3FFF) outOfWindow = true;
                            }
                            else
                            {
                                if (target < 0x4000 || target > 0x7FFF) outOfWindow = true;
                            }

                            if (outOfWindow)
                                sw.Write(string.Format("{0} ${1:X4} ({2}) ; out-of-bank", mnem, target & 0xFFFF, rel));
                            else
                                sw.Write(string.Format("{0} ${1:X4} ({2})", mnem, target & 0xFFFF, rel));
                        }
                        else
                        {
                            sw.Write(string.Format(mnem + formatStr, operandVal));
                        }
                    }

                    // File offset breadcrumb
                    sw.Write(string.Format("    ; file_off=${0:X6}", currentOff));
                    sw.WriteLine();

                    // Advance CPU address by instruction length (opcode + operand bytes)
                    cpuAddr += 1 + operandSize;

                    // Stop if we exceed the mapping window for this bank (safety)
                    if (bank == 0 && cpuAddr > 0x4000) break;
                    if (bank != 0 && cpuAddr > 0x8000) break;
                }

                sw.WriteLine();
            }
        }
    }
}




