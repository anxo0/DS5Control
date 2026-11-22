# Builds a DS5Control release from the Rust version: interface, program, portable zip and
# installer, in the `publicar-rust` folder. It uploads nothing.
#
#   powershell -ExecutionPolicy Bypass -File tools\publicar-rust.ps1
#
# Needs Node, Rust, Python and Edge (for the installer screens) and Inno Setup 6.
$ErrorActionPreference = 'Stop'
$raiz = Split-Path $PSScriptRoot
Set-Location $raiz

$version = (Select-String -Path rust\Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$salida = Join-Path $raiz 'publicar-rust'
$carpeta = Join-Path $salida 'DS5Control'
"DS5Control $version (Rust)"

"== Interface"
npm --prefix ui ci --no-audit --no-fund | Out-Null
npm --prefix ui run build | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'The interface failed to build' }

"== Tests"
Push-Location rust
cargo test --workspace --release -q 2>&1 | Select-String 'test result: FAILED|error' | ForEach-Object { $_.Line }
if ($LASTEXITCODE -ne 0) { Pop-Location; throw 'The tests failed' }

"== Program"
cargo build --release -p ds5-app
if ($LASTEXITCODE -ne 0) { Pop-Location; throw 'The program failed to build' }
Pop-Location

"== Folder"
Remove-Item $salida -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $carpeta | Out-Null
Copy-Item rust\target\release\DS5Control.exe $carpeta
New-Item -ItemType Directory (Join-Path $carpeta 'drivers') | Out-Null
Copy-Item DS5Control\Drivers\* (Join-Path $carpeta 'drivers')
Copy-Item LICENSE.txt, README.md, ui\THIRD_PARTY.md $carpeta

"== Portable"
# The program updates itself from the asset that ends in -win-x64.zip.
$zip = Join-Path $salida "DS5Control-$version-portable-win-x64.zip"
Compress-Archive (Join-Path $carpeta '*') $zip

"== Installer screens"
& powershell -NoProfile -ExecutionPolicy Bypass -File installer\diseno\construir.ps1 -Version $version
if ($LASTEXITCODE -ne 0) { throw 'The installer screens failed' }

"== Installer"
$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 is missing' }
& $iscc /Q "/DVersion=$version" "/DOrigen=$carpeta" "/O$salida" installer\ds5control.iss
if ($LASTEXITCODE -ne 0) { throw 'The installer failed to build' }

Get-ChildItem $salida -File | ForEach-Object { '{0}  {1:N1} MB' -f $_.Name, ($_.Length / 1MB) }
"DONE"
