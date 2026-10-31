# Redeploy personal CJK Degauss binary to the MiSTer TF share.
$ErrorActionPreference = "Stop"
$src = Join-Path $PSScriptRoot "target\armv7-unknown-linux-musleabihf\release\degauss"
$dst = "\\192.168.31.48\sdcard\Scripts\.config\degauss\degauss"
if (-not (Test-Path -LiteralPath $src)) { throw "Build missing: $src" }
$bytes = [System.IO.File]::ReadAllBytes($src)
[System.IO.File]::WriteAllBytes($dst, $bytes)
Write-Host "Deployed $($bytes.Length) bytes to $dst"
