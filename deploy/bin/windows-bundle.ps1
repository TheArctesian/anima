# WIN1: Build a local Windows x86_64 installer, including the device link and native CLI.
# Run in PowerShell with Rust/rustup, Bun, Go and the MSVC build tools on PATH.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot/../..").Path
$before = Get-Location
$saved = @{}
foreach ($name in @('RUSTC_WRAPPER', 'RUSTFLAGS', 'ANIMA_LOCAL_BUILD', 'GOOS', 'GOARCH')) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
function Invoke-Checked([string]$program, [string[]]$arguments) {
    & $program @arguments
    if ($LASTEXITCODE -ne 0) { throw "$program failed with exit code $LASTEXITCODE" }
}
try {
    Set-Location $root
    # The repository's Unix sccache wrapper is not executable on Windows.
    $env:RUSTC_WRAPPER = ''
    $env:ANIMA_LOCAL_BUILD = '1'
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
    Invoke-Checked go @('build', '-trimpath', '-ldflags=-s -w', '-tags', $tags,
        '-o', "$binaries/anima-link-x86_64-pc-windows-msvc.exe", './cmd/anima-link')
    Set-Location $root
    # The CLI links its CRT; Tauri applies its own static-vcruntime configuration.
    $env:RUSTFLAGS = "$($saved['RUSTFLAGS']) -C target-feature=+crt-static"
    Invoke-Checked cargo @('build', '-p', 'anima-cli', '--bin', 'anima', '--release',
        '--target', 'x86_64-pc-windows-msvc')
    Copy-Item "$root/target/x86_64-pc-windows-msvc/release/anima.exe" `
        "$binaries/anima-x86_64-pc-windows-msvc.exe" -Force
    $env:RUSTFLAGS = $saved['RUSTFLAGS']
    Invoke-Checked bun @('run', '--cwd', 'apps/desktop', 'tauri', 'build',
        '--target', 'x86_64-pc-windows-msvc', '--config', 'src-tauri/tauri.windows.conf.json')
} finally {
    Set-Location $before
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
