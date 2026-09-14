[CmdletBinding(DefaultParameterSetName = 'UrlFile')]
param(
    [Parameter(Mandatory, ParameterSetName = 'UrlFile')]
    [string] $UrlFile,

    [Parameter(Mandatory, ParameterSetName = 'Url')]
    [string] $Url,

    [string] $OutputDirectory,

    [ValidateSet('', 'brave', 'chrome', 'edge', 'firefox')]
    [string] $CookiesFromBrowser = '',

    [switch] $NoPause
)

$ErrorActionPreference = 'Stop'
$script:RunHadFailure = $false
$script:TranscriptPath = $null
$script:TranscriptStarted = $false

function Start-RunLog {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
    $script:TranscriptPath = Join-Path ([IO.Path]::GetTempPath()) "share-to-ytdlp-$timestamp-$PID.log"

    try {
        Start-Transcript -Path $script:TranscriptPath -Force | Out-Null
        $script:TranscriptStarted = $true
    }
    catch {
        Write-Warning "Could not start the run log: $($_.Exception.Message)"
        $script:TranscriptPath = $null
    }
}

function Finalize-RunLog([bool] $KeepLog) {
    if (-not $script:TranscriptStarted) {
        return
    }

    try {
        Stop-Transcript | Out-Null
    }
    catch {
        Write-Warning "Could not finish the run log: $($_.Exception.Message)"
    }
    finally {
        $script:TranscriptStarted = $false
    }

    if (-not $KeepLog) {
        Remove-Item -LiteralPath $script:TranscriptPath -Force -ErrorAction SilentlyContinue
        return
    }

    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $failureLog = Join-Path $OutputDirectory "share-to-ytdlp-failure-$timestamp-$PID.log"
    try {
        Move-Item -LiteralPath $script:TranscriptPath -Destination $failureLog -Force
        Write-Host
        Write-Host "Failure log saved to: $failureLog" -ForegroundColor Yellow
    }
    catch {
        Write-Host
        Write-Host "Could not move the failure log into the download folder: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "The temporary log may still be available at: $script:TranscriptPath" -ForegroundColor Yellow
    }
}

function Wait-BeforeExit([bool] $Failure) {
    if ($NoPause) {
        return
    }

    Write-Host
    if ($Failure) {
        Write-Host 'A failure occurred. Press Enter three times to acknowledge it and close this window.' -ForegroundColor Yellow
        foreach ($press in 1..3) {
            Write-Host "Press Enter ($press/3): " -ForegroundColor Yellow -NoNewline
            try {
                $null = [Console]::ReadLine()
            }
            catch {
                $null = Read-Host
            }
        }
        return
    }

    Write-Host 'Finished. Press any key to close this window.' -ForegroundColor Cyan
    try {
        $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
    }
    catch {
        $null = Read-Host
    }
}

function Complete-Script([int] $ExitCode) {
    $keepLog = $script:RunHadFailure -or $ExitCode -ne 0
    Finalize-RunLog $keepLog
    Wait-BeforeExit $keepLog
    exit $ExitCode
}

try {
    if ($PSCmdlet.ParameterSetName -eq 'UrlFile') {
        try {
            $Url = [IO.File]::ReadAllText($UrlFile, [Text.Encoding]::UTF8).Trim()
        }
        finally {
            Remove-Item -LiteralPath $UrlFile -Force -ErrorAction SilentlyContinue
        }
    }

    if (-not $OutputDirectory) {
        $settingsDirectory = Join-Path $env:LOCALAPPDATA 'ShareToYtDlp'
        $outputOverride = Join-Path $settingsDirectory 'output-directory.txt'
        if (Test-Path -LiteralPath $outputOverride) {
            $OutputDirectory = (Get-Content -LiteralPath $outputOverride -Raw).Trim()
        }
    }

    if (-not $OutputDirectory) {
        try {
            $OutputDirectory = (New-Object -ComObject Shell.Application).NameSpace('shell:Downloads').Self.Path
        }
        catch {
            $OutputDirectory = Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Downloads'
        }
    }

    $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
    $null = New-Item -ItemType Directory -Path $OutputDirectory -Force
    Set-Location -LiteralPath $OutputDirectory
    Start-RunLog

    $parsedUrl = $null
    if (-not [Uri]::TryCreate($Url, [UriKind]::Absolute, [ref] $parsedUrl) -or
        $parsedUrl.Scheme -notin @('http', 'https')) {
        throw 'The shared value is not a valid HTTP or HTTPS URL.'
    }

    if (-not $CookiesFromBrowser) {
        $cookieOverride = Join-Path $env:LOCALAPPDATA 'ShareToYtDlp\cookies-browser.txt'
        if (Test-Path -LiteralPath $cookieOverride) {
            $configuredBrowser = (Get-Content -LiteralPath $cookieOverride -Raw).Trim().ToLowerInvariant()
            if ($configuredBrowser -in @('brave', 'chrome', 'edge', 'firefox')) {
                $CookiesFromBrowser = $configuredBrowser
            }
        }
    }

    # Get-Command can return more than one executable when yt-dlp exists in
    # multiple PATH entries. Invoke the first match instead of stringifying all.
    $ytDlp = Get-Command yt-dlp -CommandType Application -ErrorAction Stop | Select-Object -First 1

    $commonArguments = @(
        '--windows-filenames'
        '--no-playlist'
    )
    if ($CookiesFromBrowser) {
        $commonArguments += @('--cookies-from-browser', $CookiesFromBrowser)
    }

    Write-Host "Downloading media to: $OutputDirectory" -ForegroundColor Cyan
    & $ytDlp.Source @commonArguments '--write-info-json' '--embed-metadata' '--' $parsedUrl.AbsoluteUri
    $mediaExitCode = $LASTEXITCODE

    if ($mediaExitCode -ne 0) {
        $script:RunHadFailure = $true
        Write-Host
        Write-Host "Media download failed (yt-dlp exit code $mediaExitCode)." -ForegroundColor Red
        Complete-Script $mediaExitCode
    }

    Write-Host
    Write-Host 'Media download completed. Trying subtitles separately...' -ForegroundColor Green

    # Subtitles are deliberately a second, best-effort operation. If a site has
    # no subtitles (or its subtitle endpoint fails), the downloaded video remains
    # a success and this script still exits successfully.
    & $ytDlp.Source @commonArguments '--skip-download' '--write-subs' '--write-auto-subs' '--' $parsedUrl.AbsoluteUri
    $subtitleExitCode = $LASTEXITCODE

    Write-Host
    if ($subtitleExitCode -eq 0) {
        Write-Host 'Done. Subtitles were saved when available.' -ForegroundColor Green
    }
    else {
        $script:RunHadFailure = $true
        Write-Host "Video is safe; the optional subtitle pass failed (exit code $subtitleExitCode)." -ForegroundColor Yellow
    }

    Complete-Script 0
}
catch {
    $script:RunHadFailure = $true
    Write-Host
    Write-Host "Download failed: $($_.Exception.Message)" -ForegroundColor Red
    Complete-Script 1
}
