param([string]$Compiler, [Parameter(Mandatory=$true)][string]$Emulator, [Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
if (!$Compiler) { $Compiler=Join-Path (Split-Path $PSScriptRoot) 'kitaqgb.exe' }
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$OutputDirectory=(Resolve-Path -LiteralPath $OutputDirectory).Path
$assembly=[Reflection.Assembly]::LoadFile((Resolve-Path -LiteralPath $Compiler).Path)
$expr=$assembly.GetType('Expr',$true);$operand=$assembly.GetType('AsmOperand',$true);$mode=$assembly.GetType('AddressMode',$true)
$make=$expr.GetMethod('Make',[type[]]@([object[]]));$makeAsm=$expr.GetMethod('MakeAsm',[type[]]@([string],$operand))
$implicit=$operand.GetField('Implicit').GetValue($null)
$intCtor=$operand.GetConstructor([type[]]@([int],$mode));$stringCtor=$operand.GetConstructor([type[]]@([string],$mode))
$listType=[Collections.Generic.List`1].MakeGenericType($expr);$code=[Activator]::CreateInstance($listType)
function Add-Op([string]$name,$value=$null,[string]$addressMode='Immediate') {
    $op=$implicit
    if ($null -ne $value) {
        $parsed=[Enum]::Parse($mode,$addressMode)
        if ($value -is [string]) { $op=$stringCtor.Invoke(@([string]$value,$parsed)) }
        else { $op=$intCtor.Invoke(@([int]$value,$parsed)) }
    }
    $code.Add($makeAsm.Invoke($null,@($name,$op)))
}
function Add-Label([string]$name) { $code.Add($make.Invoke($null,@(,[object[]]@('$label',$name)))) }
$code.Add($make.Invoke($null,@(,[object[]]@('$function','main'))))
Add-Op 'XOR_A'
for($i=0;$i -lt 16;$i++){Add-Op 'LD_MEM_A' (0xC600+$i) 'Absolute'}
$cases=@(@('ADD_A_IMM',255),@('SUB_IMM',0),@('INC_A',5),@('DEC_A',5))
# A remains live after the store in each arithmetic form.
for($i=0;$i -lt 4;$i++){
    Add-Op 'LD_HL_IMM' 0xC700 'Immediate16';Add-Op 'LD_A_IMM' $cases[$i][1];Add-Op 'LD_HL_A'
    Add-Op 'LD_A_HL'
    if($i -lt 2){Add-Op $cases[$i][0] 1}else{Add-Op $cases[$i][0]}
    Add-Op 'LD_HL_A';Add-Op 'LD_MEM_A' (0xC600+$i) 'Absolute'
}
# ADD/SUB carry remains live even when A's value is no longer needed.
for($i=0;$i -lt 2;$i++){
    Add-Op 'XOR_A';Add-Op 'LD_HL_IMM' 0xC700 'Immediate16';Add-Op 'LD_A_IMM' $cases[$i][1];Add-Op 'LD_HL_A'
    Add-Op 'LD_A_HL';Add-Op $cases[$i][0] 1;Add-Op 'LD_HL_A'
    Add-Op 'JR_NC' ('failed'+$i) 'Relative';Add-Op 'LD_A_IMM' 1
    Add-Op 'JP' ('done'+$i) 'Absolute';Add-Label ('failed'+$i);Add-Op 'LD_A_IMM' 0;Add-Label ('done'+$i)
    Add-Op 'LD_MEM_A' (0xC604+$i) 'Absolute'
}
# XOR A discards both differing outputs: retain all four safe folds.
for($i=0;$i -lt 4;$i++){
    Add-Op 'LD_HL_IMM' 0xC700 'Immediate16';Add-Op 'LD_A_IMM' 5;Add-Op 'LD_HL_A';Add-Op 'LD_A_HL'
    if($i -lt 2){Add-Op $cases[$i][0] 1}else{Add-Op $cases[$i][0]}
    Add-Op 'LD_HL_A';Add-Op 'XOR_A';Add-Op 'LD_MEM_A' (0xC606+$i) 'Absolute'
}
Add-Op 'LD_A_IMM' 165;Add-Op 'LD_MEM_A' 0xC60F 'Absolute';Add-Label 'halted';Add-Op 'JP' 'halted' 'Absolute'
$optimized=$assembly.GetType('Optimizer',$true).GetMethod('Optimize').Invoke($null,@($code,0))
$show=$expr.GetMethod('Show',[type[]]@())
$text=($optimized | ForEach-Object { $show.Invoke($_,@()) }) -join "`n"
$folds=([regex]::Matches($text,'INC_HL_REF|DEC_HL_REF')).Count
$rom=Join-Path $OutputDirectory 'folds.gb'
$null=$assembly.GetType('Assembler',$true).GetMethod('Assemble').Invoke($null,[object[]]@($optimized,[string]$rom))
$reportPath=Join-Path $OutputDirectory 'runtime.json'
& $Emulator $rom --hardware dmg --run-frames 60 --dump-report $reportPath --watch-window 'result:50688:16' 2>&1 | Out-File (Join-Path $OutputDirectory 'runtime.txt')
if($LASTEXITCODE -ne 0){throw 'Emulator failed'}
$state=Get-Content -Raw -LiteralPath $reportPath | ConvertFrom-Json
$actual=@(($state.watched_memory | Where-Object name -eq 'result').preview_bytes)
$expected=@(0,255,6,4,1,1,0,0,0,0,0,0,0,0,0,165)
$passed=($actual.Count -eq 16 -and ($actual -join ',') -eq ($expected -join ',') -and $folds -eq 4)
[ordered]@{actual=$actual;expected=$expected;retained_safe_folds=$folds;passed=$passed} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'results.json') -Encoding utf8
Write-Host ('Memory-update values/carry: '+$passed+'; safe folds: '+$folds)
if(!$passed){throw 'Memory-update fold regression failed'}
