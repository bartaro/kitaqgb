param(
  [Parameter(Mandatory=$true)][string]$RomPath,
  [Parameter(Mandatory=$true)][string]$MapPath,
  [string]$Symbol = '__kq_vblank_vector',
  [switch]$NoHeaderFix
)

$ErrorActionPreference = 'Stop'

$romFull = Resolve-Path $RomPath
$mapFull = Resolve-Path $MapPath

# Find the first four-hex-digit map address for the literal symbol name; a switchable-bank address is rejected.
$escaped = [regex]::Escape($Symbol)
$vectorAddr = $null
foreach ($line in Get-Content -LiteralPath $mapFull) {
  if ($line -match "^\s*([0-9A-Fa-f]{4})\s+.*\s$escaped\s*$") {
    $vectorAddr = [Convert]::ToInt32($matches[1], 16)
    break
  }
}

if ($null -eq $vectorAddr) {
  throw "patch_gb_vblank_irq: $Symbol not found in map"
}
if ($vectorAddr -ge 0x4000) {
  throw ('patch_gb_vblank_irq: {0} must be in fixed bank, got 0x{1:X4}' -f $Symbol, $vectorAddr)
}

$bytes = [IO.File]::ReadAllBytes($romFull)
# Recognize PUSH AF, BC, DE, HL near the mapped vector symbol. This short signature is a heuristic, not ISR validation.
$signature = [byte[]](0xF5, 0xC5, 0xD5, 0xE5)
$entryAddr = $null

for ($addr = $vectorAddr; $addr -lt [Math]::Min($vectorAddr + 96, $bytes.Length - $signature.Length); $addr++) {
  $match = $true
  for ($i = 0; $i -lt $signature.Length; $i++) {
    if ($bytes[$addr + $i] -ne $signature[$i]) {
      $match = $false
      break
    }
  }
  if ($match) {
    $entryAddr = $addr
    break
  }
}

if ($null -eq $entryAddr) {
  throw ('patch_gb_vblank_irq: raw ISR body signature not found near 0x{0:X4}' -f $vectorAddr)
}

# Replace the VBlank entry with JP entryAddr using a little-endian target; patch the supplied ROM in place.
$bytes[0x40] = 0xC3
$bytes[0x41] = [byte]($entryAddr -band 0xFF)
$bytes[0x42] = [byte](($entryAddr -shr 8) -band 0xFF)

# Unless disabled, also replace FF bytes in 0134..0143 with zero and recompute both header/global checksums.
# NoHeaderFix leaves checksums unchanged even though the vector changes; it does not preserve checksum validity.
if (!$NoHeaderFix) {
  for ($i = 0x0134; $i -le 0x0143; $i++) {
    if ($bytes[$i] -eq 0xFF) { $bytes[$i] = 0x00 }
  }

  $headerChk = 0
  for ($i = 0x0134; $i -le 0x014C; $i++) {
    $headerChk = ($headerChk - $bytes[$i] - 1) -band 0xFF
  }
  $bytes[0x014D] = [byte]$headerChk

  # Sum all ROM bytes except the two global-checksum bytes, then store the resulting word big-endian.
  $globalChk = 0
  for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($i -ne 0x014E -and $i -ne 0x014F) {
      $globalChk = ($globalChk + $bytes[$i]) -band 0xFFFF
    }
  }
  $bytes[0x014E] = [byte](($globalChk -shr 8) -band 0xFF)
  $bytes[0x014F] = [byte]($globalChk -band 0xFF)
}

# Persist only after symbol/signature checks and checksum work; no backup or atomic replacement is created.
[IO.File]::WriteAllBytes($romFull, $bytes)

if ($NoHeaderFix) {
  Write-Host ('patch_gb_vblank_irq: VBlank vector 0x0040 -> 0x{0:X4} ({1}, header unchanged)' -f $entryAddr, $Symbol)
} else {
  Write-Host ('patch_gb_vblank_irq: VBlank vector 0x0040 -> 0x{0:X4} ({1}), header checksums updated' -f $entryAddr, $Symbol)
}

