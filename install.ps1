[CmdletBinding()]
param(
    [switch] $SkipBuild
)

$ErrorActionPreference = 'Stop'
if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot 'build.ps1')
}

$manifest = Join-Path $PSScriptRoot 'package\AppxManifest.xml'
Add-AppxPackage -Register $manifest -ForceApplicationShutdown

$package = Get-AppxPackage -Name 'Teamy.ShareToYtDlp'
if (-not $package) {
    throw 'Windows did not report the ShareToYtDlp package after registration.'
}

Write-Host "Installed $($package.PackageFullName)"
Write-Host "The Windows Share dialog can now show 'Download with yt-dlp' for links and text."
