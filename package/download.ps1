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

function Wait-BeforeExit {
    if ($NoPause) {
        return
    }

    Write-Host
    Write-Host 'Finished. Press any key to close this window.' -ForegroundColor Cyan
    try {
        $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
    }
    catch {
        $null = Read-Host
    }
}

function Complete-Script([int] $ExitCode) {
    Wait-BeforeExit
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

    $parsedUrl = $null
    if (-not [Uri]::TryCreate($Url, [UriKind]::Absolute, [ref] $parsedUrl) -or
        $parsedUrl.Scheme -notin @('http', 'https')) {
        throw 'The shared value is not a valid HTTP or HTTPS URL.'
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
    $null = New-Item -ItemType Directory -Path $OutputDirectory -Force
    Set-Location -LiteralPath $OutputDirectory

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
        Write-Host "Video is safe; the optional subtitle pass failed (exit code $subtitleExitCode)." -ForegroundColor Yellow
    }

    Complete-Script 0
}
catch {
    Write-Host
    Write-Host "Download failed: $($_.Exception.Message)" -ForegroundColor Red
    Complete-Script 1
}
