<#
.SYNOPSIS
Removes the per-user "Open Folder in Age Max" Windows Explorer context menu entries.
#>
$ErrorActionPreference = "Stop"

$keys = @(
    "HKCU:\Software\Classes\Directory\shell\AgeMax",
    "HKCU:\Software\Classes\Directory\Background\shell\AgeMax"
)

foreach ($key in $keys) {
    if (Test-Path $key) {
        Remove-Item -Path $key -Recurse -Force
        Write-Host "Removed $key"
    }
}

Write-Host "Uninstalled Age Max Explorer context menu entries."
