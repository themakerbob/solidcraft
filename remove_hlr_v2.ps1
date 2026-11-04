$ErrorActionPreference = 'Stop'
$lib = '.\crates\geom\src\lib.rs'
if (-not (Test-Path $lib)) { throw "Run from SolidCraft repository root" }
$txt = [System.IO.File]::ReadAllText((Resolve-Path $lib))
$next = [regex]::Replace($txt, '(?m)^\s*pub\s+mod\s+hlr_v2\s*;\s*\r?\n', '')
$next = [regex]::Replace($next, '(?m)^\s*mod\s+hlr_v2\s*;\s*\r?\n', '')
if ($next -ne $txt) { [System.IO.File]::WriteAllText((Resolve-Path $lib), $next) }
$module = '.\crates\geom\src\hlr_v2.rs'
if (Test-Path $module) { Remove-Item $module }
Write-Host 'HLR V2 module declaration and file removed; HLR V1 remains.'
