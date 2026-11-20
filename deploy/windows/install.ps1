param([Parameter(Mandatory)][string]$Installer, [Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = 'Stop'
$process = Start-Process -FilePath (Resolve-Path $Installer) -ArgumentList '/S' -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
$installed = Join-Path $env:LOCALAPPDATA 'Anima.ing'
$root = (Resolve-Path "$PSScriptRoot/../..").Path
foreach ($program in @('anima', 'anima-link', 'anima-daemon', 'anima-hub', 'anima-local-hub', 'anima-database', 'anima-gateway')) {
    $source = "$root/apps/desktop/src-tauri/binaries/$program-x86_64-pc-windows-msvc.exe"
    $destination = Join-Path $installed "$program.exe"
    if ((Get-FileHash $source).Hash -ne (Get-FileHash $destination).Hash) {
        throw "The installed $program differs from the packaged program"
    }
}
$cli = Join-Path $installed 'anima.exe'
if ((& $cli --version) -ne "anima $Version" -or $LASTEXITCODE -ne 0) {
    throw 'The installed CLI did not start at the packaged version'
}
$postgres = Join-Path $installed 'database/bin/postgres.exe'
$pin = Get-Content "$root/apps/desktop/src-tauri/database-tree/windows.pin.json" -Raw | ConvertFrom-Json
if ((& $postgres --version) -ne "postgres (PostgreSQL) $($pin.postgresql.version)" -or $LASTEXITCODE -ne 0) {
    throw 'The installed PostgreSQL runtime did not start at the pinned version'
}
