[CmdletBinding()]
param(
    [string]$KitaqgbRoot = "",
    [string]$OutputDirectory = "",
    [switch]$KeepBuildFiles
)

# Copyright (c) 2026 DAISUKE OBA. MIT License.
# Build the supplied game sources and exported C assets using this repository.
# No asset editor, Python installation, download, or external patcher is needed.
$ErrorActionPreference = "Stop"
$Project = $PSScriptRoot
$Parent = Split-Path -Parent $Project

if ([string]::IsNullOrWhiteSpace($KitaqgbRoot)) {
    $Candidates = @((Split-Path -Parent $Parent))
    foreach ($Candidate in $Candidates) {
        if ((Test-Path (Join-Path $Candidate "kitaqgb.exe")) -and
            (Test-Path (Join-Path $Candidate "lib\audio.c"))) {
            $KitaqgbRoot = $Candidate
            break
        }
    }
}
if ([string]::IsNullOrWhiteSpace($KitaqgbRoot)) {
    throw "KITAQGB not found. Use: .\build.ps1 -KitaqgbRoot C:\path\to\kitaqgb"
}
$KitaqgbRoot = (Resolve-Path -LiteralPath $KitaqgbRoot).Path
$Compiler = Join-Path $KitaqgbRoot "kitaqgb.exe"
$Lib = Join-Path $KitaqgbRoot "lib"
if (!(Test-Path -LiteralPath $Compiler)) {
    throw "Compiler executable not found: $Compiler"
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $Project "out"
}
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
# Isolated build directory prevents an old ROM from being mistaken for a new one.
$BuildDir = Join-Path $OutputDirectory ("build_" + [DateTime]::Now.ToString("yyyyMMdd_HHmmss_fff"))
New-Item -ItemType Directory $BuildDir | Out-Null
$DebugOut = Join-Path $BuildDir "debug_output_shirohebi"
$Log = Join-Path $BuildDir "build.log"
$Out = Join-Path $BuildDir "shirohebi.gb"
$Source = Join-Path $BuildDir "shirohebi.c"

$SourceParts = @(
    "shirohebi_common.c", "text_tiles.c", "sfx.c", "shirohebi_audio.c",
    "shirohebi_background.c", "shirohebi_text.c", "shirohebi_save.c",
    "shirohebi_display.c", "shirohebi_apples.c", "shirohebi_pause.c",
    "shirohebi_snake.c", "shirohebi_collision.c", "shirohebi_demo.c",
    "shirohebi_input.c", "shirohebi_flow.c", "shirohebi_render.c",
    "shirohebi_main.c"
)
$Combined = New-Object System.Text.StringBuilder
foreach ($Part in $SourceParts) {
    $PartPath = Join-Path $Project $Part
    if (!(Test-Path -LiteralPath $PartPath)) { throw "Source file missing: $PartPath" }
    [void]$Combined.Append("/* $Part */`n")
    [void]$Combined.Append([System.IO.File]::ReadAllText($PartPath).Replace("`r`n", "`n").TrimEnd())
    [void]$Combined.Append("`n`n")
}
[System.IO.File]::WriteAllText($Source, $Combined.ToString(), [System.Text.Encoding]::ASCII)

$AudioSource = Join-Path $Lib "audio.c"
$AudioVBlankSource = Join-Path $Lib "audio_vblank.c"
$AudioBuildSource = Join-Path $BuildDir "audio_shirohebi.c"
$AudioVBlankBuildSource = Join-Path $BuildDir "audio_vblank_shirohebi.c"
$AudioCode = [System.IO.File]::ReadAllText($AudioSource)
if (!$AudioCode.Contains("#ifndef AUDIO_EXCLUDE_LEGACY_MUSIC_SERVICE")) {
    throw "audio.c legacy-music exclusion marker not found. Review the current library before building."
}
$AudioCode = "#define AUDIO_EXCLUDE_LEGACY_MUSIC_SERVICE 1`n" + $AudioCode
[System.IO.File]::WriteAllText($AudioBuildSource, $AudioCode, [System.Text.Encoding]::ASCII)

