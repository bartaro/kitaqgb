param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$compilerName = 'kitaqgb'
$executable = Join-Path $repositoryRoot "$compilerName.exe"
if (!$OutputDirectory) {
    $OutputDirectory = Join-Path ([IO.Path]::GetTempPath()) ("kitaq-tokenizer-" + [guid]::NewGuid().ToString('N'))
}
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path

# Exercise the real compiled tokenizer without making its internal API public.
# Use a fresh PowerShell process per compiler so assembly types cannot collide.
$assembly = [Reflection.Assembly]::LoadFile($executable)
$tokenizerType = $assembly.GetType('Tokenizer', $true)
$positionType = $assembly.GetType('FilePosition', $true)
$position = $positionType.GetField('Unknown').GetValue($null)
$tokenizer = [Activator]::CreateInstance($tokenizerType, $true)
$evaluate = $tokenizerType.GetMethod('EvaluateIfDirectiveExpression', [Reflection.BindingFlags]'NonPublic,Instance')
$validateChar = $tokenizerType.GetMethod('IsValidCharacterConstant', [Reflection.BindingFlags]'NonPublic,Static')
$tokenizeFile = $tokenizerType.GetMethod('TokenizeFile', [type[]]@([string]))
$results = [Collections.Generic.List[object]]::new()

# Mixed precedence and nested short circuits must consume every operand while
# avoiding arithmetic warnings for expressions that are not evaluated.
$cases = @(
    @('0 && 1 || 1', 1), @('(1 || 0) && 0', 0),
    @('1 || 0 && 0', 1), @('(0 && 1) || (1 && 1)', 1),
    @('0 && 1 && 1 || 1', 1), @('1 || 0 || 0', 1),
    @('0 && (1 / 0)', 0), @('1 || (1 % 0)', 1),
    @('(0 && (1 / 0)) || 1', 1), @('(1 || (1 / 0)) && 0', 0),
    @('1 + 2 * 3 == 7', 1), @('8 >> 1 == 4', 1),
    @('(5 & 3) == 1', 1), @('~0 == -1', 1),
    @('UNDEFINED_NAME || 1', 1), @('defined(UNDEFINED_NAME)', 0)
)
foreach ($case in $cases) {
    $savedError = [Console]::Error
    $capture = [IO.StringWriter]::new()
    try {
        [Console]::SetError($capture)
        $actual = $evaluate.Invoke($tokenizer, @($case[0], $position))
    } finally { [Console]::SetError($savedError) }
    $diagnostics = $capture.ToString()
    $results.Add([ordered]@{ kind='expression'; source=$case[0]; expected=$case[1]; actual=$actual; diagnostics=$diagnostics; passed=($actual -eq $case[1] -and $diagnostics.Length -eq 0) })
}

# A genuinely evaluated division by zero must still produce its diagnostic.
$savedError = [Console]::Error
$capture = [IO.StringWriter]::new()
try {
    [Console]::SetError($capture)
    $actual = $evaluate.Invoke($tokenizer, @('1 && (1 / 0)', $position))
} finally { [Console]::SetError($savedError) }
$diagnostics = $capture.ToString()
$results.Add([ordered]@{ kind='evaluated-arithmetic'; diagnostics=$diagnostics; passed=($diagnostics.Contains('division by zero')) })

# Reject line breaks, NUL, a quote and non-ASCII input; accept ordinary glyphs.
foreach ($codepoint in @(0, 10, 13, 39, 233, 32, 65, 126)) {
    $expected = $codepoint -in @(32, 65, 126)
    $actual = $validateChar.Invoke($null, @([char]$codepoint))
    $results.Add([ordered]@{ kind='character'; codepoint=$codepoint; expected=$expected; actual=$actual; passed=($actual -eq $expected) })
}

# Follow the public file-tokenization path, including object macros whose
# unevaluated replacements would otherwise emit arithmetic warnings.
$source = @'
#define ZERO_DIV (1 / 0)
#define ZERO_MOD (1 % 0)
#define PICK (0 && 1 || 1)
#if PICK && (1 || ZERO_DIV) && !(0 && ZERO_MOD)
u8 SELECTED_BRANCH = 'A';
#else
u8 WRONG_BRANCH = 0;
#endif
'@
$fixturePath = Join-Path $OutputDirectory 'conditional.c'
[IO.File]::WriteAllText($fixturePath, $source, [Text.UTF8Encoding]::new($false))
$savedError = [Console]::Error
$capture = [IO.StringWriter]::new()
try {
    [Console]::SetError($capture)
    $tokens = $tokenizeFile.Invoke($null, @([string]$fixturePath))
} finally { [Console]::SetError($savedError) }
$names = @($tokens | ForEach-Object { $_.Name })
$diagnostics = $capture.ToString()
$results.Add([ordered]@{ kind='file-macro-branch'; diagnostics=$diagnostics; passed=('SELECTED_BRANCH' -in $names -and 'WRONG_BRANCH' -notin $names -and $diagnostics.Length -eq 0) })

# Confirm that literal validation is reached from real source tokenization.
$fixturePath = Join-Path $OutputDirectory 'invalid-character.c'
[IO.File]::WriteAllText($fixturePath, ("u8 bad = '" + [char]233 + "';"), [Text.UTF8Encoding]::new($false))
$savedError = [Console]::Error
$capture = [IO.StringWriter]::new()
try {
    [Console]::SetError($capture)
    $null = $tokenizeFile.Invoke($null, @([string]$fixturePath))
} finally { [Console]::SetError($savedError) }
$diagnostics = $capture.ToString()
$results.Add([ordered]@{ kind='file-invalid-character'; diagnostics=$diagnostics; passed=$diagnostics.Contains('invalid character constant') })

# Retain a machine-readable result and executable hash for reproducibility.
$failed = @($results | Where-Object { !$_.passed })
$report = [ordered]@{
    compiler=$compilerName
    executable_sha256=(Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
    checks=$results.Count
    failures=$failed.Count
    results=@($results.ToArray())
}
$resultPath = Join-Path $OutputDirectory 'results.json'
[IO.File]::WriteAllText($resultPath, ($report | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
if ($failed.Count) { throw "$($failed.Count) tokenizer checks failed; see $resultPath" }
Write-Output "$compilerName tokenizer: $($results.Count) checks passed. Results: $resultPath"
