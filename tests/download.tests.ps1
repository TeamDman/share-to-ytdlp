[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
$downloader = Join-Path $projectDirectory 'package\download.ps1'
$testOutput = Join-Path $PSScriptRoot 'output'
$log = Join-Path $PSScriptRoot 'yt-dlp-invocations.log'

$null = New-Item -ItemType Directory -Path $testOutput -Force
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue

$originalPath = $env:PATH
try {
    $env:PATH = "$PSScriptRoot;$originalPath"
    $env:FAKE_YTDLP_LOG = $log

    # A failed optional subtitle request must not fail a successful media download.
    $env:FAKE_YTDLP_MEDIA_EXIT = '0'
    $env:FAKE_YTDLP_SUBTITLE_EXIT = '7'
    & pwsh -NoLogo -NoProfile -File $downloader -Url 'https://x.com/example/status/1' -OutputDirectory $testOutput -NoPause
    if ($LASTEXITCODE -ne 0) {
        throw "Expected subtitle failure to be ignored, got exit code $LASTEXITCODE"
    }
    if (@(Get-Content -LiteralPath $log).Count -ne 2) {
        throw 'Expected separate media and subtitle yt-dlp invocations.'
    }

    # A failed media download must remain a real failure and skip subtitles.
    Remove-Item -LiteralPath $log -Force
    $env:FAKE_YTDLP_MEDIA_EXIT = '9'
    $env:FAKE_YTDLP_SUBTITLE_EXIT = '0'
    & pwsh -NoLogo -NoProfile -File $downloader -Url 'https://x.com/example/status/2' -OutputDirectory $testOutput -NoPause
    if ($LASTEXITCODE -ne 9) {
        throw "Expected media failure exit code 9, got $LASTEXITCODE"
    }
    if (@(Get-Content -LiteralPath $log).Count -ne 1) {
        throw 'Expected subtitle pass to be skipped after media failure.'
    }

    Write-Host 'Downloader tests passed.' -ForegroundColor Green
}
finally {
    $env:PATH = $originalPath
    Remove-Item Env:FAKE_YTDLP_LOG,Env:FAKE_YTDLP_MEDIA_EXIT,Env:FAKE_YTDLP_SUBTITLE_EXIT -ErrorAction SilentlyContinue
}
