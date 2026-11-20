$ErrorActionPreference = 'Stop'
if (-not $env:WINDOWS_CERTIFICATE -or -not $env:WINDOWS_CERTIFICATE_PASSWORD) {
    throw 'Set WINDOWS_CERTIFICATE and WINDOWS_CERTIFICATE_PASSWORD in the release-windows environment'
}
$file = Join-Path $env:RUNNER_TEMP 'anima-publisher.pfx'
try {
    [IO.File]::WriteAllBytes($file, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
    $password = ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force
    $certificates = @(Import-PfxCertificate -FilePath $file -CertStoreLocation Cert:\CurrentUser\My -Password $password)
    $signers = @($certificates | Where-Object { $_.HasPrivateKey -and $_.EnhancedKeyUsageList.ObjectId -contains '1.3.6.1.5.5.7.3.3' })
    if ($signers.Count -ne 1) { throw 'The PFX must contain exactly one code-signing private key' }
    $certificate = $signers[0]
    if ($certificate.NotAfter -le (Get-Date) -or $certificate.NotBefore -gt (Get-Date)) {
        throw 'The code-signing certificate is outside its validity period'
    }
    "ANIMA_WINDOWS_CERTIFICATE=$($certificate.Thumbprint)" >> $env:GITHUB_ENV
} finally {
    Remove-Item $file -Force -ErrorAction SilentlyContinue
}
