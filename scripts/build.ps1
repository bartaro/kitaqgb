param([switch]$KeepIntermediates)
$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $repositoryRoot
try {
    & cargo test --locked --tests
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    & cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    foreach ($program in @('kitaqgb','kitaqgb-zx0','kitaqgb-patch-vblank')) {
        Copy-Item -LiteralPath (Join-Path $repositoryRoot ('target/release/' + $program + '.exe')) -Destination (Join-Path $repositoryRoot ($program + '.exe'))
    }
    if (!$KeepIntermediates) {
        & cargo clean
        if ($LASTEXITCODE -ne 0) { throw 'Could not remove build intermediates' }
    }
} finally { Pop-Location }
