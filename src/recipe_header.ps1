param([string]$Compiler = '')
$ErrorActionPreference = 'Stop'
$script:compilerOverride = ''
if ($Compiler) { $script:compilerOverride = (Get-Command -Name $Compiler -CommandType Application -ErrorAction Stop).Source }

# Keep history arguments as data, including dollar signs, quotes and shell punctuation.
function Invoke-RecordedCompiler([string]$Directory, [string]$Arguments) {
    $selectedCompiler = $script:compilerOverride
    if (!$selectedCompiler) {
        $candidate = Join-Path $Directory 'kitaqgb.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { $selectedCompiler = $candidate }
        else { $selectedCompiler = (Get-Command kitaqgb -CommandType Application -ErrorAction Stop).Source }
    }
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $selectedCompiler
    $startInfo.WorkingDirectory = $Directory
    $startInfo.Arguments = $Arguments
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $child = [System.Diagnostics.Process]::Start($startInfo)
    try { $child.WaitForExit(); return $child.ExitCode }
    finally { $child.Dispose() }
}

