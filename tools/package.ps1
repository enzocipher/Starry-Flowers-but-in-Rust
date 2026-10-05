$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskDist = Join-Path $taskRoot 'dist/StarryFlowersRust'
New-Item -ItemType Directory -Force $taskDist | Out-Null
$taskBinary = @('target/release/starryflowers-rust.exe', 'target-final/release/starryflowers-rust.exe') |
    ForEach-Object { Get-Item -LiteralPath (Join-Path $taskRoot $_) -ErrorAction SilentlyContinue } |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $taskBinary) { throw 'Run cargo build --release first.' }
Copy-Item -LiteralPath $taskBinary -Destination (Join-Path $taskDist 'StarryFlowersRust.exe') -Force
New-Item -ItemType Directory -Force (Join-Path $taskDist 'assets') | Out-Null
foreach ($taskFolder in @('images', 'audio', 'tl', 'gui')) {
    Copy-Item -LiteralPath (Join-Path $taskRoot "assets/$taskFolder") -Destination (Join-Path $taskDist 'assets') -Recurse -Force
}
Copy-Item -LiteralPath (Join-Path $taskRoot 'README.md') -Destination $taskDist -Force
Write-Output "Portable build: $taskDist"
