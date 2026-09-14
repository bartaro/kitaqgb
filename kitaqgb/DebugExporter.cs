using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

// Accumulate address-based segments, symbols and synthetic source lines for the legacy debug format.
// Bank annotations do not create separate bank-specific segment layouts.
class DebugExporter
{
    List<SegmentInfo> Segments = new List<SegmentInfo>();
    List<SpanInfo> Spans = new List<SpanInfo>();
    List<SymbolInfo> Symbols = new List<SymbolInfo>();

    // Keep a direct list of variables for vibe-friendly var table output.
    readonly List<VarInfo> Vars = new List<VarInfo>();
    List<LineInfo> Lines = new List<LineInfo>();
    List<string> Comments = new List<string>();

    // Install the fixed GB address-space segments and choose which regions the legacy file exports.
    // The uncovered FEA0-FEFF interval has no segment.
    public DebugExporter()
    {

        // 0000-7FFF: ROM (32KB)
        AddSegment(0x0000, 0x8000, true, true);

        // 8000-9FFF: VRAM (Video RAM)
        AddSegment(0x8000, 0x2000, false, false);

        // A000-BFFF: External RAM (SRAM)
        AddSegment(0xA000, 0x2000, false, false);

        // C000-DFFF: Work RAM (WRAM)
        AddSegment(0xC000, 0x2000, false, true);

        // E000-FDFF: Echo RAM (Not used)
        AddSegment(0xE000, 0x1E00, false, false);

        // FE00-FE9F: OAM (Sprite Attribute Table)
        AddSegment(0xFE00, 0x00A0, false, false);

        // FF00-FFFF: I/O Registers & HRAM
        AddSegment(0xFF00, 0x0100, false, true);

    }

    // Append a segment with its current list index as a persistent ID; overlaps and sizes are not validated.
    void AddSegment(int address, int size, bool isPrgRom, bool export = true)
    {
        Segments.Add(new SegmentInfo()
        {
            ID = Segments.Count,
            Start = address,
            Size = size,
            IsPrgRom = isPrgRom,
            Export = export,
        });
    }

    // Find the segment containing the start address and append a relative span.
    // The end address is not checked against that segment; return -1 when the start is unmapped.
    int AddSpan(int address, int size, bool isData)
    {
        SegmentInfo segment = GetSegment(address);
        if (segment == null) return -1;

        int spanID = Spans.Count;

        Spans.Add(new SpanInfo()
        {
            ID = spanID,
            SegmentID = segment.ID,
            Offset = address - segment.Start,
            Size = size,
            IsData = isData,
        });

        return spanID;
    }

    // Record a mapped variable in both the raw-name variable table and sanitized-name symbol list.
    // Unmapped starts are skipped; bank is optional metadata rather than an address resolver.
    public void AddVariable(string name, int address, int size, MemoryRegion region, int? bank = null)
    {
        if (AddSpan(address, size, true) != -1)
        {
            Vars.Add(new VarInfo
            {
                Name = name,
                Address = address,
                Size = size,
                Region = FormatRegionName(region, address),
                Bank = bank
            });
            Symbols.Add(new SymbolInfo()
            {
                ID = Symbols.Count,
                Name = SanitizeName(name),
                Address = address,
                SegmentID = GetSegment(address).ID,
                Size = size,
                Region = FormatRegionName(region, address),
                Bank = bank,
            });
        }
    }

    // Return a new list of variable tuples, preserving original names rather than sanitized symbol names.
    public IReadOnlyList<(string Name, int Address, int Size, string Region, int? Bank)> GetVariables()
    {
        return Vars.Select(v => (v.Name, v.Address, v.Size, v.Region, v.Bank)).ToList();
    }

    // Add a mapped function entry with placeholder size one; this does not measure the function body.
    public void AddFunction(string name, int address, int? bank = null)
    {
        SegmentInfo seg = GetSegment(address);
        if (seg != null)
        {
            Symbols.Add(new SymbolInfo()
            {
                ID = Symbols.Count,
                Name = SanitizeName(name),
                Address = address,
                SegmentID = seg.ID,
                Size = 1,
                Region = "ROM",
                Bank = bank
            });
        }
    }

    // Add an instruction span and, when comments exist, link it to a synthetic NOP line in the companion text.
    // An empty comment list still creates the span but no source-line record.
    public void TagInstruction(int address, int size, List<string> localComments)
    {
        int spanID = AddSpan(address, size, false);
        if (spanID == -1) return;

        if (localComments.Count > 0)
        {
            foreach (string comment in localComments)
            {
                Comments.Add("; " + comment);
            }

            Comments.Add("NOP"); // Dummy instruction text for viewer

            Lines.Add(new LineInfo()
            {
                ID = Lines.Count,
                FileID = 0,
                LineNumber = Comments.Count - 1,
                SpanID = spanID,
            });
        }
    }

    // Replace only colon and dollar characters; this is not general quoting or escaping for the debug format.
    string SanitizeName(string name)
    {
        return name.Replace(':', '_').Replace('$', '@');
    }

    // Return the first segment whose half-open interval contains the address, or null when none does.
    SegmentInfo GetSegment(int address)
    {
        foreach (SegmentInfo segment in Segments)
        {
            if (address >= segment.Start && address < segment.Start + segment.Size)
            {
                return segment;
            }
        }

        return null;
    }

