[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$packages = @(Get-AppxPackage -Name 'Teamy.ShareToYtDlp')
if ($packages.Count -eq 0) {
    Write-Host 'Download with yt-dlp is not installed.'
    return
}

$packages | Remove-AppxPackage
Write-Host 'Uninstalled Download with yt-dlp.'
