[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
$downloader = Join-Path $projectDirectory 'package\download.ps1'
$testOutput = Join-Path $PSScriptRoot 'output'
$log = Join-Path $PSScriptRoot 'yt-dlp-invocations.log'

$null = New-Item -ItemType Directory -Path $testOutput -Force
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
Get-ChildItem -LiteralPath $testOutput -Filter 'share-to-ytdlp-failure-*.log' |
    Remove-Item -Force

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
    $failureLogs = @(Get-ChildItem -LiteralPath $testOutput -Filter 'share-to-ytdlp-failure-*.log')
    if ($failureLogs.Count -ne 1) {
        throw "Expected one failure log after the subtitle failure, got $($failureLogs.Count)."
    }
    if ((Get-Content -LiteralPath $failureLogs[0].FullName -Raw) -notmatch 'optional subtitle pass failed') {
        throw 'Expected the subtitle failure to be recorded in the failure log.'
    }

    # A failed media download must remain a real failure and skip subtitles.
    Remove-Item -LiteralPath $log -Force
    $env:FAKE_YTDLP_MEDIA_EXIT = '9'
    $env:FAKE_YTDLP_SUBTITLE_EXIT = '0'
    $failureOutput = @('', '', '') |
        & pwsh -NoLogo -NoProfile -File $downloader -Url 'https://x.com/example/status/2' -OutputDirectory $testOutput 2>&1
    $mediaFailureExitCode = $LASTEXITCODE
    if ($mediaFailureExitCode -ne 9) {
        throw "Expected media failure exit code 9, got $mediaFailureExitCode"
    }
    $failureOutputText = $failureOutput -join [Environment]::NewLine
    $acknowledgementPrompts = [regex]::Matches($failureOutputText, 'Press Enter \([123]/3\)').Count
    if ($acknowledgementPrompts -ne 3) {
        throw "Expected three Enter acknowledgement prompts, got $acknowledgementPrompts."
    }
    if (@(Get-Content -LiteralPath $log).Count -ne 1) {
        throw 'Expected subtitle pass to be skipped after media failure.'
    }
    $failureLogs = @(Get-ChildItem -LiteralPath $testOutput -Filter 'share-to-ytdlp-failure-*.log')
    if ($failureLogs.Count -ne 2) {
        throw "Expected a second failure log after the media failure, got $($failureLogs.Count)."
    }
    if (-not ($failureLogs | Where-Object {
                (Get-Content -LiteralPath $_.FullName -Raw) -match 'Media download failed'
            })) {
        throw 'Expected the media failure to be recorded in a failure log.'
    }

    # A completely successful run must discard its temporary transcript.
    Remove-Item -LiteralPath $log -Force
    $env:FAKE_YTDLP_MEDIA_EXIT = '0'
    $env:FAKE_YTDLP_SUBTITLE_EXIT = '0'
    & pwsh -NoLogo -NoProfile -File $downloader -Url 'https://x.com/example/status/3' -OutputDirectory $testOutput -NoPause
    if ($LASTEXITCODE -ne 0) {
        throw "Expected a clean run to succeed, got exit code $LASTEXITCODE"
    }
    $failureLogs = @(Get-ChildItem -LiteralPath $testOutput -Filter 'share-to-ytdlp-failure-*.log')
    if ($failureLogs.Count -ne 2) {
        throw 'Expected a clean run not to create another failure log.'
    }

    Write-Host 'Downloader tests passed.' -ForegroundColor Green
}
finally {
    $env:PATH = $originalPath
    Remove-Item Env:FAKE_YTDLP_LOG,Env:FAKE_YTDLP_MEDIA_EXIT,Env:FAKE_YTDLP_SUBTITLE_EXIT -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $testOutput -Filter 'share-to-ytdlp-failure-*.log' |
        Remove-Item -Force
}

exit 0