    // Remove non-exported segments, spans and symbols in place before serializing.
    // IDs are not renumbered; the separate variable and source-line lists are not filtered by this removal.
    public void Save(string path)
    {
        foreach (SegmentInfo segment in Segments.ToArray())
        {
            if (!segment.Export)
            {
                Segments.Remove(segment);
                Spans.RemoveAll(x => x.SegmentID == segment.ID);
                Symbols.RemoveAll(x => x.SegmentID == segment.ID);
            }
        }

        List<string> lines = new List<string>();
        lines.Add(string.Format("version\tmajor=2,minor=0"));
        lines.Add(string.Format(
            "info\tcsym={0},file={1},lib={2},line={3},mod={4},scope={5},seg={6},span={7},sym={8},type={9}",
            0, 1, 0, 0, 0, 0, Segments.Count, Spans.Count, Symbols.Count, 0));
        lines.Add(string.Format("file\tid=0,name=\"{0}\"", Path.ChangeExtension(Path.GetFileName(path), ".dbc")));

        // The legacy read-only segment entry uses a fixed file offset of 16; it does not derive offsets from ROM banking.
        foreach (SegmentInfo segment in Segments)
        {
            lines.Add(string.Format("seg\tid={0},start=0x{1:X4},size=0x{2:X4},{3}", segment.ID, segment.Start, segment.Size, segment.IsPrgRom ? "type=ro,ooffs=16" : "type=rw"));
        }

        foreach (SpanInfo span in Spans)
        {
            lines.Add(string.Format("span\tid={0},seg={1},start={2},size={3}{4}", span.ID, span.SegmentID, span.Offset, span.Size, span.IsData ? ",type=0" : ""));
        }

        // Emit WRAMX banks as wbank and other banks as bank. Names and region strings are interpolated without general escaping.
        foreach (SymbolInfo symbol in Symbols)
        {
            string extra = "";
            if (!string.IsNullOrEmpty(symbol.Region)) extra += string.Format(",region=\"{0}\"", symbol.Region);
            if (symbol.Bank.HasValue)
            {
                if (symbol.Region != null && symbol.Region.StartsWith("WRAMX", StringComparison.Ordinal))
                    extra += string.Format(",wbank={0}", symbol.Bank.Value);
                else
                    extra += string.Format(",bank={0}", symbol.Bank.Value);
            }
            lines.Add(string.Format("sym\tid={0},name=\"{1}\",size={2},val=0x{3:X4},seg={4}{5}", symbol.ID, symbol.Name, symbol.Size, symbol.Address, symbol.SegmentID, extra));
        }

        foreach (LineInfo line in Lines)
        {
            lines.Add(string.Format("line\tid={0},file={1},line={2},span={3}", line.ID, line.FileID, line.LineNumber, line.SpanID));
        }

        // Use the actual debug output path when naming the companion file and remember both artifact paths.
        // The file record above already contains the originally requested companion name.
        path = IoUtil.WriteAllLinesUtf8Robust(path, lines, allowAlternatePath: true);
        string dbcPath = IoUtil.WriteAllLinesUtf8Robust(Path.ChangeExtension(path, ".dbc"), Comments, allowAlternatePath: true);
        Program.RememberArtifactPath("dbg", path);
        Program.RememberArtifactPath("dbc", dbcPath);
    }

    // Store a CPU-address interval and its export policy with a stable assigned ID.
    class SegmentInfo
    {
        public int ID;
        public int Start;
        public int Size;
        public bool IsPrgRom;
        public bool Export;
    }

    // Describe a code/data range relative to a segment start.
    class SpanInfo
    {
        public int ID;
        public int SegmentID;
        public int Offset;
        public int Size;
        public bool IsData;
    }

    // Retain the serialized name, address and optional memory/bank annotations.
    class SymbolInfo
    {
        public int ID;
        public string Name;
        public int Address;
        public int SegmentID;
        public int Size;
        public string Region;
        public int? Bank;
    }

    // Keep the original variable name and address metadata for direct table consumers.
    class VarInfo
    {
        public string Name;
        public int Address;
        public int Size;
        public string Region;
        public int? Bank;
    }

    // Associate a synthetic companion-file line with an instruction span.
    class LineInfo
    {
        public int ID;
        public int FileID;
        public int LineNumber;
        public int SpanID;
    }

    // Prefer an explicit memory-region tag; otherwise classify the address using the fixed GB map.
    // Unknown addresses fall back to the region descriptor text in uppercase.
    static string FormatRegionName(MemoryRegion region, int address)
    {
        if (region.Tag == MemoryRegionTag.ProgramRom) return "ROM";
        if (region.Tag == MemoryRegionTag.HighMem) return "HRAM";
        if (region.Tag == MemoryRegionTag.Oam) return "OAM";
        if (region.Tag == MemoryRegionTag.Wram0) return "WRAM0";
        if (region.Tag == MemoryRegionTag.WramX)
        {
            if (region.WramBank > 0) return string.Format("WRAMX[{0}]", region.WramBank);
            return "WRAMX";
        }

        if (address >= 0xFF00 && address <= 0xFF7F) return "IO";
        if (address >= 0xFF80 && address <= 0xFFFE) return "HRAM";
        if (address >= 0xFE00 && address <= 0xFE9F) return "OAM";
        if (address >= 0xD000 && address <= 0xDFFF) return "WRAMX";
        if (address >= 0xC000 && address <= 0xCFFF) return "WRAM0";
        if (address >= 0xA000 && address <= 0xBFFF) return "SRAM";
        if (address >= 0x8000 && address <= 0x9FFF) return "VRAM";
        if (address >= 0x0000 && address <= 0x7FFF) return "ROM";
        return region.ToString().ToUpperInvariant();
    }
}