$AudioVBlankCode = [System.IO.File]::ReadAllText($AudioVBlankSource)
$RestoreMarker = "void AudioVBlank_RequestRestoreCh1()"
if (!$AudioVBlankCode.Contains($RestoreMarker)) {
    throw "audio_vblank.c restore-helper marker not found. Review the current library before building."
}
$AudioVBlankCode = $AudioVBlankCode.Replace(
    $RestoreMarker, "#pragma fixed_bank -1`n#pragma bank 1`n`n" + $RestoreMarker)
if ($AudioVBlankCode.Contains("JR_Z audiovb_irq_pointer_stream")) {
    $AudioVBlankCode = $AudioVBlankCode.Replace(
        "JR_Z audiovb_irq_pointer_stream", "JP_Z audiovb_irq_pointer_stream")
} elseif (!$AudioVBlankCode.Contains("JP audiovb_irq_pointer_stream") -and
          !$AudioVBlankCode.Contains("JP_Z audiovb_irq_pointer_stream")) {
    throw "audio_vblank.c pointer-stream branch not found. Review the current library before building."
}
[System.IO.File]::WriteAllText($AudioVBlankBuildSource, $AudioVBlankCode, [System.Text.Encoding]::ASCII)

# Keep the source order and ROM/stack settings of the supplied game.
$Inputs = @(
    (Join-Path $Project "shirohebi_lib_bank0.c"),
    (Join-Path $Lib "system.c"),
    (Join-Path $Lib "audio_hwregs_gb.c"),
    $AudioBuildSource,
    (Join-Path $Project "music.c"),
    $AudioVBlankBuildSource,
    (Join-Path $Project "shirohebi_lib_bank1.c"),
    (Join-Path $Lib "sprite.c"),
    (Join-Path $Lib "fixed.c"),
    (Join-Path $Lib "physics2d.c"),
    (Join-Path $Project "shirohebi_chain.c"),
    (Join-Path $Lib "map.c"),
    (Join-Path $Lib "cgb_palette.c"),
    (Join-Path $Lib "save.c"),
    (Join-Path $Project "shirohebi_assets.c"),
    $Source
)
foreach ($InputFile in $Inputs) {
    if (!(Test-Path -LiteralPath $InputFile)) { throw "Build input missing: $InputFile" }
}
$CompilerArgs = $Inputs + @(
    "-I", $Lib, "-I", $Project, "-o", $Out,
    "--profile=release", "--no-cache", "--no-disasm", "--debug-out=$DebugOut",
    "--stack-bank=fixed", "--rst-disable", "--cgb=cgb",
    "--cart=mbc5_ram_battery", "--romsize=64k", "--ramsize=8k",
    "--rom-title=SHIROHEBI"
)
$CompilerHash = (Get-FileHash -LiteralPath $Compiler -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Compiler: $Compiler"
Write-Host "Compiler SHA-256: $CompilerHash"
Write-Host "Isolated output: $BuildDir"
$OldPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = "Continue"
    & $Compiler @CompilerArgs 2>&1 | Tee-Object -FilePath $Log
    $CompileExit = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $OldPreference
}
if ($CompileExit -ne 0 -or !(Test-Path -LiteralPath $Out)) {
    throw "KITAQGB build failed. Do not use an older ROM. See: $Log"
}

$MapPath = Join-Path $BuildDir "shirohebi.map"
$MapLines = Get-Content -LiteralPath $MapPath
function Get-FixedRomAddress([string]$Symbol) {
    $Pattern = '^([0-9A-Fa-f]{4})\s+0\s+[0-9A-Fa-f]+\s+S\s+ROM\s+' +
        [regex]::Escape($Symbol) + '$'
    $Entries = @($MapLines | Where-Object { $_ -match $Pattern })
    if ($Entries.Count -ne 1) { throw "Expected one fixed-bank symbol: $Symbol" }
    $Address = [Convert]::ToInt32($Entries[0].Substring(0, 4), 16)
    if ($Address -lt 0x0150 -or $Address -ge 0x4000) {
        throw "IRQ symbol is outside fixed ROM bank 0: $Symbol"
    }
    return $Address
}
$Vector = Get-FixedRomAddress "__kq_vblank_vector"
[void](Get-FixedRomAddress "AudioVBlank_RestoreCh1")
[void](Get-FixedRomAddress "AudioVBlank_RestoreCh3")
[void](Get-FixedRomAddress "SND_DemoVBlankHook")

