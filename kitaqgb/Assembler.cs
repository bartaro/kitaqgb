using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

class Assembler
{
    public static AssemblerAnalysisReport LastReport
    {
        get { return Program.CurrentAssemblerLastReport; }
        private set { Program.CurrentAssemblerLastReport = value; }
    }

    Dictionary<string, AsmSymbol> Symbols = new Dictionary<string, AsmSymbol>();
    List<Fixup> Fixups = new List<Fixup>();
    DebugExporter Debug = new DebugExporter();

    const int BankSize = 0x4000;
    // Code starts immediately after the entry stub at 0x0150.
    // (Keep this in sync with the bytes we write into rom[0x150..].)
    const int CodeStart = 0x0160;
    const int MinRomSize = 0x8000; // at least bank0+bank1
    const int MaxRomSize = 0x800000; // 8MB (512 banks), max on real hardware with MBC5

    static byte GuessDefaultCartType(int romSize)
    {
        return (byte)(romSize > MinRomSize ? 0x19 : 0x00);
    }

    static byte GuessRomSizeCode(int romSize)
    {
        int[] sizes = new int[]
        {
            32 * 1024,
            64 * 1024,
            128 * 1024,
            256 * 1024,
            512 * 1024,
            1024 * 1024,
            2 * 1024 * 1024,
            4 * 1024 * 1024,
            8 * 1024 * 1024,
        };
        byte[] codes = new byte[] { 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08 };
        for (int i = 0; i < sizes.Length; i++)
        {
            if (romSize <= sizes[i]) return codes[i];
        }
        return 0x08;
    }

    static int CpuAddrFromFileOffset(int fileOffset)
    {
        if (fileOffset < 0) return fileOffset;
        if (fileOffset < BankSize) return fileOffset;
        return BankSize + (fileOffset & (BankSize - 1));
    }

    static int CpuOfFileOffset(int pc)
    {
        // Debug map helper: map file offset to a CPU-visible address.
        return CpuAddrFromFileOffset(pc);
    }

    static MemoryRegion InferRegionFromCpuAddress(int address)
    {
        if (address >= 0xFF80 && address <= 0xFFFE) return MemoryRegion.HighMem;
        if (address >= 0xFE00 && address <= 0xFE9F) return MemoryRegion.Oam;
        if (address >= 0xD000 && address <= 0xDFFF) return MemoryRegion.WramX;
        if (address >= 0xC000 && address <= 0xCFFF) return MemoryRegion.Wram0;
        if (address >= 0x0000 && address <= 0x7FFF) return MemoryRegion.ProgramRom;
        return MemoryRegion.Ram;
    }

    static string RegionBaseName(MemoryRegion region, int address)
    {
        if (region.Tag == MemoryRegionTag.ProgramRom) return "ROM";
        if (region.Tag == MemoryRegionTag.HighMem) return "HRAM";
        if (region.Tag == MemoryRegionTag.Oam) return "OAM";
        if (region.Tag == MemoryRegionTag.Wram0) return "WRAM0";
        if (region.Tag == MemoryRegionTag.WramX) return "WRAMX";

        if (address >= 0xFF00 && address <= 0xFF7F) return "IO";
        if (address >= 0xFF80 && address <= 0xFFFE) return "HRAM";
        if (address >= 0xFE00 && address <= 0xFE9F) return "OAM";
        if (address >= 0xD000 && address <= 0xDFFF) return "WRAMX";
        if (address >= 0xC000 && address <= 0xCFFF) return "WRAM0";
        if (address >= 0xA000 && address <= 0xBFFF) return "SRAM";
        if (address >= 0x8000 && address <= 0x9FFF) return "VRAM";
        if (address >= 0x0000 && address <= 0x7FFF) return "ROM";

        if (region.Tag == MemoryRegionTag.Fixed) return string.Format("FIXED(${0:X4})", region.FixedAddress);
        return region.ToString().ToUpperInvariant();
    }

    static string RegionName(MemoryRegion region, int address)
    {
        if (region.Tag == MemoryRegionTag.WramX && region.WramBank > 0)
            return string.Format("WRAMX[{0}]", region.WramBank);
        return RegionBaseName(region, address);
    }

    static int? LogicalBank(MemoryRegion region, int value)
    {
        if (region.Tag == MemoryRegionTag.ProgramRom)
        {
            if (value < 0) return null;
            return value >> 14;
        }

        if (region.Tag == MemoryRegionTag.WramX)
        {
            if (region.WramBank > 0) return region.WramBank;
            if (value >= 0xD000 && value <= 0xDFFF) return 1;
        }

        return null;
    }

    static int RegionOffset(MemoryRegion region, int value)
    {
        if (region.Tag == MemoryRegionTag.ProgramRom) return value & 0x3FFF;
        if (value >= 0xFF80 && value <= 0xFFFE) return value - 0xFF80;
        if (value >= 0xFF00 && value <= 0xFF7F) return value - 0xFF00;
        if (value >= 0xFE00 && value <= 0xFE9F) return value - 0xFE00;
        if (value >= 0xD000 && value <= 0xDFFF) return value - 0xD000;
        if (value >= 0xC000 && value <= 0xCFFF) return value - 0xC000;
        if (value >= 0xA000 && value <= 0xBFFF) return value - 0xA000;
        if (value >= 0x8000 && value <= 0x9FFF) return value - 0x8000;
        if (value >= 0x0000 && value <= 0x7FFF) return value;
        return value & 0xFFFF;
    }

    static int CpuAddressOf(AsmSymbol symbol)
    {
        if (symbol == null) return 0;
        if (symbol.Region.Tag == MemoryRegionTag.ProgramRom) return CpuAddrFromFileOffset(symbol.Value);
        return symbol.Value & 0xFFFF;
    }

    static bool IsBankBoundarySkip(int skipTarget)
    {
        return skipTarget != 0 && (skipTarget & (BankSize - 1)) == 0;
    }

    static bool ShouldIgnoreBackwardBankBoundarySkip(int pc, int skipTarget)
    {
        if (!IsBankBoundarySkip(skipTarget)) return false;
        return pc >= skipTarget;
    }
    public static string Assemble(IReadOnlyList<Expr> assembly, string outputFilename)
    {
        Assembler assembler = new Assembler();
        return assembler.Run(assembly, outputFilename);
    }

