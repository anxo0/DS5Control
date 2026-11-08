# Renders the installer screens for a version into installer\piezas\*.bmp, which ds5control.iss uses.
# Needs Python, Node and Edge, and ui\node_modules for the Figtree font.
param([Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = 'Stop'
$aqui = $PSScriptRoot
$construido = Join-Path $aqui 'construido'
python -I (Join-Path $aqui 'generar.py') $Version | Out-Null
$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Falta Microsoft Edge para dibujar el instalador' }
Remove-Item (Join-Path $construido 'piezas') -Recurse -Force -ErrorAction SilentlyContinue
node (Join-Path $aqui 'piezas.mjs') $edge $construido | Out-Null

Add-Type -AssemblyName System.Drawing
$destino = Join-Path (Split-Path $aqui) 'piezas'
Remove-Item $destino -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $destino | Out-Null
foreach ($png in Get-ChildItem (Join-Path $construido 'piezas\*.png')) {
    $imagen = [System.Drawing.Image]::FromFile($png.FullName)
    $bmp = New-Object System.Drawing.Bitmap $imagen.Width, $imagen.Height, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.ColorTranslator]::FromHtml('#101114'))
    $g.DrawImage($imagen, 0, 0, $imagen.Width, $imagen.Height)
    $g.Dispose()
    $bmp.Save((Join-Path $destino ($png.BaseName + '.bmp')), [System.Drawing.Imaging.ImageFormat]::Bmp)
    $bmp.Dispose(); $imagen.Dispose()
}
"Instalador dibujado para la version ${Version}: $((Get-ChildItem "$destino\*.bmp").Count) piezas"
