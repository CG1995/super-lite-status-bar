param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\src-tauri\icons")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function New-StatusIconPng {
    param(
        [int]$Size,
        [string]$Path
    )

    $bmp = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $pad = [single][Math]::Max(1.0, $Size * 0.11)
    $radius = [single][Math]::Max(4.0, $Size * 0.20)
    $rect = [System.Drawing.RectangleF]::new($pad, $pad, [single]($Size - 2 * $pad), [single]($Size - 2 * $pad))
    $pathObj = New-Object System.Drawing.Drawing2D.GraphicsPath
    $diameter = $radius * 2
    $pathObj.AddArc($rect.X, $rect.Y, $diameter, $diameter, 180, 90)
    $pathObj.AddArc($rect.Right - $diameter, $rect.Y, $diameter, $diameter, 270, 90)
    $pathObj.AddArc($rect.Right - $diameter, $rect.Bottom - $diameter, $diameter, $diameter, 0, 90)
    $pathObj.AddArc($rect.X, $rect.Bottom - $diameter, $diameter, $diameter, 90, 90)
    $pathObj.CloseFigure()

    $bg = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $rect,
        [System.Drawing.Color]::FromArgb(255, 26, 33, 41),
        [System.Drawing.Color]::FromArgb(255, 39, 49, 59),
        [single]35.0
    )
    $g.FillPath($bg, $pathObj)

    $strokeWidth = [single][Math]::Max(1.0, $Size * 0.035)
    $border = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(190, 230, 236, 240), $strokeWidth)
    $g.DrawPath($border, $pathObj)

    $accent = [System.Drawing.Color]::FromArgb(255, 90, 196, 138)
    $accentBrush = New-Object System.Drawing.SolidBrush($accent)
    $mutedBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 118, 133, 145))
    $lightBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 236, 246, 240))

    $barWidth = [single][Math]::Max(2.0, $Size * 0.115)
    $gap = [single][Math]::Max(1.5, $Size * 0.065)
    $base = [single]($Size * 0.70)
    $startX = [single](($Size - (3 * $barWidth + 2 * $gap)) / 2)
    $heights = @([single]($Size * 0.24), [single]($Size * 0.38), [single]($Size * 0.52))

    for ($i = 0; $i -lt 3; $i++) {
        $x = $startX + $i * ($barWidth + $gap)
        $h = $heights[$i]
        $y = $base - $h
        $barRect = [System.Drawing.RectangleF]::new([single]$x, [single]$y, [single]$barWidth, [single]$h)
        $barPath = New-Object System.Drawing.Drawing2D.GraphicsPath
        $barRadius = [single][Math]::Max(1.2, $barWidth * 0.42)
        $d = $barRadius * 2
        $barPath.AddArc($barRect.X, $barRect.Y, $d, $d, 180, 90)
        $barPath.AddArc($barRect.Right - $d, $barRect.Y, $d, $d, 270, 90)
        $barPath.AddArc($barRect.Right - $d, $barRect.Bottom - $d, $d, $d, 0, 90)
        $barPath.AddArc($barRect.X, $barRect.Bottom - $d, $d, $d, 90, 90)
        $barPath.CloseFigure()
        $g.FillPath($(if ($i -eq 2) { $accentBrush } elseif ($i -eq 1) { $lightBrush } else { $mutedBrush }), $barPath)
        $barPath.Dispose()
    }

    $dotSize = [single][Math]::Max(2.0, $Size * 0.085)
    $dotRect = [System.Drawing.RectangleF]::new([single]($Size * 0.66), [single]($Size * 0.235), $dotSize, $dotSize)
    $g.FillEllipse($accentBrush, $dotRect)

    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)

    $bg.Dispose()
    $border.Dispose()
    $accentBrush.Dispose()
    $mutedBrush.Dispose()
    $lightBrush.Dispose()
    $pathObj.Dispose()
    $g.Dispose()
    $bmp.Dispose()
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
foreach ($size in $sizes) {
    New-StatusIconPng -Size $size -Path (Join-Path $OutDir "$($size)x$($size).png")
}

$icoImages = @()
foreach ($size in $sizes) {
    $pngPath = Join-Path $OutDir "$($size)x$($size).png"
    $bitmap = [System.Drawing.Bitmap]::FromFile($pngPath)
    $imageStream = New-Object IO.MemoryStream
    $writer = New-Object IO.BinaryWriter $imageStream

    $maskStride = [int]([Math]::Ceiling($size / 32.0) * 4)
    $xorSize = $size * $size * 4
    $maskSize = $maskStride * $size

    $writer.Write([UInt32]40)
    $writer.Write([Int32]$size)
    $writer.Write([Int32]($size * 2))
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]32)
    $writer.Write([UInt32]0)
    $writer.Write([UInt32]($xorSize + $maskSize))
    $writer.Write([Int32]0)
    $writer.Write([Int32]0)
    $writer.Write([UInt32]0)
    $writer.Write([UInt32]0)

    for ($y = $size - 1; $y -ge 0; $y--) {
        for ($x = 0; $x -lt $size; $x++) {
            $color = $bitmap.GetPixel($x, $y)
            $writer.Write([byte]$color.B)
            $writer.Write([byte]$color.G)
            $writer.Write([byte]$color.R)
            $writer.Write([byte]$color.A)
        }
    }

    $writer.Write([byte[]](New-Object byte[] $maskSize))
    $writer.Flush()
    $icoImages += ,@{
        Size = $size
        Bytes = $imageStream.ToArray()
    }
    $writer.Dispose()
    $imageStream.Dispose()
    $bitmap.Dispose()
}

$headerSize = 6 + 16 * $icoImages.Count
$offset = $headerSize
$ms = New-Object IO.MemoryStream
$bw = New-Object IO.BinaryWriter $ms
$bw.Write([UInt16]0)
$bw.Write([UInt16]1)
$bw.Write([UInt16]$icoImages.Count)
for ($i = 0; $i -lt $icoImages.Count; $i++) {
    $size = $icoImages[$i].Size
    $bytes = $icoImages[$i].Bytes
    $encodedSize = if ($size -eq 256) { 0 } else { $size }
    $bw.Write([byte]$encodedSize)
    $bw.Write([byte]$encodedSize)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]32)
    $bw.Write([UInt32]$bytes.Length)
    $bw.Write([UInt32]$offset)
    $offset += $bytes.Length
}
foreach ($entry in $icoImages) {
    $bw.Write([byte[]]$entry.Bytes)
}
$bw.Flush()
[IO.File]::WriteAllBytes((Join-Path $OutDir "icon.ico"), $ms.ToArray())
$bw.Dispose()
$ms.Dispose()

Get-ChildItem $OutDir | Select-Object Name, Length
