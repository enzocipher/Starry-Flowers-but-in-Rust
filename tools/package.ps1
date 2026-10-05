$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskDist = Join-Path $taskRoot 'dist/StarryFlowersRust'
New-Item -ItemType Directory -Force $taskDist | Out-Null
$taskBinary = Join-Path $taskRoot 'target/release/starryflowers-rust.exe'
if (-not (Test-Path -LiteralPath $taskBinary)) { throw 'Run cargo build --release first.' }
Copy-Item -LiteralPath $taskBinary -Destination (Join-Path $taskDist 'StarryFlowersRust.exe') -Force
New-Item -ItemType Directory -Force (Join-Path $taskDist 'assets') | Out-Null
foreach ($taskFolder in @('images', 'audio', 'tl', 'gui')) {
    Copy-Item -LiteralPath (Join-Path $taskRoot "assets/$taskFolder") -Destination (Join-Path $taskDist 'assets') -Recurse -Force
}
New-Item -ItemType Directory -Force (Join-Path $taskDist 'assets/gui/credits') | Out-Null
Copy-Item -LiteralPath (Join-Path $taskRoot 'licenses/RENPY-LICENSE.txt') -Destination (Join-Path $taskDist 'assets/gui/credits/RENPY-LICENSE.txt') -Force
Copy-Item -LiteralPath (Join-Path $taskRoot 'README.md') -Destination $taskDist -Force
Write-Output "Portable build: $taskDist"
