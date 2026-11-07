param(
    [Parameter(Mandatory=$true)][string]$Version,
    [string]$InstallDir = (Join-Path $HOME '.local\bin'),
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$InstallDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($InstallDir)
if ($Version -notmatch '^v[0-9A-Za-z][0-9A-Za-z.-]*$') { throw 'Invalid version' }
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw 'Only Windows x86-64 is supported'
}
$names = @('git-compose-commit.exe', 'cli-compose-text.exe')
function Test-Destination {
    foreach ($name in $names) {
        $target = Join-Path $InstallDir $name
        $item = Get-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
        if ($null -ne $item) {
            if ($item.PSIsContainer -or ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
                throw "Refusing to replace a symlink or non-file: $target"
            }
            if (-not $Force) { throw "Already exists: $target (use -Force to replace)" }
        }
    }
}
Test-Destination
foreach ($name in $names) {
    $found = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found -and $found.Source -ne (Join-Path $InstallDir $name)) {
        Write-Warning "$name is already on PATH at $($found.Source); check PATH order."
    }
}
$archive = "git-compose-commit-$Version-x86_64-pc-windows-msvc.zip"
$base = if ($env:GIT_COMPOSE_COMMIT_RELEASE_BASE) { $env:GIT_COMPOSE_COMMIT_RELEASE_BASE } else { 'https://github.com/demerphq/git-compose-commit/releases/download' }
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory $tempDir | Out-Null
$stage = $null
$lock = $null
$lockPath = $null
$keepStage = $false
$installed = @()
try {
    $file = Join-Path $tempDir $archive
    Invoke-WebRequest "$base/$Version/$archive" -OutFile $file -TimeoutSec 120
    $check = Join-Path $tempDir 'checksum'
    Invoke-WebRequest "$base/$Version/$archive.sha256" -OutFile $check -TimeoutSec 120
    $expected = ((Get-Content $check -Raw) -split '\s+')[0]
    if ($expected -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid checksum' }
    if ((Get-FileHash -Algorithm SHA256 $file).Hash -ne $expected) { throw 'Checksum mismatch' }
    $unpacked = Join-Path $tempDir 'unpacked'
    Expand-Archive -LiteralPath $file -DestinationPath $unpacked
    foreach ($name in $names) {
        if (-not (Test-Path -LiteralPath (Join-Path $unpacked $name) -PathType Leaf)) {
            throw "Archive is missing $name"
        }
    }

    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    $lockPath = Join-Path $InstallDir '.git-compose-commit-install.lock'
    $lock = [System.IO.File]::Open($lockPath, [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    $stage = Join-Path $InstallDir ('.git-compose-commit-install.' + [System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory $stage | Out-Null
    Test-Destination
    foreach ($name in $names) {
        Copy-Item -LiteralPath (Join-Path $unpacked $name) -Destination (Join-Path $stage "new-$name")
    }
    foreach ($name in $names) {
        $target = Join-Path $InstallDir $name
        $new = Join-Path $stage "new-$name"
        $backup = Join-Path $stage "old-$name"
        if (Test-Path -LiteralPath $target) {
            [System.IO.File]::Replace($new, $target, $backup)
        } else {
            [System.IO.File]::Move($new, $target)
        }
        $installed += $name
    }
    Write-Output "Installed both executables to $InstallDir. Ensure it is on your PATH."
} catch {
    foreach ($name in $installed) {
        $target = Join-Path $InstallDir $name
        $backup = Join-Path $stage "old-$name"
        try {
            if (Test-Path -LiteralPath $backup) {
                [System.IO.File]::Replace($backup, $target, (Join-Path $stage "failed-$name"))
            } else {
                Remove-Item -LiteralPath $target -Force
            }
        } catch {
            $keepStage = $true
            Write-Warning "Could not restore $name; backups remain in $stage. $($_.Exception.Message)"
        }
    }
    throw
} finally {
    if ($stage -and -not $keepStage -and (Test-Path -LiteralPath $stage)) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    if ($lock) {
        $lock.Dispose()
        Remove-Item -LiteralPath $lockPath -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $tempDir -Recurse -Force
}
