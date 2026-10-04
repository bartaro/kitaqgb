// Diagnostic names and descriptions ported from the audited C# source.
pub fn help_entry(code: u16) -> (&'static str, &'static str) {
    match code {
        0 => ("None", "このコードの詳細説明は未登録です。"),
        1000 => (
            "ParseError",
            "構文を解析できません。トークンの欠落や順序不整合が疑われます。",
        ),
        1001 => ("ExpectedToken", "期待された記号/トークンが見つかりません。"),
        1002 => (
            "ExpectedType",
            "型指定が必要な位置で型が解決できませんでした。",
        ),
        1003 => ("PreprocessorWarning", "このコードの詳細説明は未登録です。"),
        1004 => ("PreprocessorError", "このコードの詳細説明は未登録です。"),
        2100 => (
            "AggregateNotDefined",
            "struct/union が未定義のまま参照されています。",
        ),
        2101 => (
            "IncompleteType",
            "不完全型のままサイズ確定やフィールドアクセスが要求されました。",
        ),
        2102 => ("InvalidWramXBank", "このコードの詳細説明は未登録です。"),
        2103 => (
            "BankedWramRequiresCgbOnly",
            "このコードの詳細説明は未登録です。",
        ),
        2104 => (
            "BankedWramLocalNotAllowed",
            "このコードの詳細説明は未登録です。",
        ),
        2105 => ("WramXBankOverflow", "このコードの詳細説明は未登録です。"),
        2106 => ("InvalidStackTop", "このコードの詳細説明は未登録です。"),
        2107 => ("InvalidStackReserve", "このコードの詳細説明は未登録です。"),
        2108 => ("StackTopBankMismatch", "このコードの詳細説明は未登録です。"),
        2109 => ("StackReservedOverlap", "このコードの詳細説明は未登録です。"),
        2110 => (
            "FixedAllocationOverlap",
            "このコードの詳細説明は未登録です。",
        ),
        2201 => ("ConstAssign", "constオブジェクトへの代入が検出されました。"),
        2202 => ("ConstModify", "const修飾の実体を変更しようとしています。"),
        2301 => ("StaticAssertFailed", "static_assert の条件が偽です。"),
        2401 => ("MustCheckUnused", "__must_check関数の戻り値が未使用です。"),
        2402 => (
            "NonNullArgument",
            "nonnull制約に違反する引数が渡されています。",
        ),
        2403 => ("RangeViolation", "__range制約を超える値が検出されました。"),
        2404 => (
            "RangeIndexOob",
            "__range付きインデックスが配列境界外となる可能性があります。",
        ),
        2405 => (
            "SwitchCaseEnumMismatch",
            "switch(enum)に対するcaseの列挙型が一致しません。",
        ),
        2406 => (
            "SwitchCaseNonEnumOnEnumSwitch",
            "enum switchに非enum caseが含まれています。",
        ),
        2407 => (
            "EnumStrictMix",
            "__enum_strictモードでenumと整数の混在が検出されました。",
        ),
        2408 => (
            "BitFlagsOp",
            "__bitflags型に不適切な演算が適用されています。",
        ),
        2409 => (
            "BitFlagsMix",
            "__bitflagsと通常整数の混在が検出されました。",
        ),
        2410 => (
            "SafeIndexIndex",
            "__safe_indexで安全でない添字式が検出されました。",
        ),
        2411 => (
            "SafeIndexMix",
            "__safe_index型と通常整数の混在が検出されました。",
        ),
        2412 => (
            "RestrictAlias",
            "__restrict違反の可能性があるエイリアスが検出されました。",
        ),
        2413 => (
            "ConstDiscard",
            "ポインタ変換でconst修飾が暗黙に破棄されています。",
        ),
        2414 => (
            "SwitchImplicitFallthrough",
            "switch caseで暗黙fallthroughが検出されました。",
        ),
        2415 => ("SwitchFallthroughUsage", "fallthrough使用位置が不正です。"),
        2416 => ("UnreachableCode", "到達不能コードが検出されました。"),
        2417 => ("UnusedSymbol", "未使用の変数/関数/シンボルです。"),
        2418 => ("ImplicitNarrowing", "暗黙縮小変換の可能性があります。"),
        2419 => (
            "PointerArithmeticDanger",
            "危険なポインタ演算の可能性があります。",
        ),
        2420 => ("ManualSvbkRequired", "このコードの詳細説明は未登録です。"),
        2421 => ("LargeStructCopy", "このコードの詳細説明は未登録です。"),
        2501 => (
            "ExternUndefined",
            "extern宣言に対応する定義が見つかりません。",
        ),
        2502 => ("ExternTypeMismatch", "extern宣言と定義の型が一致しません。"),
        2503 => (
            "ExternNotSupported",
            "externの扱いとして未対応なパターンです。",
        ),
        9000 => ("Internal", "内部エラーです。"),
        9001 => ("InternalNYI", "未実装パスに到達しました。"),
        9002 => ("InternalUnhandledCase", "想定外ケースが発生しました。"),
        _ => ("<unknown>", "このコードの詳細説明は未登録です。"),
    }
}
