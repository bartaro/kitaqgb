param([string]$Compiler, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
if (!$Compiler) { $Compiler = Join-Path (Split-Path $PSScriptRoot) 'kitaqgb.exe' }
if (!$OutputDirectory) { throw 'Specify a private OutputDirectory for generated regression files.' }
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$assembly = [Reflection.Assembly]::LoadFile((Resolve-Path -LiteralPath $Compiler).Path)
$expr = $assembly.GetType('Expr', $true)
$operand = $assembly.GetType('AsmOperand', $true)
$mode = $assembly.GetType('AddressMode', $true)
$make = $expr.GetMethod('Make', [type[]]@([object[]]))
$makeAsm = $expr.GetMethod('MakeAsm', [type[]]@([string], $operand))
$implicit = $operand.GetField('Implicit').GetValue($null)
$relative = [Enum]::Parse($mode, 'Relative')
$constructor = $operand.GetConstructor([type[]]@([string], $mode))
$assembler = $assembly.GetType('Assembler', $true).GetMethod('Assemble')
$listType = [Collections.Generic.List`1].MakeGenericType($expr)
$rows = [Collections.Generic.List[object]]::new()
# Test both already-resolved backward labels and deferred forward labels at the signed-byte limits.
foreach ($case in @(@('backward',126,$false),@('backward',127,$true),@('forward',127,$false),@('forward',128,$true))) {
    $code = [Activator]::CreateInstance($listType)
    $code.Add($make.Invoke($null, @(,[object[]]@('$function','main'))))
    $target = $make.Invoke($null, @(,[object[]]@('$label','target')))
    $jump = $makeAsm.Invoke($null, @('JR',$constructor.Invoke(@('target',$relative))))
    if ($case[0] -eq 'backward') { $code.Add($target) } else { $code.Add($jump) }
    for ($i=0; $i -lt $case[1]; $i++) { $code.Add($makeAsm.Invoke($null,@('NOP',$implicit))) }
    if ($case[0] -eq 'backward') { $code.Add($jump) } else { $code.Add($target) }
    $code.Add($makeAsm.Invoke($null,@('RET',$implicit)))
    $capture=[IO.StringWriter]::new();$saved=[Console]::Error
    try {
        [Console]::SetError($capture)
        $output=Join-Path $OutputDirectory ($case[0]+'-'+$case[1]+'.gb')
        $null=$assembler.Invoke($null,[object[]]@($code,[string]$output))
    } finally { [Console]::SetError($saved) }
    $diagnostic=$capture.ToString()
    $rejected=$diagnostic.Contains('Branch out of range')
    $rows.Add([ordered]@{direction=$case[0];padding=$case[1];expected_rejection=$case[2];rejected=$rejected;passed=($rejected -eq $case[2])})
}
$rows | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'results.json') -Encoding utf8
$rows | Format-Table
if ($rows.passed -contains $false) { throw 'Relative-branch regression failed.' }
