$ErrorActionPreference='Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try { cargo test --locked tokenizer; if ($LASTEXITCODE -ne 0) { throw 'Tokenizer tests failed' } } finally { Pop-Location }
