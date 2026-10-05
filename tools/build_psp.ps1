param(
    [string]$Python = 'python',
    [string]$Toolchain = 'nightly-x86_64-pc-windows-gnu'
)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location $taskRoot
try {
    if (-not (Test-Path -LiteralPath 'assets/gui/emoji/symbols.json')) {
        & $Python tools/prepare_inline.py
        if ($LASTEXITCODE -ne 0) { throw 'Original inline resource extraction failed.' }
    }
    & $Python tools/compile_story.py
    if ($LASTEXITCODE -ne 0) { throw 'Story compilation failed.' }
    & $Python tools/prepare_psp.py
    if ($LASTEXITCODE -ne 0) { throw 'PSP resource conversion failed.' }
    Push-Location psp
    try {
        & cargo "+$Toolchain" psp --release '-Zbuild-std=core,alloc,compiler_builtins,panic_unwind'
        if ($LASTEXITCODE -ne 0) { throw 'PSP compilation failed.' }
    } finally { Pop-Location }
    & $Python tools/package_psp.py
    if ($LASTEXITCODE -ne 0) { throw 'PSP packaging failed.' }
} finally { Pop-Location }