    string Run(IReadOnlyList<Expr> assembly, string outputFilename)
    {
        // Jump relaxation (JP -> JR) to reduce code size.
        // Monotonic transform: once a JP is proven in-range for JR, it stays in-range.
        var relaxedAssembly = RelaxJumps(assembly);

        // Reorder by $section markers so that same-named sections are emitted together.
        // This is a lightweight "link-unit" section mechanism used to control output order.
        // Default behavior:
        // - Within each ROM bank, code/data in named sections are buffered and emitted at the end of the bank.
        // - The default (unnamed) section is emitted first.
        // - The order of named sections is the order of first appearance.
        // This is intentionally conservative: we keep $skip_to/$align in-place (they remain in the default section)
        // so bank layout barriers aren't reordered.
        var sectionedAssembly = ReorderSections(relaxedAssembly);
        sectionedAssembly = InsertAutoBankBoundarySkips(sectionedAssembly);
        if (Program.EnableDebugOutput)
            Program.WritePassOutputToFile("assembly_sectioned", Program.ShowAssemblyPublic(sectionedAssembly));

        // Collect RST vector mapping directives emitted by codegen/optimizer.
        // Format: ($rst_map <vector:int> <targetLabel:string>)
        Dictionary<int, string> rstVectorTargets = new Dictionary<int, string>();
        bool hasScrollVblankVector = false;
        bool hasScrollStatVector = false;
        foreach (Expr e in sectionedAssembly)
        {
            int v; string target;
            if (e.Match(Tag.RstMap, out v, out target))
            {
                // Only accept valid RST vectors.
                if (v < 0x00 || v > 0x38 || (v & 0x07) != 0)
                {
                    Program.Error($"Ignoring invalid $rst_map vector: {v:X2} (target {target})");
                    continue;
                }
                rstVectorTargets[v] = target;
            }
            else if (e.Match(Tag.Function, out string funcLabel))
            {
                if (funcLabel == "__kq_vblank_vector") hasScrollVblankVector = true;
                else if (funcLabel == "__kq_stat_vector") hasScrollStatVector = true;
            }
        }

        int requiredMaxPc = EstimateMaxPcForOutput(sectionedAssembly);
        int romSize = Math.Max(MinRomSize, RoundUp(requiredMaxPc, BankSize));
        if (romSize > MaxRomSize)
            Program.Panic($"assembler: program requires ROM larger than supported maximum ({romSize} bytes > {MaxRomSize} bytes)");

        byte[] rom = new byte[romSize];
        List<string> comments = new List<string>();
        for (int i = 0; i < rom.Length; i++) rom[i] = 0xFF;

        // RST vectors (0x00..0x38)
        // NOTE: Interrupt vectors are at 0x40..0x60 and must NOT be overwritten.
        for (int v = 0x00; v <= 0x38; v += 0x08)
        {
            // Default: JP $0150 (entry stub; real entry is at 0x0100)
            rom[v] = 0xC3;
            rom[v + 1] = 0x50;
            rom[v + 2] = 0x01;

            // If the optimizer selected a hot function for this RST vector,
            // patch the stub to JP <targetLabel>.
            if (rstVectorTargets.TryGetValue(v, out string targetLabel))
            {
                Fixups.Add(new Fixup
                {
                    Operand = new AsmOperand(targetLabel, AddressMode.Absolute),
                    Location = v + 1,
                    Mode = AddressMode.Absolute,
                    IsRelativeBranch = false,
                    IsWordDefinition = false,
                    Size = 2
                });
                // Placeholder bytes (will be filled by fixup)
                rom[v + 1] = 0x00;
                rom[v + 2] = 0x00;
            }

            // Fill the rest of the 8-byte slot with NOP (0x00) for safety.
            for (int i = 3; i < 8; i++) rom[v + i] = 0x00;
        }

        // Interrupt vectors (0x40..0x60)
        // Default to RETI so that enabling IE/IME won't crash even if the program
        // doesn't install handlers yet. Users can still jump to their own handlers
        // manually if they want full control.
        for (int v = 0x40; v <= 0x60; v += 0x08)
        {
            rom[v] = 0xD9; // RETI
            for (int i = 1; i < 8; i++) rom[v + i] = 0x00;
        }

        if (hasScrollVblankVector)
        {
            rom[0x40] = 0xC3; // JP a16
            rom[0x41] = 0x00;
            rom[0x42] = 0x00;
            Fixups.Add(new Fixup
            {
                Operand = new AsmOperand("__kq_vblank_vector", AddressMode.Absolute),
                Location = 0x41,
                Mode = AddressMode.Absolute,
                IsRelativeBranch = false,
                IsWordDefinition = false,
                Size = 2
            });
            for (int i = 3; i < 8; i++) rom[0x40 + i] = 0x00;
        }
        else
        {
            // VBlank vector (0x0040): X-style frame flag set for HALT wait loops.
            // 0x0040: PUSH AF; LD A,1; LD [$C29C],A; POP AF; RETI
            rom[0x40] = 0xF5; // PUSH AF
            rom[0x41] = 0x3E; // LD A,imm8
            rom[0x42] = 0x01; // imm8 = 1
            rom[0x43] = 0xEA; // LD (a16),A
            rom[0x44] = 0x9C; // low($C29C)
            rom[0x45] = 0xC2; // high($C29C)
            rom[0x46] = 0xF1; // POP AF
            rom[0x47] = 0xD9; // RETI
        }

        if (hasScrollStatVector)
        {
            rom[0x48] = 0xC3; // JP a16
            rom[0x49] = 0x00;
            rom[0x4A] = 0x00;
            Fixups.Add(new Fixup
            {
                Operand = new AsmOperand("__kq_stat_vector", AddressMode.Absolute),
                Location = 0x49,
                Mode = AddressMode.Absolute,
                IsRelativeBranch = false,
                IsWordDefinition = false,
                Size = 2
            });
            for (int i = 3; i < 8; i++) rom[0x48 + i] = 0x00;
        }

        // Entry Point
        rom[0x100] = 0x00;
        rom[0x101] = 0xC3;
        rom[0x102] = 0x50;
        rom[0x103] = 0x01;

        // Header
        RomHeaderPatcher.WriteHeaderLogo(rom, Program.ShouldEmitHeaderLogo());
        byte[] titleBytes = Encoding.ASCII.GetBytes("KITAQGB");
        for (int i = 0; i < 16; i++) rom[0x134 + i] = (i < titleBytes.Length) ? titleBytes[i] : (byte)0;
        rom[0x143] = 0x00;
        // Provisional header values are derived from the assembled ROM footprint so the
        // raw output remains bankable even before the later header-patch pass runs.
        rom[0x147] = GuessDefaultCartType(romSize);
        rom[0x148] = GuessRomSizeCode(romSize);
        rom[0x149] = 0x00;

        byte checksum = 0;
        for (int i = 0x134; i <= 0x14C; i++) checksum = (byte)(checksum - rom[i] - 1);
        rom[0x14D] = checksum;
        rom[0x14E] = 0x00;
        rom[0x14F] = 0x00;

        // Minimal startup (A6): always initialize SP, then select the ROM bank that contains 'main', then CALL it.
        // This makes the generated ROM robust even if 'main' spills into bank2+.
        // 0x0150:
        // DI
        // LD SP,<effective stack top>
        // LD A,BANK(main)
        // LD (0x2000),A ; MBC bank select (low 8-bit)
        // LDH (0x82),A ; __rom_bank mirror (HRAM)
        // CALL main
        // JR $-2 ; spin
        int stackTop = Program.EffectiveStackTop & 0xFFFF;
        rom[0x150] = 0xF3; // DI
        rom[0x151] = 0x31; // LD SP,d16
        rom[0x152] = (byte)(stackTop & 0xFF); // lo
        rom[0x153] = (byte)((stackTop >> 8) & 0xFF); // hi
        rom[0x154] = 0x3E; // LD A,d8
        Fixups.Add(new Fixup
        {
            Operand = new AsmOperand("main", ImmediateModifier.Bank),
            Location = 0x155,
            Mode = AddressMode.Immediate,
            IsRelativeBranch = false,
            IsWordDefinition = false,
            Size = 1
        });
        rom[0x156] = 0xEA; // LD (a16),A
        rom[0x157] = 0x00; // lo (0x2000)
        rom[0x158] = 0x20; // hi
        rom[0x159] = 0xE0; // LDH (a8),A
        rom[0x15A] = 0x82; // a8 = __rom_bank mirror (0xFF82)
        rom[0x15B] = 0xCD; // CALL a16
        Fixups.Add(new Fixup
        {
            Operand = new AsmOperand("main", AddressMode.Absolute),
            Location = 0x15C,
            Mode = AddressMode.Absolute,
            IsRelativeBranch = false,
            IsWordDefinition = false,
            Size = 2
        });
        rom[0x15E] = 0x18; // JR r8
        rom[0x15F] = 0xFE; // -2

        int pc = CodeStart;
        // Bank size is 16KB on the GB-compatible target. We emit a simple per-bank size report.
        int[] bankMaxPc = new int[(romSize + BankSize - 1) / BankSize];
        var functionSizes = new List<FunctionSizeInfo>();
        var symbolMetadata = new List<SymbolMetadataInfo>();
        var variableMetadata = new List<VariableMetadataInfo>();
        var sourceLocations = new List<SourceLocationMetadataInfo>();
        string currentFunctionName = null;
        string currentFunctionSection = "";
        string currentSectionName = "";
        int currentFunctionStart = 0;
        int currentFunctionBytes = 0;

        void UpdateBankMax(int newPc)
        {
            if (newPc < 0) return;
            int bank = newPc / BankSize;
            if (bank < 0) return;
            if (bank >= bankMaxPc.Length) Program.Panic("assembler: PC beyond allocated ROM buffer");
            if (newPc > bankMaxPc[bank]) bankMaxPc[bank] = newPc;
        }

        void FinalizeCurrentFunction()
        {
            if (string.IsNullOrEmpty(currentFunctionName)) return;
            if ((currentFunctionStart >> 14) != ((currentFunctionStart + Math.Max(0, currentFunctionBytes - 1)) >> 14) &&
                currentFunctionBytes > 0)
            {
                Program.Error("assembler: function '{0}' crosses a ROM bank boundary (start=${1:X5}, size={2} bytes)",
                    currentFunctionName, currentFunctionStart, currentFunctionBytes);
            }
            functionSizes.Add(new FunctionSizeInfo
            {
                Name = currentFunctionName,
                StartFileOffset = currentFunctionStart,
                EndFileOffset = currentFunctionStart + currentFunctionBytes,
                SizeBytes = currentFunctionBytes,
                Bank = currentFunctionStart >> 14,
                CpuAddress = CpuAddrFromFileOffset(currentFunctionStart),
                Section = string.IsNullOrEmpty(currentFunctionSection) ? null : currentFunctionSection
            });
            currentFunctionName = null;
            currentFunctionSection = "";
            currentFunctionStart = 0;
            currentFunctionBytes = 0;
        }

        void TrackSymbolMetadata(string name, int address, bool isLabel, MemoryRegion region, string section)
        {
            int cpuAddress = region.Tag == MemoryRegionTag.ProgramRom ? CpuAddrFromFileOffset(address) : (address & 0xFFFF);
            int bank = LogicalBank(region, address) ?? 0;
            symbolMetadata.Add(new SymbolMetadataInfo
            {
                Name = name,
                Bank = bank,
                Start = cpuAddress & 0xFFFF,
                End = (cpuAddress & 0xFFFF) + 1,
                Kind = isLabel ? "label" : "symbol",
                Region = RegionName(region, cpuAddress),
                Section = string.IsNullOrEmpty(section) ? null : section,
            });
        }

        void TrackVariableMetadata(string name, int address, int size, MemoryRegion region)
        {
            variableMetadata.Add(new VariableMetadataInfo
            {
                Name = name,
                Address = address & 0xFFFF,
                Size = size,
                Region = RegionName(region, address),
                Bank = LogicalBank(region, address),
            });
        }

        void TrackSourceLocation(Expr expr, int address, string symbol, string section)
        {
            if (expr == null) return;
            if (expr.Source.Filename == null || expr.Source.Filename == FilePosition.Unknown.Filename) return;
            if (expr.Source.Line < 0) return;

            sourceLocations.Add(new SourceLocationMetadataInfo
            {
                Bank = address < BankSize ? 0 : (address >> 14),
                Address = CpuAddrFromFileOffset(address) & 0xFFFF,
                Path = expr.Source.Filename,
                Line = expr.Source.Line + 1,
                Column = expr.Source.Column >= 0 ? (int?)(expr.Source.Column + 1) : null,
                Symbol = string.IsNullOrEmpty(symbol) ? null : symbol,
                Section = string.IsNullOrEmpty(section) ? null : section,
            });
        }

        void EnsureSpanFitsRomBank(int startPc, int size, string description)
        {
            if (size <= 0) return;
            if ((startPc >> 14) == ((startPc + size - 1) >> 14)) return;
            Program.Error("assembler: {0} crosses a ROM bank boundary (start=${1:X5}, size={2} bytes)",
                description, startPc, size);
        }

        UpdateBankMax(pc);
        foreach (Expr e in sectionedAssembly)
        {
            string rdName; byte[] rdBytes;
            if (e.Match(Tag.ReadonlyData, out rdName, out rdBytes))
            {
                EnsureSpanFitsRomBank(pc, rdBytes.Length, "readonly data '" + rdName + "'");
                DefineSymbol(rom, rdName, pc, isLabel: false, MemoryRegion.ProgramRom, currentSectionName);
                Debug.AddVariable(rdName, CpuOfFileOffset(pc), rdBytes.Length, MemoryRegion.ProgramRom, pc >> 14);
                TrackSymbolMetadata(rdName, pc, false, MemoryRegion.ProgramRom, currentSectionName);
                TrackVariableMetadata(rdName, CpuOfFileOffset(pc), rdBytes.Length, MemoryRegion.ProgramRom);
                TrackSourceLocation(e, pc, currentFunctionName ?? rdName, currentSectionName);
                if (pc + rdBytes.Length <= rom.Length) Array.Copy(rdBytes, 0, rom, pc, rdBytes.Length);
                else Program.Error("Not enough ROM space for data: " + rdName);
                pc += rdBytes.Length;
                UpdateBankMax(pc);
                continue;
            }

            string label, name, mnemonic, text;
            int skipTarget, address, size;
            AsmOperand operand;

            if (e.Match(Tag.Comment, out text))
            {
                comments.Add(text);
            }
            else if (e.Match(Tag.RstMap, out int rstV, out string rstTarget))
            {
                // Already handled before ROM init.
                continue;
            }
            else if (e.Match(Tag.Function, out label))
            {
                FinalizeCurrentFunction();
                DefineSymbol(rom, label, pc, isLabel: false, MemoryRegion.ProgramRom, currentSectionName);
                Debug.AddFunction(label, CpuOfFileOffset(pc), pc >> 14);
                TrackSymbolMetadata(label, pc, false, MemoryRegion.ProgramRom, currentSectionName);
                currentFunctionName = label;
                currentFunctionSection = currentSectionName;
                currentFunctionStart = pc;
                currentFunctionBytes = 0;
                // clean up local labels
                string[] labels = Symbols.Where(x => x.Value.IsLabel).Select(x => x.Key).ToArray();
                foreach (string key in labels) Symbols.Remove(key);
            }
            else if (e.Match(Tag.Label, out label))
            {
                DefineSymbol(rom, label, pc, isLabel: true, MemoryRegion.ProgramRom, currentSectionName);
                Debug.AddFunction(label, CpuOfFileOffset(pc), pc >> 14);
                TrackSymbolMetadata(label, pc, true, MemoryRegion.ProgramRom, currentSectionName);
            }
            else if (e.Match(Tag.SkipTo, out skipTarget))
            {
                // Codegen emits $skip_to 0 to mark bank0. Our PC already starts at CodeStart.
                if (skipTarget == 0)
                {
                    if (pc < CodeStart) pc = CodeStart;
                    UpdateBankMax(pc);
                    continue;
                }

                if (skipTarget < CodeStart) Program.Panic("assembler: skip address is too small");
                if (skipTarget > rom.Length) Program.Panic("assembler: skip address is too large");
                if (pc > skipTarget)
                {
                    if (ShouldIgnoreBackwardBankBoundarySkip(pc, skipTarget))
                    {
                        UpdateBankMax(pc);
                        continue;
                    }
                    Program.Panic("assembler: cannot skip backward during emit (pc=${0:X5}, skip=${1:X5})", pc, skipTarget);
                }

                int paddingSize = skipTarget - pc;
                if (paddingSize > 0)
                {
                    Program.WriteInfoLine(string.Format("{0} bytes padding inserted at {1:X4}", paddingSize, pc));
                    Array.Clear(rom, pc, paddingSize);
                }
                pc = skipTarget;
                UpdateBankMax(pc);
            }
            else if (e.Match(Tag.Align, out int alignBytes))
            {
                if (alignBytes <= 0 || (alignBytes & (alignBytes - 1)) != 0) Program.Panic("assembler: $align expects power-of-two positive integer");
                int newPc = (pc + (alignBytes - 1)) & ~(alignBytes - 1);
                if (newPc > rom.Length) Program.Panic("assembler: alignment advances PC beyond ROM size");
                int pad = newPc - pc;
                if (pad > 0) Array.Clear(rom, pc, pad);
                pc = newPc;
                UpdateBankMax(pc);
            }
            else if (e.Match(Tag.Section, out string sectName))
            {
                // Section markers are primarily for trace readability / organization.
                // The assembler itself is single-pass over a flat ROM image, so this is a no-op.
                // (We still accept it so codegen can emit $section and traces remain self-contained.)
                currentSectionName = sectName ?? "";
                continue;
            }
            else if (e.Match(Tag.Word, out label))
            {
                EnsureSpanFitsRomBank(pc, 2, "$word '" + label + "'");
                AsmSymbol sym;
                int val = 0;
                if (Symbols.TryGetValue(label, out sym)) val = CpuAddressOf(sym);
                else Fixups.Add(new Fixup { Operand = new AsmOperand(label, AddressMode.Immediate), Location = pc, Mode = AddressMode.Immediate, IsRelativeBranch = false, IsWordDefinition = true });

                if (pc + 2 <= rom.Length)
                {
                    rom[pc++] = LowByte(val);
                    rom[pc++] = HighByte(val);
                }
                else
                {
                    Program.Error("Not enough ROM space for $word: " + label);
                    pc += 2;
                }
                if (!string.IsNullOrEmpty(currentFunctionName)) currentFunctionBytes += 2;
                UpdateBankMax(pc);
            }
            else if (e.Match(Tag.Variable, out name, out address, out size, out MemoryRegion explicitRegion))
            {
                MemoryRegion region = explicitRegion;
                if (region.Tag == MemoryRegionTag.Ram || (region.Tag == MemoryRegionTag.WramX && region.WramBank == 0 && address >= 0xC000 && address <= 0xCFFF))
                    region = InferRegionFromCpuAddress(address);

                DefineSymbol(rom, name, address, isLabel: false, region, currentSectionName);
                Debug.AddVariable(name, address, size, region, LogicalBank(region, address));
                TrackSymbolMetadata(name, address, false, region, currentSectionName);
                TrackVariableMetadata(name, address, size, region);
            }
            else if (e.Match(Tag.Variable, out name, out address, out size))
            {
                MemoryRegion region = InferRegionFromCpuAddress(address);
                DefineSymbol(rom, name, address, isLabel: false, region, currentSectionName);
                Debug.AddVariable(name, address, size, region, LogicalBank(region, address));
                TrackSymbolMetadata(name, address, false, region, currentSectionName);
                TrackVariableMetadata(name, address, size, region);
            }
            else if (e.Match(Tag.Asm, out mnemonic, out operand))
            {
                // HRAM/IO short-form fix: allow codegen to use LD_A_MEM/LD_MEM_A for IO/HRAM.
                // Convert to LDH_* forms when operand fits 0xFF00-0xFFFF (encoded as a8).
                if (mnemonic == "LD_A_MEM" || mnemonic == "LD_MEM_A")
                {
                    if (operand.Mode == AddressMode.Immediate)
                    {
                        mnemonic = (mnemonic == "LD_A_MEM") ? "LDH_A_MEM" : "LDH_MEM_A";
                    }
                    else if (operand.Mode == AddressMode.Absolute)
                    {
                        // Immediate absolute constant 0xFFxx => LDH with a8.
                        if (!operand.Base.HasValue && ((operand.Offset & 0xFF00) == 0xFF00))
                        {
                            mnemonic = (mnemonic == "LD_A_MEM") ? "LDH_A_MEM" : "LDH_MEM_A";
                            operand = new AsmOperand(operand.Offset & 0xFF, AddressMode.Immediate);
                        }
                        else
                        {
                            // If the symbol resolves now and is 0xFFxx, also use LDH.
                            int resolved;
                            if (TryGetOperandValue(operand, AddressMode.Absolute, pc, false, out resolved) && ((resolved & 0xFF00) == 0xFF00))
                            {
                                mnemonic = (mnemonic == "LD_A_MEM") ? "LDH_A_MEM" : "LDH_MEM_A";
                                operand = new AsmOperand(resolved & 0xFF, AddressMode.Immediate);
                            }
                        }
                    }
                }

                bool isRelative = IsRelativeBranch(mnemonic);
                AddressMode actualMode = operand.Mode;

                if (isRelative && (actualMode == AddressMode.Absolute || actualMode == AddressMode.Immediate))
                {
                    actualMode = AddressMode.Relative;
                }

                // Optimization
                if (operand.Mode == AddressMode.AbsoluteX && operand.Offset < 256)
                {
                    // GB doesn't have a direct indexed FF00+n mode like this, but keeping the logic structure.
                    // actualMode = AddressMode.HighMemX;
                }

                int formalSize = 0;
                List<byte> candidates = new List<byte>();
                int operandFormat = ParseAddressMode(actualMode);

                int matchedFormat = -1;

                for (int opcode = 0; opcode < 256; opcode++)
                {
                    if (mnemonic == AsmInfo.Mnemonics[opcode])
                    {
                        int reqFormat = AsmInfo.OperandFormats[opcode];
                        bool match = (operandFormat == reqFormat);
                        if (!match && (reqFormat == AsmInfo.IMM16 || reqFormat == AsmInfo.IMM8 || reqFormat == AsmInfo.LDH))
                        {
                            if (operandFormat == AsmInfo.ABS || operandFormat == AsmInfo.IMM) match = true;
                            if (operandFormat == AsmInfo.IMM16 && reqFormat == AsmInfo.IMM16) match = true;
                        }
                        if (match)
                        {
                            candidates.Add((byte)opcode);
                            matchedFormat = reqFormat;
                        }
                    }
                }

                if (candidates.Count == 1)
                {
                    formalSize = AsmInfo.OperandSizes[matchedFormat];
                    int instructionSize = 1 + formalSize;
                    string owner = string.IsNullOrEmpty(currentFunctionName)
                        ? string.Format("instruction '{0}'", mnemonic)
                        : string.Format("instruction '{0}' in function '{1}'", mnemonic, currentFunctionName);
                    EnsureSpanFitsRomBank(pc, instructionSize, owner);

                    Debug.TagInstruction(pc, 1 + formalSize, comments);
                    TrackSourceLocation(e, pc, currentFunctionName, currentSectionName);
                    comments.Clear();
                    rom[pc++] = candidates[0];
                }
                else if (candidates.Count == 0) Program.Panic("invalid instruction: {0} {1}", mnemonic, operand.Show());
                else Program.Panic("ambiguous instruction: {0} {1}", mnemonic, operand.Show());

                int operandValue;
                if (!TryGetOperandValue(operand, actualMode, pc, isRelative, out operandValue))
                {
                    Fixups.Add(new Fixup { Operand = operand, Location = pc, Mode = actualMode, IsRelativeBranch = isRelative, Size = formalSize });
                    operandValue = 0;
                }

                if (formalSize == 1) rom[pc++] = LowByte(operandValue);
                else if (formalSize == 2) { rom[pc++] = LowByte(operandValue); rom[pc++] = HighByte(operandValue); }
                if (!string.IsNullOrEmpty(currentFunctionName)) currentFunctionBytes += 1 + formalSize;
                UpdateBankMax(pc);
            }
        }
        FinalizeCurrentFunction();
        DoFixups(rom);

        if (Fixups.Count > 0)
        {
            Console.Error.WriteLine("Error: Unresolved symbols remaining:");
            foreach (var fixup in Fixups)
            {
                if (fixup.Operand.Base.HasValue)
                    Console.Error.WriteLine("  - " + fixup.Operand.Base.Value + " at " + fixup.Location.ToString("X4"));
            }
            Program.Error("Assembly failed due to unresolved symbols.");
        }

        if (pc > rom.Length) Program.Error("Program size exceeds ROM size");
        Program.WriteInfoLine(string.Format("Assembly complete. Size: {0} bytes used (ROM={1} bytes).", pc, rom.Length));

        var buildReport = new AssemblerAnalysisReport
        {
            BankMaxPc = bankMaxPc.ToArray(),
            RomSizeBytes = rom.Length,
            UsedBytes = pc
        };
        foreach (var f in functionSizes) buildReport.FunctionSizes.Add(f);
        foreach (var s in symbolMetadata) buildReport.Symbols.Add(s);
        foreach (var v in variableMetadata) buildReport.Variables.Add(v);
        foreach (var s in sourceLocations
            .OrderBy(x => x.Bank)
            .ThenBy(x => x.Address)
            .ThenBy(x => x.Path, StringComparer.Ordinal)
            .ThenBy(x => x.Line)
            .ThenBy(x => x.Column ?? 0))
        {
            if (buildReport.SourceLocations.Any(existing =>
                existing.Bank == s.Bank &&
                existing.Address == s.Address &&
                existing.Path == s.Path &&
                existing.Line == s.Line &&
                existing.Column == s.Column &&
                existing.Symbol == s.Symbol))
                continue;
            buildReport.SourceLocations.Add(s);
        }
        LastReport = buildReport;

        int globalChecksum = 0;
        for (int i = 0; i < rom.Length; i++)
        {
            if (i != 0x14E && i != 0x14F) globalChecksum += rom[i];
        }
        rom[0x14E] = (byte)((globalChecksum >> 8) & 0xFF);
        rom[0x14F] = (byte)(globalChecksum & 0xFF);

        outputFilename = IoUtil.WriteAllBytesRobust(outputFilename, rom, allowAlternatePath: true);
        Program.RememberArtifactPath("output_rom", outputFilename);
        Debug.Save(Path.ChangeExtension(outputFilename, ".dbg"));

        // Always emit a symbol map and bank size report (vibe-friendly).
        try
        {
            WriteSymbolMap(Path.ChangeExtension(outputFilename, ".map"));
            WriteBankSizeReport(Path.ChangeExtension(outputFilename, ".banks.txt"), bankMaxPc, BankSize);
            WriteFunctionSizeReport(Path.ChangeExtension(outputFilename, ".funcsizes.txt"), functionSizes);
            WriteSourceMap(Path.ChangeExtension(outputFilename, ".source_map.txt"), buildReport.SourceLocations);

            // Optional variable list (vlist)
            if (Program.EmitVarList)
            {
                string vpath = Program.VarListOutputPath;
                if (string.IsNullOrEmpty(vpath)) vpath = Path.ChangeExtension(outputFilename, ".vlist.txt");
                WriteVarList(vpath);
            }
        }
        catch (Exception ex)
        {
            // Do not fail compilation due to report generation.
            Program.Warning("Failed to write symbol/bank report: " + ex.Message);
        }

        return outputFilename;
    }

