//! Stable diagnostic identifiers shared by the CLI and embedders.
pub fn infer(message: &str, fallback: u16) -> u16 {
    let m = message;
    if m.starts_with("#warning") {
        1003
    } else if m.starts_with("#error") {
        1004
    } else if m.starts_with("expected constant expression")
        || m.starts_with("incompatible struct/union assignment:")
        || m == "compound assignment is not supported for struct/union types"
        || m.starts_with("__range:")
        || m == "sizeof(expr) is not supported in __range constant expressions"
        || m.contains(": expected 2 arguments: __slice(ptr,len)")
    {
        1000
    } else if m.starts_with("expected ") && m.contains(", got ") {
        1001
    } else if m == "expected a type"
        || m.starts_with("expected a struct/union tag name")
        || m.starts_with("string initializer is only supported")
        || m.starts_with("too many initializers for array")
        || m.contains("first argument must be a pointer or array")
        || m.contains("second argument must be an integer length")
    {
        1002
    } else if m.starts_with("incomplete type")
        || m.contains("requires a complete struct/union type")
    {
        2101
    } else if m.contains("WRAMX bank must be in range 1..7") {
        2102
    } else if m.starts_with("banked WRAM requires") {
        2103
    } else if m.starts_with("banked WRAM is allowed only") {
        2104
    } else if m.contains("WRAMX bank") && m.contains("overflow") {
        2105
    } else if m.starts_with("--stack-top: address must be within") {
        2108
    } else if m.starts_with("--stack-top:") {
        2106
    } else if m.starts_with("--stack-reserve:") {
        2107
    } else if m.contains("selected stack window") || m.contains("selected stack bank") {
        2108
    } else if m.contains("overlaps reserved stack") {
        2109
    } else if m.contains("overlaps") && m.contains("allocation") {
        2110
    } else if m.starts_with("cannot use constant") || m.starts_with("cannot assign to const") {
        2201
    } else if m.contains("const-qualified") && m.contains("modify")
        || m.starts_with("cannot modify const")
    {
        2202
    } else if m.starts_with("static assertion failed") || m.starts_with("static_assert failed") {
        2301
    } else if m.starts_with("return value of ") && m.ends_with("is ignored") {
        2401
    } else if m.contains("outside declared range") {
        2403
    } else if m.contains("array index") && m.contains("declared range") {
        2404
    } else if m.starts_with("case label enum ") {
        2405
    } else if m.starts_with("case label is not an enum value") {
        2406
    } else if m.contains("__enum_strict") {
        2407
    } else if m.contains("__bitflags") && m.contains("is discouraged") {
        2408
    } else if m.contains("__bitflags") {
        2409
    } else if m.starts_with("array index") && m.contains("not __safe_index") {
        2410
    } else if m.contains("__safe_index") {
        2411
    } else if m.contains("__restrict may be violated") {
        2412
    } else if m.contains("discards 'const' qualifier") {
        2413
    } else if m.contains("implicit fallthrough") {
        2414
    } else if m.contains("'fallthrough'") {
        2415
    } else if m.starts_with("unreachable") {
        2416
    } else if m.starts_with("unused ") {
        2417
    } else if m.contains("implicit narrowing") {
        2418
    } else if m.contains("pointer arithmetic")
        || m.contains("subtracting a pointer from an integer")
    {
        2419
    } else if m.contains("manual SVBK") || m.contains("switch SVBK manually") {
        2420
    } else if m.starts_with("struct copy of ") {
        2421
    } else if m.starts_with("extern ") && m.contains("type mismatch") {
        2502
    } else if m.starts_with("extern ") && m.contains("compile-time constant") {
        2503
    } else if m.starts_with("extern ") && (m.contains("definition") || m.contains("not defined")) {
        2501
    } else if m.starts_with("NYI:") {
        9001
    } else {
        fallback
    }
}
pub fn suggestion(code: u16, message: &str) -> &'static str {
    match code {
        1001 => {
            "Check matching tokens (; ) ] }) and ensure the previous expression/declarator is properly closed."
        }
        1002 => {
            "A type is required here. Add a valid type specifier such as u8/u16/s8/s16/struct/enum."
        }
        1000 => "Check the previous line for missing brackets, commas, or semicolons.",
        2201 | 2202 => {
            "You are modifying a const-qualified object. Write to a non-const variable or revise qualifiers."
        }
        2301 => "Revisit the static_assert condition and the constants it depends on.",
        2403 => "Adjust the assigned value or the __range declaration so the value stays in range.",
        2404 => "Align array length and possible index range; clamp the index if needed.",
        2414 => "If fallthrough is intentional, add 'fallthrough;' explicitly.",
        2415 => "Use 'fallthrough;' only as the last statement in a case block.",
        2416 => "Remove statements after return/goto/break/continue or split control flow.",
        2417 => "The symbol is unused. Remove it or add a real use-site.",
        2418 => "If narrowing is intentional, add an explicit cast.",
        2419 => "Review pointer-offset type/range and add explicit casts if intentional.",
        2421 => {
            "Consider passing a pointer, copying only changed fields, or moving this copy out of a hot loop."
        }
        2501 => "Add a matching definition for this extern declaration.",
        2502 => "Make the extern declaration type match the actual definition type.",
        2503 => "For const-scalar address-taking, consider enabling -Zconst-scalar-in-rom.",
        2102 => "Use WRAMX bank numbers in the range 1..7.",
        2103 => {
            "Build with #pragma rom_cgb cgb_only or --cgb=cgb_only before using banked WRAM/SVBK."
        }
        2104 => {
            "Move this declaration to file scope; MVP banked WRAM is only for globals/file-statics."
        }
        2105 => "Reduce the allocation size or move data into a different WRAMX bank.",
        2420 => {
            "Save SVBK, switch to the symbol's bank, use the data, then restore the previous SVBK."
        }
        2106 => {
            "Use a 16-bit address inside the selected stack window, for example --stack-top=0xCFFF or --stack-top=$DFFF."
        }
        2107 => {
            "Choose a reserve size that fits between the selected stack top and the bottom of the chosen WRAM window."
        }
        2108 => "Keep --stack-top inside the WRAM window selected by --stack-bank.",
        2109 => {
            "Move the fixed allocation or lower its address so it does not overlap the reserved stack range."
        }
        2110 => {
            "Move the fixed allocation or reduce earlier automatic WRAM allocations so their ranges do not overlap."
        }
        _ if message.to_lowercase().contains("unknown option") => {
            "Use --help to check valid option names."
        }
        _ if message.to_lowercase().contains("division by zero") => {
            "Fix the constant expression so it does not divide by zero."
        }
        _ => "",
    }
}
