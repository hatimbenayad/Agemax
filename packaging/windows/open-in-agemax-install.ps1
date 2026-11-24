<#
.SYNOPSIS
Installs the per-user "Open Folder in Age Max" Windows Explorer context menu entry.
Does not require Administrator privileges (writes to HKCU\Software\Classes).
#>
param(
    [string]$ExePath = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ExePath)) {
    $candidates = @(
        (Join-Path $PSScriptRoot "agemax-app.exe"),
        (Join-Path $PSScriptRoot "..\..\target\release\agemax-app.exe"),
        (Join-Path $env:LOCALAPPDATA "Programs\AgeMax\agemax-app.exe")
    )
    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            $ExePath = (Resolve-Path $candidate).Path
            break
        }
    }
}

if ([string]::IsNullOrWhiteSpace($ExePath) -or -not (Test-Path $ExePath)) {
    throw "Could not find agemax-app.exe. Pass -ExePath <path-to-agemax-app.exe>."
}

$entries = @(
    @{ Key = "HKCU:\Software\Classes\Directory\shell\AgeMax"; Arg = "%1" },
    @{ Key = "HKCU:\Software\Classes\Directory\Background\shell\AgeMax"; Arg = "%V" }
)

foreach ($entry in $entries) {
    $cmdKey = Join-Path $entry.Key "command"
    New-Item -Path $cmdKey -Force | Out-Null
    Set-ItemProperty -Path $entry.Key -Name "(Default)" -Value "Open Folder in Age Max"
    Set-ItemProperty -Path $entry.Key -Name "Icon" -Value "`"$ExePath`",0"
    Set-ItemProperty -Path $cmdKey -Name "(Default)" -Value "`"$ExePath`" `"$($entry.Arg)`""
}

Write-Host "Installed Explorer context menu entry pointing to: $ExePath"