    void WriteVarList(string filename)
    {
        var vars = Debug.GetVariables();
        var items = vars
            .OrderBy(v => v.Name, StringComparer.Ordinal)
            .ThenBy(v => v.Address)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("; KITAQGB variable list");
        sb.AppendLine("; Name  Region  Bank  Addr  Size");
        foreach (var v in items)
        {
            string bank = v.Bank.HasValue ? v.Bank.Value.ToString() : "";
            sb.AppendFormat("{0,-24} {1,-6} {2,4}  {3:X4}  {4,5}\n", v.Name, v.Region, bank, v.Address, v.Size);
        }
        filename = IoUtil.WriteAllTextUtf8Robust(filename, sb.ToString(), allowAlternatePath: true);
        Program.RememberArtifactPath("vlist", filename);
    }

    void WriteSymbolMap(string filename)
    {
        var items = Symbols
            .Select(kv => new
            {
                Name = kv.Key,
                Symbol = kv.Value,
                Address = CpuAddressOf(kv.Value),
                Bank = LogicalBank(kv.Value.Region, kv.Value.Value),
                Offset = RegionOffset(kv.Value.Region, kv.Value.Value),
                Kind = kv.Value.IsLabel ? "L" : "S",
                Region = RegionName(kv.Value.Region, CpuAddressOf(kv.Value))
            })
            .OrderBy(x => x.Region, StringComparer.Ordinal)
            .ThenBy(x => x.Bank ?? -1)
            .ThenBy(x => x.Address)
            .ThenBy(x => x.Name, StringComparer.Ordinal)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("; KITAQGB symbol map");
        sb.AppendLine("; Addr(CPU) Bank  Off   Kind Region Name");
        foreach (var it in items)
        {
            string bank = (it.Bank ?? 0).ToString();
            sb.AppendFormat("{0:X4}  {1,4}  {2:X4}   {3}    {4,-8} {5}\n", it.Address, bank, it.Offset & 0xFFFF, it.Kind, it.Region, it.Name);
        }
        filename = IoUtil.WriteAllTextUtf8Robust(filename, sb.ToString(), allowAlternatePath: true);
        Program.RememberArtifactPath("map", filename);
    }

