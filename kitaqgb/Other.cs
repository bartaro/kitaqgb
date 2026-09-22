using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;


// Store computed aggregate layout and a borrowed field array; the model does not recalculate or validate offsets.
class AggregateInfo
{
    public readonly AggregateLayout Layout;
    public readonly int TotalSize;
    public readonly int Alignment;
    public readonly bool IsPacked;
    public readonly FieldInfo[] Fields;

    // Retain the provided layout result and field references as supplied.
    public AggregateInfo(AggregateLayout layout, int totalSize, int alignment, bool isPacked, FieldInfo[] fields)
    {
        Layout = layout;
        TotalSize = totalSize;
        Alignment = alignment;
        IsPacked = isPacked;
        Fields = fields;
    }
}

enum AggregateLayout { Struct, Union }

// Describe a field or parameter by type/name/offset without owning its type object.
class FieldInfo
{
    public readonly CType Type;
    public readonly string Name;
    public readonly int Offset;

    // Store a field descriptor; offset meaning and validation belong to the consuming compiler phase.
    public FieldInfo(CType type, string name, int offset)
    {
        Type = type;
        Name = name;
        Offset = offset;
    }

    // Show the offset, diagnostic type text and field name for inspection.
    public override string ToString() => string.Format("Field: {0}, {1}, {2}", Offset, Type.Show(), Name);
}

[DebuggerDisplay("{Show(),nq}")]
// Collect mutable function signature, calling convention, placement and optional body metadata.
class CFunctionInfo
{
    public FieldInfo[] Parameters;
    public Symbol[] ParameterSymbols;
    public CType ReturnType;
    public Symbol ReturnSymbol;
    public Symbol ReturnPointerSymbol;
    public bool IsFastCall;
    public bool IsStackCall;
    public bool IsPrototype;

    public bool MustCheck;

    public int RomBank = 1;
    public bool HasFixedBank;
    public int PlacementOrder = int.MaxValue;
    public bool HasFixedOrder;

    public bool IsInline;
    public Expr Body;

    // Prototype parameter names do not bind the definition body. Retain the
    // allocated argument slots while adopting names from the definition.
    public void SetDefinitionParameters(FieldInfo[] parameters)
    {
        Parameters = parameters;
        if (ParameterSymbols == null || ParameterSymbols.Length != parameters.Length) return;
        for (int i = 0; i < parameters.Length; i++)
        {
            Symbol previous = ParameterSymbols[i];
            if (previous.Name != parameters[i].Name)
                ParameterSymbols[i] = new Symbol(previous.Tag, previous.Value, previous.Type, parameters[i].Name, previous.WramBank);
        }
    }

    // Format parameter types and selected calling-convention attributes; placement and body details are omitted.
    public string Show()
    {
        var paramTypes = Parameters.Select(x => string.Format("{0} {1}", x.Type.Show(), x.Name));
        string attr = (IsStackCall ? " [stackcall]" : "") + (IsFastCall ? " [fastcall]" : "");
        if (MustCheck) attr += " [must_check]";
        return string.Format("function({0}) {1}{2}", string.Join(", ", paramTypes), ReturnType.Show(), attr);
    }
}

// Describe requested placement, optionally with a fixed address or bank. These descriptors do not allocate memory.
struct MemoryRegion
{
    public readonly MemoryRegionTag Tag;
    public readonly int FixedAddress;
    public readonly int WramBank;

    // Store the requested region and optional address/bank without range validation.
    MemoryRegion(MemoryRegionTag tag, int address, int wramBank = 0)
    {
        Tag = tag;
        FixedAddress = address;
        WramBank = wramBank;
    }

    public static readonly MemoryRegion HighMem = new MemoryRegion(MemoryRegionTag.HighMem, 0);
    public static readonly MemoryRegion Oam = new MemoryRegion(MemoryRegionTag.Oam, 0);
    public static readonly MemoryRegion Ram = new MemoryRegion(MemoryRegionTag.Ram, 0);
    public static readonly MemoryRegion Wram0 = new MemoryRegion(MemoryRegionTag.Wram0, 0);
    public static readonly MemoryRegion WramX = new MemoryRegion(MemoryRegionTag.WramX, 0);
    public static readonly MemoryRegion ProgramRom = new MemoryRegion(MemoryRegionTag.ProgramRom, 0);
    // Build a fixed-address request without checking address-space availability.
    public static MemoryRegion Fixed(int address) => new MemoryRegion(MemoryRegionTag.Fixed, address);
    // Build a banked-memory request; the target allocator interprets and validates the bank.
    public static MemoryRegion WramXBank(int bank) => new MemoryRegion(MemoryRegionTag.WramX, 0, bank);

