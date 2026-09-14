[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectDirectory = $PSScriptRoot
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Visual Studio Installer (vswhere.exe) was not found. Install Visual Studio 2022 with the Desktop development with C++ workload.'
}

$visualStudioDirectory = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
if (-not $visualStudioDirectory) {
    throw 'Visual Studio 2022 C++ build tools were not found. Install the Desktop development with C++ workload.'
}

$msbuild = Join-Path $visualStudioDirectory 'MSBuild\Current\Bin\MSBuild.exe'

if (-not (Test-Path -LiteralPath $msbuild)) {
    throw "MSBuild was not found at $msbuild"
}

& $msbuild (Join-Path $projectDirectory 'ShareToYtDlp.vcxproj') /nologo /m /p:Configuration=Release /p:Platform=x64
if ($LASTEXITCODE -ne 0) {
    throw "MSBuild failed with exit code $LASTEXITCODE"
}

$packageDirectory = Join-Path $projectDirectory 'package'
$assetDirectory = Join-Path $packageDirectory 'Assets'
$null = New-Item -ItemType Directory -Path $assetDirectory -Force
Copy-Item -LiteralPath (Join-Path $projectDirectory 'bin\x64\Release\ShareToYtDlp.exe') -Destination $packageDirectory -Force

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

Write-Host "Package layout built at $packageDirectory"