    static void WriteBankSizeReport(string filename, int[] bankMaxPc, int bankSize)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB bank size report");
        sb.AppendLine("# bank, start, end, used, free");

        for (int b = 0; b < bankMaxPc.Length; b++)
        {
            int start = b * bankSize;
            int end = (b + 1) * bankSize;
            int maxPc = bankMaxPc[b];
            int used = Math.Max(0, Math.Min(maxPc, end) - start);
            int total = end - start;
            int free = Math.Max(0, total - used);
            sb.AppendFormat("{0,2}, 0x{1:X4}, 0x{2:X4}, {3,5}, {4,5}\n", b, start, end, used, free);
        }

        filename = IoUtil.WriteAllTextUtf8Robust(filename, sb.ToString(), allowAlternatePath: true);
        Program.RememberArtifactPath("source_map", filename);
    }

    static void WriteFunctionSizeReport(string filename, List<FunctionSizeInfo> functionSizes)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB function size report");
        sb.AppendLine("# name, bank, cpu_addr, start_file, end_file, size");

        foreach (var f in (functionSizes ?? new List<FunctionSizeInfo>()).OrderByDescending(x => x.SizeBytes).ThenBy(x => x.Name, StringComparer.Ordinal))
        {
            sb.AppendFormat("{0}, {1}, 0x{2:X4}, 0x{3:X5}, 0x{4:X5}, {5}\n",
                f.Name, f.Bank, f.CpuAddress, f.StartFileOffset, f.EndFileOffset, f.SizeBytes);
        }

        filename = IoUtil.WriteAllTextUtf8Robust(filename, sb.ToString(), allowAlternatePath: true);
        Program.RememberArtifactPath("banks_txt", filename);
    }

    static void WriteSourceMap(string filename, IReadOnlyList<SourceLocationMetadataInfo> sourceLocations)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB source map");
        sb.AppendLine("# bank:addr path:line:column symbol=... section=...");

        foreach (var info in sourceLocations ?? Array.Empty<SourceLocationMetadataInfo>())
        {
            if (string.IsNullOrEmpty(info.Path) || info.Line <= 0) continue;

            sb.AppendFormat(
                "{0:X2}:{1:X4} {2}:{3}",
                info.Bank & 0xFFFF,
                info.Address & 0xFFFF,
                info.Path,
                info.Line);
            if (info.Column.HasValue) sb.AppendFormat(":{0}", info.Column.Value);
            if (!string.IsNullOrEmpty(info.Symbol))
                sb.Append(" symbol=").Append(info.Symbol);
            if (!string.IsNullOrEmpty(info.Section))
                sb.Append(" section=").Append(info.Section);
            sb.AppendLine();
        }

        filename = IoUtil.WriteAllTextUtf8Robust(filename, sb.ToString(), allowAlternatePath: true);
        Program.RememberArtifactPath("funcsizes_txt", filename);
    }

    static int RoundUp(int value, int align)
    {
        if (align <= 0) return value;
        int mask = align - 1;
        return (value + mask) & ~mask;
    }

    static int EstimateMaxPcForOutput(IReadOnlyList<Expr> assembly)
    {
        int pc = CodeStart;
        int maxPc = pc;

        foreach (Expr e in assembly)
        {
            string rdName; byte[] rdBytes;
            if (e.Match(Tag.ReadonlyData, out rdName, out rdBytes))
            {
                pc += rdBytes.Length;
                if (pc > maxPc) maxPc = pc;
                continue;
            }

            int skipTarget;
            if (e.Match(Tag.SkipTo, out skipTarget))
            {
                if (skipTarget == 0)
                {
                    if (pc < CodeStart) pc = CodeStart;
                }
                else
                {
                    if (skipTarget < CodeStart) Program.Panic("assembler: skip address is too small");
                    if (skipTarget > MaxRomSize) Program.Panic("assembler: skip address is too large");
                    if (skipTarget < pc)
                    {
                        if (ShouldIgnoreBackwardBankBoundarySkip(pc, skipTarget))
                        {
                            if (pc > maxPc) maxPc = pc;
                            continue;
                        }
                        Program.Panic("assembler: cannot skip backward during estimate (pc=${0:X5}, skip=${1:X5})", pc, skipTarget);
                    }
                    pc = skipTarget;
                }
                if (pc > maxPc) maxPc = pc;
                continue;
            }

            int alignBytes;
            if (e.Match(Tag.Align, out alignBytes))
            {
                if (alignBytes <= 0 || (alignBytes & (alignBytes - 1)) != 0) Program.Panic("assembler: $align expects power-of-two positive integer");
                int newPc = (pc + (alignBytes - 1)) & ~(alignBytes - 1);
                if (newPc > MaxRomSize) Program.Panic("assembler: alignment advances PC beyond max ROM size");
                pc = newPc;
                if (pc > maxPc) maxPc = pc;
                continue;
            }

            string label;
            if (e.Match(Tag.Word, out label))
            {
                pc += 2;
                if (pc > maxPc) maxPc = pc;
                continue;
            }

            string mnemonic;
            AsmOperand operand;
            if (e.Match(Tag.Asm, out mnemonic, out operand))
            {
                pc += GetAsmSizeForLayout(mnemonic, operand);
                if (pc > maxPc) maxPc = pc;
                continue;
            }
        }

        return maxPc;
    }

    static IReadOnlyList<Expr> InsertAutoBankBoundarySkips(IReadOnlyList<Expr> assembly)
    {
        if (assembly == null || assembly.Count == 0) return assembly;

        var functionSizes = EstimateFunctionBodySizes(assembly);
        var output = new List<Expr>(assembly.Count + 16);
        int pc = CodeStart;

        void PadToNextBankIfNeeded(int size, Expr origin)
        {
            if (size <= 0) return;
            if (size > BankSize)
            {
                Program.Error("assembler: object too large for one ROM bank ({0} bytes)", size);
                return;
            }
            int off = pc & (BankSize - 1);
            if (off == 0 || off + size <= BankSize) return;
            int next = RoundUp(pc, BankSize);
            output.Add(Expr.Make(Tag.SkipTo, next).WithSource(origin.Source));
            pc = next;
        }

        foreach (Expr e in assembly)
        {
            string rdName; byte[] rdBytes;
            if (e.Match(Tag.ReadonlyData, out rdName, out rdBytes))
            {
                PadToNextBankIfNeeded(rdBytes == null ? 0 : rdBytes.Length, e);
                output.Add(e);
                pc += rdBytes == null ? 0 : rdBytes.Length;
                continue;
            }

            string label;
            if (e.Match(Tag.Function, out label))
            {
                int size;
                if (label != null && functionSizes.TryGetValue(label, out size))
                    PadToNextBankIfNeeded(size, e);
                output.Add(e);
                continue;
            }

            int skipTarget;
            if (e.Match(Tag.SkipTo, out skipTarget))
            {
                output.Add(e);
                if (skipTarget == 0)
                {
                    if (pc < CodeStart) pc = CodeStart;
                }
                else if (skipTarget < pc && ShouldIgnoreBackwardBankBoundarySkip(pc, skipTarget))
                {
                    // Keep the marker for trace/debug parity, but do not move PC back.
                }
                else
                {
                    pc = skipTarget;
                }
                continue;
            }

            if (e.Match(Tag.Align, out int alignBytes))
            {
                output.Add(e);
                if (alignBytes > 0)
                    pc = (pc + (alignBytes - 1)) & ~(alignBytes - 1);
                continue;
            }

            if (e.MatchTag(Tag.Word))
            {
                output.Add(e);
                pc += 2;
                continue;
            }

            string mnemonic;
            AsmOperand operand;
            if (e.Match(Tag.Asm, out mnemonic, out operand))
            {
                output.Add(e);
                pc += GetAsmSizeForLayout(mnemonic, operand);
                continue;
            }

            output.Add(e);
        }

        return output;
    }

    static Dictionary<string, int> EstimateFunctionBodySizes(IReadOnlyList<Expr> assembly)
    {
        var sizes = new Dictionary<string, int>(StringComparer.Ordinal);
        string current = null;
        int bytes = 0;

        void Finish()
        {
            if (string.IsNullOrEmpty(current)) return;
            sizes[current] = bytes;
            current = null;
            bytes = 0;
        }

        foreach (Expr e in assembly)
        {
            string label;
            if (e.Match(Tag.Function, out label))
            {
                Finish();
                current = label;
                bytes = 0;
                continue;
            }

            if (e.Match(Tag.SkipTo, out int _))
            {
                Finish();
                continue;
            }

            if (string.IsNullOrEmpty(current)) continue;

            string rdName; byte[] rdBytes;
            if (e.Match(Tag.ReadonlyData, out rdName, out rdBytes))
            {
                bytes += rdBytes == null ? 0 : rdBytes.Length;
                continue;
            }

            if (e.MatchTag(Tag.Word))
            {
                bytes += 2;
                continue;
            }

            string mnemonic;
            AsmOperand operand;
            if (e.Match(Tag.Asm, out mnemonic, out operand))
            {
                bytes += GetAsmSizeForLayout(mnemonic, operand);
                continue;
            }
        }

        Finish();
        return sizes;
    }

    // Jump relaxation

    static IReadOnlyList<Expr> RelaxJumps(IReadOnlyList<Expr> assembly)
    {
        // Code starts immediately after the entry stub (see Assembler.CodeStart).
        const int CodeStart = 0x0160;

        // Work on a mutable copy.
        var cur = assembly.ToList();

        // Apply until no more conversions are possible.
        for (int iter = 0; iter < 16; iter++)
        {
            // Pass 1: address map for labels/functions in the code section.
            var addr = new Dictionary<string, int>();
            int pc = CodeStart;
            foreach (var e in cur)
            {
                string _rdn; byte[] _rdb;
                if (e.Match(Tag.ReadonlyData, out _rdn, out _rdb)) { pc += _rdb.Length; continue; }

                string label;
                int skipTarget;
                string mnemonic;
                AsmOperand operand;

                if (e.Match(Tag.Function, out label) || e.Match(Tag.Label, out label))
                {
                    if (!addr.ContainsKey(label)) addr.Add(label, pc);
                }
                else if (e.Match(Tag.SkipTo, out skipTarget))
                {
                    if (skipTarget < pc && ShouldIgnoreBackwardBankBoundarySkip(pc, skipTarget))
                    {
                        continue;
                    }
                    pc = skipTarget;
                }
                else if (e.Match(Tag.Align, out int alignBytes))
                {
                    pc = (pc + (alignBytes - 1)) & ~(alignBytes - 1);
                }
                else if (e.MatchTag(Tag.Word))
                {
                    pc += 2;
                }
                else if (e.Match(Tag.Asm, out mnemonic, out operand))
                {
                    pc += GetAsmSizeForLayout(mnemonic, operand);
                }
            }

            // Pass 2: decide conversions (JP -> JR) using the *current* layout.
            bool changed = false;
            pc = CodeStart;
            for (int i = 0; i < cur.Count; i++)
            {
                var e = cur[i];
                string _rdn2; byte[] _rdb2;
                if (e.Match(Tag.ReadonlyData, out _rdn2, out _rdb2)) { pc += _rdb2.Length; continue; }

                int skipTarget;
                string mnemonic;
                AsmOperand operand;

                if (e.Match(Tag.SkipTo, out skipTarget))
                {
                    if (skipTarget < pc && ShouldIgnoreBackwardBankBoundarySkip(pc, skipTarget))
                    {
                        continue;
                    }
                    pc = skipTarget;
                    continue;
                }
                if (e.Match(Tag.Align, out int alignBytes2))
                {
                    pc = (pc + (alignBytes2 - 1)) & ~(alignBytes2 - 1);
                    continue;
                }
                if (e.MatchTag(Tag.Word))
                {
                    pc += 2;
                    continue;
                }

                if (e.Match(Tag.Asm, out mnemonic, out operand))
                {
                    string jrMnemonic;
                    if (TryMapJPToJR(mnemonic, out jrMnemonic) && operand.Modifier == ImmediateModifier.None)
                    {
                        int target;
                        if (TryResolveAbsoluteTarget(operand, addr, out target))
                        {
                            // If we were to encode this as JR, the displacement is relative to (pc + 2).
                            int delta = target - (pc + 2);
                            if (delta >= -128 && delta <= 127)
                            {
                                cur[i] = Expr.MakeAsm(jrMnemonic, operand).WithSource(e.Source);
                                changed = true;
                            }
                        }
                    }

                    // IMPORTANT: advance pc based on the *current* layout of this iteration.
                    pc += GetAsmSizeForLayout(mnemonic, operand);
                    continue;
                }
            }

            if (!changed) break;
        }

        return cur;
    }

    // Implements a simple "link-unit section" mechanism for ROM output ordering.
    // CodeGen can emit ($section "NAME") markers. We buffer subsequent expressions
    // into per-bank section buckets and emit named sections together at the end of the bank.
    // Notes/constraints:
    // - $skip_to and $align are kept in the default (unnamed) stream and are not moved.
    // - Bank boundaries are inferred from $skip_to targets (multiples of 0x4000).
    // - Section markers themselves are re-emitted once per section (for trace readability).
    static IReadOnlyList<Expr> ReorderSections(IReadOnlyList<Expr> assembly)
    {
        // Fast path: if no $section exists, return as-is.
        bool hasSection = false;
        foreach (var e in assembly)
        {
            if (e.Match(Tag.Section, out string _)) { hasSection = true; break; }
        }
        if (!hasSection) return assembly;

        // Key: (bankIndex, sectionName)
        var buckets = new Dictionary<(int bank, string sect), List<Expr>>();
        var order = new Dictionary<int, List<string>>();

        int bank = 0;
        string sect = ""; // default/unnamed

        Func<int, string, List<Expr>> getBucket = (b, s) =>
        {
            var key = (b, s ?? "");
            if (!buckets.TryGetValue(key, out var list))
            {
                list = new List<Expr>();
                buckets[key] = list;
            }
            return list;
        };

        // Split into buckets.
        foreach (Expr e in assembly)
        {
            // Section marker: changes current section (does not emit immediately).
            if (e.Match(Tag.Section, out string sectName))
            {
                sect = sectName ?? "";
                if (!string.IsNullOrEmpty(sect))
                {
                    if (!order.TryGetValue(bank, out var list)) { list = new List<string>(); order[bank] = list; }
                    if (!list.Contains(sect)) list.Add(sect);
                }
                continue;
            }

            // Layout barriers: keep in default section, and update inferred bank.
            int skip;
            if (e.Match(Tag.SkipTo, out skip))
            {
                // Bank-aligned skips start the target bank. Attach them to the target
                // bank's default stream so old-bank named sections cannot be emitted
                // after the boundary and force a later backward skip.
                if ((skip & 0x3FFF) == 0)
                {
                    bank = skip >> 14;
                    sect = ""; // reset section at bank boundaries
                    getBucket(bank, "").Add(e);
                }
                else
                {
                    // Preserve intra-bank skip directives in the current bank's default stream.
                    getBucket(bank, "").Add(e);
                }
                continue;
            }

            int align;
            if (e.Match(Tag.Align, out align))
            {
                // Keep alignment directives in default stream.
                getBucket(bank, "").Add(e);
                continue;
            }

            // Everything else goes into the current bucket.
            getBucket(bank, sect).Add(e);
        }

        // Determine bank iteration order. We emit banks in ascending numeric order.
        var banks = new HashSet<int>();
        foreach (var k in buckets.Keys) banks.Add(k.bank);
        var bankList = banks.OrderBy(x => x).ToList();

        var output = new List<Expr>(assembly.Count);
        foreach (int b in bankList)
        {
            // Default section first.
            output.AddRange(getBucket(b, ""));

            // Named sections at the end of the bank, in first-seen order.
            if (order.TryGetValue(b, out var sectOrder))
            {
                foreach (string s in sectOrder)
                {
                    var data = getBucket(b, s);
                    if (data.Count == 0) continue;
                    output.Add(Expr.Make(Tag.Section, s));
                    output.AddRange(data);
                }
            }
        }

        return output;
    }

    static bool TryMapJPToJR(string mnemonic, out string jrMnemonic)
    {
        jrMnemonic = null;
        if (mnemonic == "JP") { jrMnemonic = "JR"; return true; }
        if (mnemonic == "JP_NZ") { jrMnemonic = "JR_NZ"; return true; }
        if (mnemonic == "JP_Z") { jrMnemonic = "JR_Z"; return true; }
        if (mnemonic == "JP_NC") { jrMnemonic = "JR_NC"; return true; }
        if (mnemonic == "JP_C") { jrMnemonic = "JR_C"; return true; }
        return false;
    }

    static bool TryResolveAbsoluteTarget(AsmOperand operand, Dictionary<string, int> addr, out int target)
    {
        target = 0;

        // Numeric absolute target
        if (!operand.Base.HasValue)
        {
            target = operand.Offset;
            return true;
        }

        int baseAddr;
        if (!addr.TryGetValue(operand.Base.Value, out baseAddr)) return false;
        target = baseAddr + operand.Offset;
        return true;
    }

    static int GetAsmSizeForLayout(string mnemonic, AsmOperand operand)
    {
        bool isRelative = AsmInfo.ShortJumpInstructions.Contains(mnemonic);
        AddressMode actualMode = operand.Mode;
        if (isRelative && (actualMode == AddressMode.Absolute || actualMode == AddressMode.Immediate))
            actualMode = AddressMode.Relative;

        // Some patterns can be encoded in smaller forms (e.g., LDH for 0xFF00-0xFFFF).
        // Keep the layout pass consistent with the emitter to avoid PC drift.
        if ((mnemonic == "LD_A_MEM" || mnemonic == "LD_MEM_A")
            && actualMode == AddressMode.Absolute
            && !operand.Base.HasValue
            && ((operand.Offset & 0xFF00) == 0xFF00))
        {
            actualMode = AddressMode.Immediate;
        }

        int format = ParseAddressModeForLayout(actualMode);
        return 1 + AsmInfo.OperandSizes[format];
    }

    static int ParseAddressModeForLayout(AddressMode mode)
    {
        if (mode == AddressMode.Implicit) return AsmInfo.IMP;
        if (mode == AddressMode.Immediate) return AsmInfo.IMM;
        if (mode == AddressMode.Immediate16) return AsmInfo.IMM16;
        if (mode == AddressMode.Absolute) return AsmInfo.ABS;
        if (mode == AddressMode.HighMem) return AsmInfo.ZPG;
        if (mode == AddressMode.Indirect) return AsmInfo.IND;
        if (mode == AddressMode.Relative) return AsmInfo.REL;
        return AsmInfo.IMP;
    }

    void DefineSymbol(byte[] rom, string symbol, int address, bool isLabel, MemoryRegion region, string section)
    {
        if (!Symbols.ContainsKey(symbol))
        {
            Symbols.Add(symbol, new AsmSymbol { Value = address, IsLabel = isLabel, Region = region, Section = section });
        }
        DoFixups(rom);
    }

    void DoFixups(byte[] rom)
    {
        for (int i = Fixups.Count - 1; i >= 0; i--)
        {
            Fixup fixup = Fixups[i];
            int target;
            if (TryGetOperandValue(fixup.Operand, fixup.Mode, fixup.Location, fixup.IsRelativeBranch, out target))
            {
                if (fixup.IsWordDefinition)
                {
                    if (fixup.Location < 0 || fixup.Location + 2 > rom.Length)
                        Program.Panic("assembler: fixup location out of ROM bounds");
                    rom[fixup.Location] = LowByte(target);
                    rom[fixup.Location + 1] = HighByte(target);
                }
                else
                {
                    int formalSize = fixup.Size;

                    if (fixup.Location < 0 || fixup.Location + formalSize > rom.Length)
                        Program.Panic("assembler: fixup location out of ROM bounds");

                    if (formalSize == 1)
                    {
                        if (fixup.IsRelativeBranch)
                        {
                            if (target < -128 || target > 127) Program.Error("Branch out of range at {0:X4}", fixup.Location);
                            rom[fixup.Location] = (byte)((sbyte)target);
                        }
                        else
                        {
                            rom[fixup.Location] = LowByte(target);
                        }
                    }
                    else if (formalSize == 2)
                    {
                        rom[fixup.Location] = LowByte(target);
                        rom[fixup.Location + 1] = HighByte(target);
                    }
                }
                Fixups.RemoveAt(i);
            }
        }
    }

    bool TryGetOperandValue(AsmOperand operand, AddressMode mode, int operandLocation, bool isRelativeBranch, out int value)
    {
        value = 0;
        AsmSymbol sym = null;
        int baseValue = 0;
        if (!operand.Base.HasValue) baseValue = 0;
        else if (Symbols.TryGetValue(operand.Base.Value, out sym)) { baseValue = sym.Value; }
        else return false;

        if (operand.Modifier == ImmediateModifier.Bank)
        {
            int bank = 0;
            if (sym != null)
            {
                if (sym.Region.Tag == MemoryRegionTag.ProgramRom) bank = (baseValue + operand.Offset) >> 14;
                else if (sym.Region.Tag == MemoryRegionTag.WramX) bank = LogicalBank(sym.Region, baseValue + operand.Offset) ?? 0;
            }
            else bank = (baseValue + operand.Offset) >> 14;

            value = bank & 0xFF;
            return true;
        }

        value = baseValue + operand.Offset;

        // ROM symbols are stored as file offsets; RAM symbols are already CPU-visible addresses.
        if (!isRelativeBranch && operand.Base.HasValue && sym != null && sym.Region.Tag == MemoryRegionTag.ProgramRom)
        {
            value = CpuAddrFromFileOffset(value);
        }
        else if (isRelativeBranch)
        {
            // Relative branches use file offsets; delta remains correct within the same bank.
            value = value - (operandLocation + 1);
        }

        if (operand.Modifier == ImmediateModifier.LowByte) value = value & 0xFF;
        else if (operand.Modifier == ImmediateModifier.HighByte) value = (value >> 8) & 0xFF;

        return true;
    }

    bool IsRelativeBranch(string mnemonic)
    {
        return AsmInfo.ShortJumpInstructions.Contains(mnemonic);
    }

    static int ParseAddressMode(AddressMode mode)
    {
        if (mode == AddressMode.Implicit) return AsmInfo.IMP;
        if (mode == AddressMode.Immediate) return AsmInfo.IMM;
        if (mode == AddressMode.Immediate16) return AsmInfo.IMM16;
        if (mode == AddressMode.Absolute) return AsmInfo.ABS;
        if (mode == AddressMode.HighMem) return AsmInfo.ZPG;
        if (mode == AddressMode.Indirect) return AsmInfo.IND;
        if (mode == AddressMode.Relative) return AsmInfo.REL;
        return AsmInfo.IMP;
    }

    static int GetFormalOperandSize(AddressMode mode)
    {
        return AsmInfo.OperandSizes[ParseAddressMode(mode)];
    }
    static byte LowByte(int n) => (byte)(n & 0xFF);
    static byte HighByte(int n) => (byte)((n >> 8) & 0xFF);
}