# Install an explicit JP at 0040 to the fixed-bank ISR, then update both
# cartridge checksums. The IRQ must work while any switchable bank is selected.
# Refuse an unknown pre-existing vector instead of silently overwriting it.
$Rom = [System.IO.File]::ReadAllBytes($Out)
if ($Rom.Length -ne 65536) { throw "Expected a 64 KiB ROM, got $($Rom.Length) bytes." }
$AlreadyPatched = ($Rom[0x40] -eq 0xC3 -and
    ([int]$Rom[0x41] -bor ([int]$Rom[0x42] -shl 8)) -eq $Vector)
if (!$AlreadyPatched -and $Rom[0x40] -ne 0xD9) {
    throw "Unexpected VBlank vector at 0040. Review it rather than overwriting another ISR."
}
$Rom[0x40] = 0xC3
$Rom[0x41] = [byte]($Vector -band 0xFF)
$Rom[0x42] = [byte](($Vector -shr 8) -band 0xFF)
$HeaderChecksum = 0
for ($i = 0x134; $i -le 0x14C; $i++) {
    $HeaderChecksum = ($HeaderChecksum - [int]$Rom[$i] - 1) -band 0xFF
}
$Rom[0x14D] = [byte]$HeaderChecksum
$GlobalChecksum = 0
for ($i = 0; $i -lt $Rom.Length; $i++) {
    if ($i -ne 0x14E -and $i -ne 0x14F) {
        $GlobalChecksum = ($GlobalChecksum + [int]$Rom[$i]) -band 0xFFFF
    }
}
$Rom[0x14E] = [byte](($GlobalChecksum -shr 8) -band 0xFF)
$Rom[0x14F] = [byte]($GlobalChecksum -band 0xFF)
[System.IO.File]::WriteAllBytes($Out, $Rom)

# Only copy the freshly successful build to the easy-to-find release path.
$FinalRom = Join-Path $OutputDirectory "shirohebi.gb"
Copy-Item -LiteralPath $Out -Destination $FinalRom -Force
$Manifest = [ordered]@{
    application = "HARAPEKO SHIROHEBI"
    built_at = [DateTime]::Now.ToString("o")
    reference_repository = "https://github.com/bartaro/kitaqgb"
    compiler_path = $Compiler
    compiler_sha256 = $CompilerHash
    rom_path = $FinalRom
    rom_sha256 = (Get-FileHash -LiteralPath $FinalRom -Algorithm SHA256).Hash.ToLowerInvariant()
    rom_bytes = $Rom.Length
    vblank_vector = ("0x{0:X4}" -f $Vector)
    build_directory = $(if ($KeepBuildFiles) { $BuildDir } else { $null })
    assets = "Supplied shirohebi_assets.c; no art regeneration"
    emulator_tested_by_this_script = $false
    physical_hardware_tested_by_this_script = $false
}
$Manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutputDirectory "build_manifest.json") -Encoding UTF8
# Keep the symbol map for emulator debugging without retaining all intermediates.
Copy-Item -LiteralPath $MapPath -Destination (Join-Path $OutputDirectory "shirohebi.map") -Force
if (!$KeepBuildFiles) {
    # Delete only the unique directory created above, after checking containment.
    $ResolvedBuild = [System.IO.Path]::GetFullPath($BuildDir)
    $ResolvedOutput = [System.IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\') + '\'
    if (!$ResolvedBuild.StartsWith($ResolvedOutput, [System.StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $ResolvedBuild) -notmatch '^build_[0-9_]+$') {
        throw "Refusing to remove a build directory outside the selected output directory."
    }
    Remove-Item -LiteralPath $ResolvedBuild -Recurse -Force
}
Write-Host "Built ROM: $FinalRom"
Write-Host "Build success is not gameplay verification. Test DMG and CGB input scenarios before publishing."
