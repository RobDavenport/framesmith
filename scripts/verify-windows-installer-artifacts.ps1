[CmdletBinding()]
param(
    [string]$Path = "src-tauri/target/release/bundle",
    [string]$Version,
    [int64]$MinimumBytes = 1024
)

$ErrorActionPreference = "Stop"

function Fail {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

if (-not (Test-Path -LiteralPath $Path)) {
    Fail "Installer artifact path not found: $Path"
}

$msi = @(Get-ChildItem -LiteralPath $Path -Filter "*.msi" -File -Recurse)
$nsis = @(Get-ChildItem -LiteralPath $Path -Filter "*setup.exe" -File -Recurse)

if ($Version) {
    $msi = @($msi | Where-Object { $_.Name -like "*$Version*" })
    $nsis = @($nsis | Where-Object { $_.Name -like "*$Version*" })
}

if ($msi.Count -eq 0) {
    Fail "No MSI installer was found under $Path."
}

if ($nsis.Count -eq 0) {
    Fail "No NSIS setup executable was found under $Path."
}

$installerFiles = @($msi) + @($nsis)
foreach ($file in $installerFiles) {
    if ($file.Length -lt $MinimumBytes) {
        Fail "Installer output is smaller than $MinimumBytes bytes: $($file.FullName)"
    }
}

Write-Host "Windows installer artifacts verified."
Write-Host "Path: $Path"
Write-Host "Version filter: $(if ($Version) { $Version } else { '<none>' })"
foreach ($file in $installerFiles) {
    Write-Host "$($file.Name) $($file.Length) bytes"
}
