param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\src-tauri\icons")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName PresentationCore,WindowsBase
Add-Type -AssemblyName System.Drawing

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$LogoPathData = "m50 95c-24.801 0-45-20.199-45-45.102 0-23.5 17.602-42.797 40.898-44.898 2.1992-0.19922 4.1016 1.3984 4.3008 3.6016 0.19922 2.1992-1.3984 4.1016-3.6016 4.3008-19.098 1.7969-33.598 17.699-33.598 36.996 0 20.5 16.602 37.102 37 37.102 7.6992 0 15-2.3008 21.301-6.8008 1.8008-1.3008 4.3008-0.89844 5.6016 0.89844 1.3008 1.8008 0.89844 4.3008-0.89844 5.6016-7.7031 5.4023-16.605 8.3008-26.004 8.3008zm34-18.199c-0.80078 0-1.6016-0.19922-2.1992-0.69922-1.8008-1.1992-2.3008-3.6992-1.1016-5.6016 2.8008-4.1016 4.6016-8.6016 5.6016-13.398 0.5-2.3008 0.69922-4.6992 0.69922-7.1992 0-11.102-4.8984-21.5-13.5-28.602-4-3.3008-8.6016-5.6992-13.398-7-2.1016-0.60156-3.3984-2.8008-2.8008-4.8984 0.60156-2.1016 2.8008-3.3984 4.8984-2.8008 5.8984 1.6016 11.398 4.5 16.398 8.6016 10.402 8.6953 16.402 21.297 16.402 34.797 0 3-0.30078 5.8984-0.89844 8.6992-1.1016 5.8984-3.3984 11.398-6.8008 16.398-0.69922 1.1016-2 1.7031-3.3008 1.7031z"
$LogoColor = [System.Windows.Media.Color]::FromRgb(0x17, 0x69, 0xff)

function New-StatusIconPng {
    param(
        [int]$Size,
        [string]$Path
    )

    $geometry = [System.Windows.Media.Geometry]::Parse($LogoPathData)
    $scale = $Size / 110.0
    $group = [System.Windows.Media.TransformGroup]::new()
    $group.Children.Add([System.Windows.Media.TranslateTransform]::new(5, 10))
    $group.Children.Add([System.Windows.Media.ScaleTransform]::new($scale, $scale))

    $visual = [System.Windows.Media.DrawingVisual]::new()
    $context = $visual.RenderOpen()
    $brush = [System.Windows.Media.SolidColorBrush]::new($LogoColor)
    $context.PushTransform($group)
    $context.DrawGeometry($brush, $null, $geometry)
    $context.Pop()
    $context.Close()

    $bitmap = [System.Windows.Media.Imaging.RenderTargetBitmap]::new(
        $Size,
        $Size,
        96,
        96,
        [System.Windows.Media.PixelFormats]::Pbgra32
    )
    $bitmap.Render($visual)

    $encoder = [System.Windows.Media.Imaging.PngBitmapEncoder]::new()
    $encoder.Frames.Add([System.Windows.Media.Imaging.BitmapFrame]::Create($bitmap))
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Create)
    try {
        $encoder.Save($stream)
    } finally {
        $stream.Dispose()
    }
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