class Fixup
{
    public AsmOperand Operand;
    public int Location;
    public AddressMode Mode;
    public bool IsRelativeBranch;
    public bool IsWordDefinition;
    public int Size;
}

[DebuggerDisplay("{Show(),nq}")]
class AsmOperand
{
    public readonly Maybe<string> Base = Maybe.Nothing;
    public readonly int Offset = 0;
    public readonly AddressMode Mode;
    public readonly ImmediateModifier Modifier;
    public readonly string Comment;
    public static readonly AsmOperand Implicit = new AsmOperand(0, AddressMode.Implicit);

    public AsmOperand(string actualBase, AddressMode mode) : this(actualBase, 0, mode, ImmediateModifier.None) { }
    public AsmOperand(string actualBase, ImmediateModifier modifier) : this(actualBase, 0, AddressMode.Immediate, modifier) { }
    public AsmOperand(int value, AddressMode mode) : this(Maybe.Nothing, value, mode, ImmediateModifier.None) { }
    public AsmOperand(int value, ImmediateModifier modifier) : this(Maybe.Nothing, value, AddressMode.Immediate, modifier) { }

    public AsmOperand(Maybe<string> optionalBase, int offset, AddressMode mode, ImmediateModifier modifier, string comment = null)
    {
        Base = optionalBase; Offset = offset; Mode = mode; Modifier = modifier; Comment = comment;
    }

