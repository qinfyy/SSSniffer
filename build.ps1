param([switch]$Debug)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

if (-not (Get-Command nmake.exe -ErrorAction SilentlyContinue)) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $visualStudio) { throw 'Install the Visual Studio C++ build tools required by detours-rs.' }
    Import-Module (Join-Path $visualStudio 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
    Enter-VsDevShell -VsInstallPath $visualStudio -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64'
}

$arguments = @('build')
$profile = 'debug'
if (-not $Debug) { $arguments += '--release'; $profile = 'release' }
& cargo @arguments
if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'StandaloneStart.bat') -Destination (Join-Path $PSScriptRoot "target\$profile\StandaloneStart.bat") -Force
Write-Host "Built target\$profile\SSSniffer.dll"
