param([Parameter(Mandatory)][string]$Path)
$ErrorActionPreference = 'Stop'
if ($env:ANIMA_WINDOWS_CERTIFICATE -notmatch '^[A-Fa-f0-9]{40}$') {
    throw 'A Windows code-signing certificate thumbprint is required'
}
& signtool sign /sha1 $env:ANIMA_WINDOWS_CERTIFICATE /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $Path
if ($LASTEXITCODE -ne 0) { throw "Authenticode signing failed: $Path" }
& signtool verify /pa /all /tw $Path
if ($LASTEXITCODE -ne 0) { throw "Authenticode verification failed: $Path" }
$signature = Get-AuthenticodeSignature -LiteralPath $Path
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $env:ANIMA_WINDOWS_CERTIFICATE -or -not $signature.TimeStamperCertificate) {
    throw "The signature, publisher or timestamp is invalid: $Path"
}
