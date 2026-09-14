using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

enum CTypeTag
{
    Simple,
    Pointer,
    Struct,
    Union,
    Enum,
    Array,
    ArrayWithDimensionExpression,
    Function,
}

enum CSimpleType
{
    Implied,
    Void,
    UInt8,
    Int8,
    UInt16,
    Int16,
}

[DebuggerDisplay("{Show(),nq}")]
// Represent a mutable tagged C type. Factories retain child/parameter objects;
// shared built-in instances must not be modified as though they were private copies.
class CType : IEquatable<CType>
{
    public CTypeTag Tag;
    public CSimpleType SimpleType;
    public string Name;
    public CType Subtype;
    public int Dimension;
    public Expr DimensionExpression;

    // C qualifier (subset): we currently only track `const`.
    // Notes:
    // - const does not affect layout/sizeof.
    // - This is primarily used to enable parsing/typing of `const` declarations
    // and to support treating `const <scalar>` globals as compile-time constants.
    public bool IsConst;

    // C qualifier: pointer restrict (subset). Only meaningful for pointer nodes.
    // Lint-only; does not affect layout/sizeof.
    public bool IsRestrict;

    // If true, this enum type participates in stricter enum/integer mixing diagnostics (\"__enum_strict\").
    public bool IsEnumStrict;

    // If true, this integer type is intended to be used as a bitflags set ("__bitflags").
    // Lint-only; does not affect layout.
    public bool IsBitFlags;

    // If true, this integer type is intended to be used as an array index ("__safe_index").
    // Lint-only; does not affect layout.
    public bool IsSafeIndex;

    // Optional forced alignment (in bytes). 0 means "use natural alignment".
    // This can be attached either to a type (e.g. __aligned(4) struct S) or to an object
    // via a declaration-level annotation that clones the type with this field set.
    // Notes:
    // - In this compiler, the "natural" alignments are currently 1 (u8) and 2 (u16/pointer/enum).
    // - Alignment affects layout of aggregates and placement of globals/readonly_data.
    public int ForcedAlign;

    // Aggregate packing flag ("__packed").
    // When true and this is a struct/union type, its layout uses 1-byte field alignment.
    // (Union size still rounds up to the aggregate alignment.)
    public bool IsPackedAggregate;

    // Function type: Subtype = return type, ParamTypes = parameter types.
    public CType[] ParamTypes;

    public static readonly CType Implied = MakeSimple(CSimpleType.Implied);
    public static readonly CType Void = MakeSimple(CSimpleType.Void);
    public static readonly CType UInt8 = MakeSimple(CSimpleType.UInt8);
    public static readonly CType Int8 = MakeSimple(CSimpleType.Int8);
    public static readonly CType UInt8Ptr = MakePointer(UInt8);
    public static readonly CType UInt16 = MakeSimple(CSimpleType.UInt16);
    public static readonly CType Int16 = MakeSimple(CSimpleType.Int16);
    public static readonly CType UInt16Ptr = MakePointer(UInt16);

