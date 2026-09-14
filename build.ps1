[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectDirectory = $PSScriptRoot

$cargo = Get-Command cargo -CommandType Application -ErrorAction Stop | Select-Object -First 1
& $cargo.Source build --release --locked --bins
if ($LASTEXITCODE -ne 0) {
    throw "Cargo build failed with exit code $LASTEXITCODE"
}

$shareTargetExecutable = Join-Path $projectDirectory 'target\release\share-to-ytdlp-share-target.exe'
if (-not (Test-Path -LiteralPath $shareTargetExecutable -PathType Leaf)) {
    throw "Share-target executable was not produced at $shareTargetExecutable"
}

$packageDirectory = Join-Path $projectDirectory 'package'
$assetDirectory = Join-Path $packageDirectory 'Assets'
$null = New-Item -ItemType Directory -Path $assetDirectory -Force
Copy-Item -LiteralPath $shareTargetExecutable -Destination (Join-Path $packageDirectory 'ShareToYtDlp.exe') -Force

Add-Type -AssemblyName System.Drawing
function New-Logo([string] $Path, [int] $Size) {
    $bitmap = [Drawing.Bitmap]::new($Size, $Size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.Clear([Drawing.Color]::FromArgb(255, 30, 24, 48))

        $purpleBrush = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(255, 124, 81, 255))
        $whiteBrush = [Drawing.SolidBrush]::new([Drawing.Color]::White)
        try {
            $padding = [Math]::Max(2, [int]($Size * 0.14))
            $graphics.FillEllipse($purpleBrush, $padding, $padding, $Size - 2 * $padding, $Size - 2 * $padding)

            $center = $Size / 2.0
            $shaftWidth = [Math]::Max(2, [int]($Size * 0.12))
            $shaftTop = [int]($Size * 0.25)
            $shaftBottom = [int]($Size * 0.55)
            $graphics.FillRectangle($whiteBrush, [int]($center - $shaftWidth / 2), $shaftTop, $shaftWidth, $shaftBottom - $shaftTop)

            $arrow = [Drawing.PointF[]] @(
                [Drawing.PointF]::new([float]($Size * 0.29), [float]($Size * 0.50)),
                [Drawing.PointF]::new([float]($Size * 0.71), [float]($Size * 0.50)),
                [Drawing.PointF]::new([float]($Size * 0.50), [float]($Size * 0.72))
            )
            $graphics.FillPolygon($whiteBrush, $arrow)
            $graphics.FillRectangle($whiteBrush, [int]($Size * 0.29), [int]($Size * 0.76), [int]($Size * 0.42), [Math]::Max(2, [int]($Size * 0.07)))
        }
        finally {
            $purpleBrush.Dispose()
            $whiteBrush.Dispose()
        }

        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

New-Logo (Join-Path $assetDirectory 'StoreLogo.png') 50
New-Logo (Join-Path $assetDirectory 'Square44x44Logo.png') 44
New-Logo (Join-Path $assetDirectory 'Square150x150Logo.png') 150

Write-Host "Rust package layout built at $packageDirectory"
Write-Host "CLI executable: $(Join-Path $projectDirectory 'target\release\share-to-ytdlp.exe')"