    public AsmOperand WithMode(AddressMode newMode) => new AsmOperand(Base, Offset, newMode, Modifier, Comment);
    public AsmOperand WithModifier(ImmediateModifier newModifier) => new AsmOperand(Base, Offset, Mode, newModifier, Comment);
    public AsmOperand WithComment(string newComment) => new AsmOperand(Base, Offset, Mode, Modifier, newComment);
    public AsmOperand WithComment(string newCommentFormat, params object[] args) => WithComment(string.Format(newCommentFormat, args));
    public AsmOperand ReplaceBase(int baseValue)
    {
        if (!Base.HasValue) throw new Exception("this operand has no base symbol");
        return new AsmOperand(Maybe.Nothing, baseValue + Offset, Mode, Modifier, Comment);
    }

    public string Show()
    {
        string s;
        if (!Base.HasValue) s = Program.FormatAssemblyInteger(Offset);
        else if (Offset == 0) s = Base.Value;
        else s = string.Format("{0}+{1}", Base.Value, Program.FormatAssemblyInteger(Offset));

        if (Modifier == ImmediateModifier.LowByte) s = "LOW(" + s + ")";
        else if (Modifier == ImmediateModifier.HighByte) s = "HIGH(" + s + ")";
        else if (Modifier == ImmediateModifier.Bank) s = "BANK(" + s + ")";

        if (Mode == AddressMode.Immediate) return s;
        if (Mode == AddressMode.Immediate16) return s; // Add support
        if (Mode == AddressMode.Absolute) return "[" + s + "]";
        if (Mode == AddressMode.Indirect) return "[HL]";
        if (Mode == AddressMode.HighMemX) return "[" + s + "+X]";
        return s;
    }
}

enum AddressMode
{
    Implicit, Immediate, Immediate16, HighMem, HighMemX, Absolute, AbsoluteX, AbsoluteY, Indirect, IndirectX, IndirectY, Relative,
}
enum ImmediateModifier { None, LowByte, HighByte, Bank }
class AsmSymbol { public int Value; public bool IsLabel; public MemoryRegion Region; public string Section; }