    // Create an unqualified scalar node; semantic annotations start at their default values.
    public static CType MakeSimple(CSimpleType simple)
    {
        return new CType
        {
            Tag = CTypeTag.Simple,
            SimpleType = simple,
            IsConst = false,
			IsRestrict = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Create an unqualified pointer node referencing the supplied pointee type without cloning it.
    public static CType MakePointer(CType subtype)
    {
        return new CType
        {
            Tag = CTypeTag.Pointer,
            Subtype = subtype,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Create a named struct reference; field layout is resolved separately from this type node.
    public static CType MakeStruct(string name)
    {
        return new CType
        {
            Tag = CTypeTag.Struct,
            Name = name,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Create a named union reference without constructing or validating its member layout.
    public static CType MakeUnion(string name)
    {
        return new CType
        {
            Tag = CTypeTag.Union,
            Name = name,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Create an enum reference with strict enum-mixing diagnostics initially disabled.
    public static CType MakeEnum(string name)
    {
        return new CType
        {
            Tag = CTypeTag.Enum,
            Name = name,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }


    // Store an already evaluated array dimension without checking positivity or total layout size.
    public static CType MakeArray(CType elementType, int dimension)
    {
        return new CType
        {
            Tag = CTypeTag.Array,
            Subtype = elementType,
            Dimension = dimension,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Retain an unevaluated dimension expression for a later compiler phase.
    public static CType MakeArray(CType elementType, Expr dimension)
    {
        return new CType
        {
            Tag = CTypeTag.ArrayWithDimensionExpression,
            Subtype = elementType,
            DimensionExpression = dimension,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Retain the return type and parameter array, treating a null parameter array as empty.
    public static CType MakeFunction(CType returnType, CType[] paramTypes)
    {
        return new CType
        {
            Tag = CTypeTag.Function,
            Subtype = returnType,
            ParamTypes = paramTypes ?? Array.Empty<CType>(),
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = false,
            IsSafeIndex = false,
            ForcedAlign = 0,
            IsPackedAggregate = false,
        };
    }

    // Return this node when already non-const; otherwise make a shallow copy with const
    // removed. The copy also resets enum-strict/restrict/safe-index annotations in this implementation.
    public CType WithoutConst()
    {
        if (!IsConst) return this;
        // Shallow clone is enough because const does not change layout.
        return new CType
        {
            Tag = Tag,
            SimpleType = SimpleType,
            Name = Name,
            Subtype = Subtype,
            Dimension = Dimension,
            DimensionExpression = DimensionExpression,
            ParamTypes = ParamTypes,
            IsConst = false,
            IsEnumStrict = false,
            IsBitFlags = IsBitFlags,
            ForcedAlign = ForcedAlign,
            IsPackedAggregate = IsPackedAggregate,
        };
    }

    // These convenience predicates inspect the tag; they do not resolve named aggregate definitions.
    public bool IsSimple => Tag == CTypeTag.Simple;
    public bool IsPointer => Tag == CTypeTag.Pointer;
    public bool IsStructOrUnion => Tag == CTypeTag.Struct || Tag == CTypeTag.Union;
    public bool IsEnum => Tag == CTypeTag.Enum;
    public bool IsArray => Tag == CTypeTag.Array || Tag == CTypeTag.ArrayWithDimensionExpression;
    public bool IsFunction => Tag == CTypeTag.Function;
    public bool IsInteger => (IsSimple &&
        (SimpleType == CSimpleType.UInt8 || SimpleType == CSimpleType.Int8 ||
         SimpleType == CSimpleType.UInt16 || SimpleType == CSimpleType.Int16)) || IsEnum;

    // Enums satisfy IsInteger but are not classified as signed/unsigned by these scalar-kind checks.
    public bool IsUnsigned => IsInteger && (SimpleType == CSimpleType.UInt8 || SimpleType == CSimpleType.UInt16);
    public bool IsSigned => IsSimple && (SimpleType == CSimpleType.Int8 || SimpleType == CSimpleType.Int16);

    public override bool Equals(object obj)
    {
        // Don't bother supporting equality-testing with arbitrary other types.
        throw new NotSupportedException();
    }

    // Hash-based use is unsupported; callers must not put CType keys in an ordinary hash collection.
    public override int GetHashCode()
    {
        throw new NotSupportedException();
    }

    // Compare type structure and selected annotations against a non-null peer.
    // Array dimensions/expressions are intentionally not consulted by this comparator,
    // so equality here is not an aggregate-size or exact-declarator comparison.
    public bool Equals(CType other)
    {
        if (IsConst != other.IsConst) return false;
        if (IsRestrict != other.IsRestrict) return false;
        if (IsBitFlags != other.IsBitFlags) return false;
        if (IsSafeIndex != other.IsSafeIndex) return false;
        if (ForcedAlign != other.ForcedAlign) return false;
        if (IsPackedAggregate != other.IsPackedAggregate) return false;
        if (Tag != other.Tag) return false;
        else if (Tag == CTypeTag.Simple) return SimpleType.Equals(other.SimpleType);
        else if (Tag == CTypeTag.Pointer || Tag == CTypeTag.Array || Tag == CTypeTag.ArrayWithDimensionExpression) return Subtype.Equals(other.Subtype);
        else if (Tag == CTypeTag.Struct) return Name == other.Name;
        else if (Tag == CTypeTag.Union) return Name == other.Name;
        else if (Tag == CTypeTag.Enum) return Name == other.Name && IsEnumStrict == other.IsEnumStrict;
        else if (Tag == CTypeTag.Function)
        {
            if (!Subtype.Equals(other.Subtype)) return false;
            if ((ParamTypes == null ? 0 : ParamTypes.Length) != (other.ParamTypes == null ? 0 : other.ParamTypes.Length)) return false;
            int n = ParamTypes == null ? 0 : ParamTypes.Length;
            for (int i = 0; i < n; i++)
            {
                if (!ParamTypes[i].Equals(other.ParamTypes[i])) return false;
            }
            return true;
        }
        else
        {
            Program.NYI();
            return false;
        }
    }

    // Handle identity and null operands before dispatching to typed structural equality.
    public static bool operator ==(CType a, CType b)
    {
        if (ReferenceEquals(a, b)) return true;
        if (ReferenceEquals(a, null)) return false;
        if (ReferenceEquals(b, null)) return false;
        return a.Equals(b);
    }

    // Negate the null-safe typed equality operator.
    public static bool operator !=(CType a, CType b) => !(a == b);

    // Render a diagnostic type description with recursive const/restrict placement.
    // Alignment, packing and safe-index annotations are omitted; this is not a lossless C declaration.
    public string Show()
    {
        string core;
        if (Tag == CTypeTag.Simple) core = SimpleType.ToString().ToLower();
        else if (Tag == CTypeTag.Pointer) core = Subtype.Show() + "*";
        else if (Tag == CTypeTag.Struct) core = "struct " + Name;
        else if (Tag == CTypeTag.Union) core = "union " + Name;
        else if (Tag == CTypeTag.Enum) core = (Name == null || Name.Length==0) ? "enum" : ("enum " + Name);
        else if (Tag == CTypeTag.Array) core = string.Format("{0}[{1}]", Subtype.Show(), Dimension);
        else if (Tag == CTypeTag.ArrayWithDimensionExpression) core = string.Format("{0}[{1}]", Subtype.Show(), DimensionExpression.Show());
        else if (Tag == CTypeTag.Function)
        {
            string ps = "";
            if (ParamTypes != null && ParamTypes.Length > 0)
            {
                ps = string.Join(",", ParamTypes.Select(p => p.Show()));
            }
            core = string.Format("fn({0})->{1}", ps, Subtype.Show());
        }

        else throw new NotImplementedException();

        // Prefer a C-like rendering for const placement.
        // - Pointer const (`u8* const`) should show as "<subtype>* const".
        // - Base const (`const u8*`) shows as "const <base>*" via the base node.
        // Also render __bitflags as a prefix on the base integer/enum type.
        if (IsBitFlags && (Tag == CTypeTag.Simple || Tag == CTypeTag.Enum))
        {
            core = "__bitflags " + core;
        }
        if (IsEnumStrict && Tag == CTypeTag.Enum)
        {
            core = "__enum_strict " + core;
        }
        // Prefer a C-like rendering for qualifier placement.
        // - Pointer qualifiers (const/restrict) render as suffix: '<subtype>* __restrict const'.
        // - Base const (`const u8*`) renders as prefix on the base node.
        if (Tag == CTypeTag.Pointer)
        {
            string suf = "";
            if (IsRestrict) suf += " __restrict";
            if (IsConst) suf += " const";
            return suf.Length == 0 ? core : (core + suf);
        }
        return IsConst ? ("const " + core) : core;
    }
}




