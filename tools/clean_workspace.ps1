[CmdletBinding(SupportsShouldProcess = $true)]
param()
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Split-Path -Parent $PSScriptRoot)).Path
$taskPrefix = $taskRoot + [IO.Path]::DirectorySeparatorChar
$taskFolders = @('target-final', 'target', 'psp/target', 'psp/test-emulator', 'assets/gui/credits')
$taskTargets = @()
foreach ($taskRelative in $taskFolders) {
    $taskCandidate = Join-Path $taskRoot $taskRelative
    if (Test-Path -LiteralPath $taskCandidate) {
        $taskItem = Get-Item -LiteralPath $taskCandidate -Force
        if ($taskItem.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing linked folder: $taskCandidate" }
        $taskTargets += $taskItem
    }
}
$taskTargets += @(Get-ChildItem -LiteralPath $taskRoot -File -Filter 'smoke*.png')
$taskTargets += @(Get-ChildItem -LiteralPath (Join-Path $taskRoot 'dist/StarryFlowersRust') -File -Filter 'smoke*.png' -ErrorAction SilentlyContinue)
$taskTargets += @(Get-Item -LiteralPath (Join-Path $taskRoot 'layout-audit.txt'), (Join-Path $taskRoot 'psp/build.log') -ErrorAction SilentlyContinue)
$taskKeep = @('StarryFlowers.iso', 'EBOOT.PBP', 'ICON0.PNG', 'PIC1.PNG')
$taskTargets += @(Get-ChildItem -LiteralPath (Join-Path $taskRoot 'psp/dist') -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -notin $taskKeep })
$taskManifest = Join-Path $taskRoot 'psp/dist/DATA/MANIFEST.JSON'
if (Test-Path -LiteralPath $taskManifest) {
    $taskActive = (Get-Content -LiteralPath $taskManifest -Raw | ConvertFrom-Json).files
    if (-not $taskActive -or 'MANIFEST.JSON' -notin $taskActive) { throw 'Invalid resource manifest' }
    $taskTargets += @(Get-ChildItem -LiteralPath (Split-Path $taskManifest) -File | Where-Object { $_.Name -notin $taskActive })
}
foreach ($taskItem in $taskTargets) {
    if (-not $taskItem.FullName.StartsWith($taskPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "Outside workspace: $($taskItem.FullName)" }
}
foreach ($taskItem in $taskTargets) {
    if ($PSCmdlet.ShouldProcess($taskItem.FullName, 'Remove generated cache or temporary output')) {
        if ($taskItem.PSIsContainer) { Remove-Item -LiteralPath $taskItem.FullName -Recurse -Force }
        else { Remove-Item -LiteralPath $taskItem.FullName -Force }
    }
}