    // Display an explicit address for Fixed or a positive bank for WramX, otherwise the tag name.
    public override string ToString()
    {
        if (Tag == MemoryRegionTag.Fixed) return string.Format("{0}=${1:X4}", Tag, FixedAddress);
        if (Tag == MemoryRegionTag.WramX && WramBank > 0) return string.Format("{0}[{1}]", Tag, WramBank);
        return Tag.ToString();
    }
}

// Memory placement tags.
// Ram = "default WRAM (prefers WRAM0, may fall back to WRAMX depending on allocator policy)"
// Wram0 = force fixed WRAM0 (0xC000-0xCFFF)
// WramX = force bankable WRAMX (0xD000-0xDFFF)
enum MemoryRegionTag { HighMem, Oam, Ram, Wram0, WramX, ProgramRom, Fixed }

// Store a tagged constant or storage reference; Value interpretation depends on the symbol tag.
class Symbol
{
    public readonly SymbolTag Tag;
    public readonly int Value;
    public readonly CType Type;
    public readonly string Name;
    public readonly int WramBank;

    // Retain symbol metadata and the shared type object without assigning storage.
    public Symbol(SymbolTag tag, int value, CType type, string name, int wramBank = 0)
    {
        Tag = tag;
        Value = value;
        Type = type;
        Name = name;
        WramBank = wramBank;
    }
}

enum SymbolTag { Constant, ReadonlyData, Global, Local, StackParam }

// Link nested loops to their break/continue assembly targets.
class LoopScope
{
    public LoopScope Outer;
    public AsmOperand ContinueLabel;
    public AsmOperand BreakLabel;
}

// Keep local symbol bindings, the outer scope and allocator checkpoints for scope exit.
class LexicalScope
{
    public readonly LexicalScope Outer;
    public readonly Dictionary<string, Symbol> Symbols = new Dictionary<string, Symbol>();
    public int SavedHramNext;
    public int SavedWram0Next;
    public int SavedWram1Next;
    public bool PreserveAllocations = false;
    // Create an empty binding scope linked to its supplied parent.
    public LexicalScope(LexicalScope outer) { Outer = outer; }
}

// Track a named allocation interval and its next free cursor; allocation policy is implemented elsewhere.
class AllocationRegion
{
    public readonly string Name;
    public readonly int Bottom;
    public int Top;
    public int Next;
    public readonly List<AllocationReservation> Reservations = new List<AllocationReservation>();

    // Initialize the next cursor at the region bottom without validating the interval.
    public AllocationRegion(string name, int bottom, int top)
    {
        Name = name;
        Bottom = bottom;
        Next = bottom;
        Top = top;
    }
}

// Buffer speculative expression output together with its failure reason and reserved-register mask.
class OutputTransaction
{
    public List<Expr> Lines = new List<Expr>();
    public bool SpeculationError = false;
    public string AbortReason = "";
    public Register Reserved = Register.None;
}

// Record a named reserved interval for allocator overlap checks; this descriptor does not enforce it.
class AllocationReservation
{
    public readonly int Begin;
    public readonly int End;
    public readonly string Name;

    // Retain the supplied reservation endpoints and label without validating ordering.
    public AllocationReservation(int begin, int end, string name)
    {
        Begin = begin;
        End = end;
        Name = name;
    }
}

[Flags]
enum Register { None = 0, A = 1, B = 2, C = 4, D = 8, E = 16, H = 32, L = 64 }

// Pair low/high assembly operands for a wider value; the referenced operands are not cloned.
class WideOperand
{
    public AsmOperand Low, High;
    // Retain the low and high operand references in byte order.
    public WideOperand(AsmOperand low, AsmOperand high)
    {
        Low = low;
        High = high;
    }
}
