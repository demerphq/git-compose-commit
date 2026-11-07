param(
    [Parameter(Mandatory=$true)][string]$Version,
    [string]$InstallDir = (Join-Path $HOME '.local\bin')
)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^v[0-9A-Za-z][0-9A-Za-z.-]*$') { throw 'Invalid version' }
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw 'Only Windows x86-64 is supported'
}
$archive = "git-compose-commit-$Version-x86_64-pc-windows-msvc.zip"
$base = if ($env:GIT_COMPOSE_COMMIT_RELEASE_BASE) { $env:GIT_COMPOSE_COMMIT_RELEASE_BASE } else { 'https://github.com/demerphq/git-compose-commit/releases/download' }
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory $tempDir | Out-Null
try {
    $file = Join-Path $tempDir $archive
    Invoke-WebRequest "$base/$Version/$archive" -OutFile $file
    $check = Join-Path $tempDir 'checksum'
    Invoke-WebRequest "$base/$Version/$archive.sha256" -OutFile $check
    $expected = ((Get-Content $check -Raw) -split '\s+')[0]
    if ($expected -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid checksum' }
    if ((Get-FileHash -Algorithm SHA256 $file).Hash -ne $expected) { throw 'Checksum mismatch' }
    Expand-Archive -LiteralPath $file -DestinationPath (Join-Path $tempDir 'unpacked')
    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    Copy-Item (Join-Path $tempDir 'unpacked\git-compose-commit.exe') (Join-Path $InstallDir 'git-compose-commit.exe') -Force
    Copy-Item (Join-Path $tempDir 'unpacked\compose-text.exe') (Join-Path $InstallDir 'compose-text.exe') -Force
    Write-Output "Installed to $InstallDir. Ensure it is on your PATH."
} finally {
    Remove-Item -Recurse -Force $tempDir
}
