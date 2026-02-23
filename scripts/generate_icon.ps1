Add-Type -AssemblyName System.Drawing

$width = 512
$height = 512
$bmp = New-Object System.Drawing.Bitmap $width, $height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

# Background: Clear
$g.Clear([System.Drawing.Color]::Transparent)

# Draw a rounded square/box background with gradient
$rect = New-Object System.Drawing.Rectangle 40, 40, 432, 432
$blue = [System.Drawing.Color]::FromArgb(255, 30, 58, 138)
$purple = [System.Drawing.Color]::FromArgb(255, 124, 58, 237)

# Use -ArgumentList for New-Object
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush -ArgumentList $rect, $blue, $purple, 45.0

$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$cornerRadius = 80
$path.AddArc($rect.X, $rect.Y, $cornerRadius, $cornerRadius, 180, 90)
$path.AddArc($rect.X + $rect.Width - $cornerRadius, $rect.Y, $cornerRadius, $cornerRadius, 270, 90)
$path.AddArc($rect.X + $rect.Width - $cornerRadius, $rect.Y + $rect.Height - $cornerRadius, $cornerRadius, $cornerRadius, 0, 90)
$path.AddArc($rect.X, $rect.Y + $rect.Height - $cornerRadius, $cornerRadius, $cornerRadius, 90, 90)
$path.CloseFigure()
$g.FillPath($brush, $path)

# Draw an arrow
$arrowColor = [System.Drawing.Color]::White
$arrowBrush = New-Object System.Drawing.SolidBrush $arrowColor

# Create points explicitly
$p1 = New-Object System.Drawing.Point 256, 380
$p2 = New-Object System.Drawing.Point 150, 250
$p3 = New-Object System.Drawing.Point 210, 250
$p4 = New-Object System.Drawing.Point 210, 130
$p5 = New-Object System.Drawing.Point 302, 130
$p6 = New-Object System.Drawing.Point 302, 250
$p7 = New-Object System.Drawing.Point 362, 250

$points = @($p1, $p2, $p3, $p4, $p5, $p6, $p7)

$g.FillPolygon($arrowBrush, $points)

$bmp.Save("src-tauri/icons/icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
$brush.Dispose()
$arrowBrush.Dispose()

Write-Host "Icon generated at src-tauri/icons/icon.png"