# WIN2: Build a local Windows x86_64 installer with the native local hub and PostgreSQL.
# Run in PowerShell with Rust/rustup, Bun, Go and the MSVC build tools on PATH.
param([string]$Version = '0.1.0', [switch]$Release)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$') {
    throw 'The version must be like 0.1.0'
}
$root = (Resolve-Path "$PSScriptRoot/../..").Path
$before = Get-Location
$saved = @{}
foreach ($name in @('RUSTC_WRAPPER', 'RUSTFLAGS', 'ANIMA_LOCAL_BUILD', 'VITE_ANIMA_LOCAL_BUILD', 'ANIMA_VERSION', 'GOOS', 'GOARCH', 'CGO_ENABLED')) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
function Invoke-Checked([string]$program, [string[]]$arguments) {
    & bun "$root/deploy/bin/windows-command.ts" $program @arguments
    if ($LASTEXITCODE -ne 0) { throw "$program failed with exit code $LASTEXITCODE" }
}
try {
    Set-Location $root
    # The repository's Unix sccache wrapper is not executable on Windows.
    $env:RUSTC_WRAPPER = ''
    if ($Release) {
        if ($env:ANIMA_WINDOWS_CERTIFICATE -notmatch '^[A-Fa-f0-9]{40}$') {
            throw 'Release builds require ANIMA_WINDOWS_CERTIFICATE in the Windows certificate store'
        }
        Remove-Item Env:ANIMA_LOCAL_BUILD, Env:VITE_ANIMA_LOCAL_BUILD -ErrorAction SilentlyContinue
    } else {
        $env:ANIMA_LOCAL_BUILD = '1'
        $env:VITE_ANIMA_LOCAL_BUILD = '1'
    }
    $env:ANIMA_VERSION = $Version
    $env:CGO_ENABLED = '0'
    Invoke-Checked bun @('install', '--frozen-lockfile')
    Invoke-Checked rustup @('target', 'add', 'x86_64-pc-windows-msvc')
    $binaries = "$root/apps/desktop/src-tauri/binaries"
    New-Item -ItemType Directory -Force $binaries | Out-Null
    Set-Location "$root/link"
    $env:GOOS = 'windows'
    $env:GOARCH = 'amd64'
    Invoke-Checked go @('mod', 'download', 'github.com/tailscale/tailcat')
    $tailcat = & go list -m -f '{{.Dir}}' github.com/tailscale/tailcat
    if ($LASTEXITCODE -ne 0) { throw 'Cannot locate the Tailcat build tags' }
    $tags = (Get-Content "$tailcat/build-tags.txt" | Where-Object { $_.Trim() }) -join ','
    if (-not ($tags -match '(^|,)ts_omit_portmapper(,|$)')) { throw 'Tailcat must omit port mapping' }
    Invoke-Checked go @('build', '-trimpath', '-ldflags=-s -w -H windowsgui', '-tags', $tags,
        '-o', "$binaries/anima-link-x86_64-pc-windows-msvc.exe", './cmd/anima-link')
    Set-Location $root
    # The CLI links its CRT; Tauri applies its own static-vcruntime configuration.
    $env:RUSTFLAGS = "$($saved['RUSTFLAGS']) -C target-feature=+crt-static"
    Invoke-Checked cargo @('build', '-p', 'anima-cli', '-p', 'anima-daemon', '-p', 'anima-hub',
        '-p', 'anima-local-hub', '-p', 'anima-database', '-p', 'anima-gateway', '--release',
        '--target', 'x86_64-pc-windows-msvc')
    foreach ($program in @('anima', 'anima-daemon', 'anima-hub', 'anima-local-hub', 'anima-database', 'anima-gateway')) {
        Copy-Item "$root/target/x86_64-pc-windows-msvc/release/$program.exe" `
            "$binaries/$program-x86_64-pc-windows-msvc.exe" -Force
    }
    if ($Release) {
        Get-ChildItem $binaries -Filter '*-x86_64-pc-windows-msvc.exe' | ForEach-Object {
            & "$root/deploy/windows/sign.ps1" -Path $_.FullName
        }
        Copy-Item "$binaries/anima-x86_64-pc-windows-msvc.exe" `
            "$root/target/x86_64-pc-windows-msvc/release/anima.exe" -Force
    }
    # The recipe verifies both official archives and carries the CRT beside PostgreSQL.
    # Build separately before replacing the previous generated runtime.
    $runtime = "$root/apps/desktop/src-tauri/database"
    $staged = "$root/target/windows-database-$([Guid]::NewGuid().ToString('N'))"
    Invoke-Checked bun @('apps/desktop/src-tauri/database-tree/windows.ts',
        "$root/target/windows-database-work", $staged)
    if (Test-Path $runtime) {
        if (-not (Test-Path "$runtime/anima-runtime.json")) { throw 'Refusing an unrecognized database folder' }
        Remove-Item $runtime -Recurse -Force
    }
    Move-Item $staged $runtime
    $env:RUSTFLAGS = $saved['RUSTFLAGS']
    $bundle = @{ createUpdaterArtifacts = $false }
    if ($Release) {
        $bundle.windows = @{
            certificateThumbprint = $env:ANIMA_WINDOWS_CERTIFICATE
            digestAlgorithm = 'sha256'
            timestampUrl = 'http://timestamp.digicert.com'
            tsp = $true
        }
    }
    $configuration = @{ version = $Version; bundle = $bundle } | ConvertTo-Json -Depth 5 -Compress
    $configurationPath = "$root/target/windows-bundle.json"
    [IO.File]::WriteAllText($configurationPath, $configuration)
    Invoke-Checked bun @('run', '--cwd', 'apps/desktop', 'tauri', 'build',
        '--target', 'x86_64-pc-windows-msvc', '--config', 'src-tauri/tauri.windows.conf.json', '--config', $configurationPath)
} finally {
    Set-Location $before
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
