# Romit installer for Windows PowerShell
# SPDX-License-Identifier: MIT
[CmdletBinding()]
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$RomitRepo = if ($env:ROMIT_REPO) { $env:ROMIT_REPO } else { 'romit07/romitlang' }
$RomitInstallDir = if ($env:ROMIT_INSTALL_DIR) { $env:ROMIT_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.romit\bin' }

function Write-InstallMsg {
    param([string]$Message)
    Write-Host "romit install: $Message"
}

function Get-RomitTarget {
    if (-not $env:PROCESSOR_ARCHITECTURE) {
        throw 'Could not detect processor architecture'
    }
    switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { return 'x86_64-pc-windows-msvc' }
        'ARM64' { throw 'ARM64 Windows builds are not published yet; build from source with cargo.' }
        default { throw "Unsupported Windows architecture: $($env:PROCESSOR_ARCHITECTURE)" }
    }
}

function Normalize-Version {
    param([string]$Version)
    if ($Version -match '^v') { return $Version.Substring(1) }
    return $Version
}

function Get-LatestVersion {
    $uri = "https://api.github.com/repos/$RomitRepo/releases/latest"
    $release = Invoke-RestMethod -Uri $uri -Headers @{ 'User-Agent' = 'romit-installer' }
    return (Normalize-Version $release.tag_name)
}

function Get-DownloadUrls {
    param(
        [string]$Version,
        [string]$ArchiveName
    )
    if ($env:ROMIT_BASE_URL) {
        $base = $env:ROMIT_BASE_URL.TrimEnd('/')
        return @{
            Archive = "$base/$ArchiveName"
            Sums    = "$base/SHA256SUMS"
        }
    }
    $tag = "v$Version"
    $root = "https://github.com/$RomitRepo/releases/download/$tag"
    return @{
        Archive = "$root/$ArchiveName"
        Sums    = "$root/SHA256SUMS"
    }
}

function Verify-FileChecksum {
    param(
        [string]$FilePath,
        [string]$SumsPath
    )
    $base = Split-Path -Leaf $FilePath
    $line = Get-Content $SumsPath | Where-Object { $_ -match "\s+$([regex]::Escape($base))$" } | Select-Object -First 1
    if (-not $line) {
        throw "No SHA256 entry for $base"
    }
    $expected = ($line -split '\s+')[0].ToLower()
    $actual = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) {
        throw "Checksum mismatch for $base`nexpected: $expected`nactual:   $actual"
    }
}

function Add-RomitToUserPath {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -split ';' | Where-Object { $_ -eq $RomitInstallDir }) {
        Write-InstallMsg "PATH already contains $RomitInstallDir"
        return
    }
    if ($env:ROMIT_MODIFY_PATH -eq '1') {
        $newPath = if ([string]::IsNullOrEmpty($userPath)) { $RomitInstallDir } else { "$RomitInstallDir;$userPath" }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$RomitInstallDir;$env:Path"
        Write-InstallMsg "Updated user PATH"
    }
    else {
        Write-InstallMsg ''
        Write-InstallMsg 'Add Romit to your PATH (User environment variable):'
        Write-InstallMsg "  $RomitInstallDir"
        Write-InstallMsg 'Or re-run: $env:ROMIT_MODIFY_PATH=''1''; irm https://romitlang.org/install.ps1 | iex'
    }
}

function Uninstall-Romit {
    $exe = Join-Path $RomitInstallDir 'romit.exe'
    if (Test-Path $exe) {
        Remove-Item $exe -Force
        Write-InstallMsg "removed $exe"
    }
    else {
        Write-InstallMsg "no romit.exe at $exe"
    }
}

function Install-Romit {
    $target = Get-RomitTarget
    if ($env:ROMIT_VERSION) {
        $version = Normalize-Version $env:ROMIT_VERSION
    }
    else {
        $version = Get-LatestVersion
    }

    $archive = "romit-$version-$target.zip"
    $urls = Get-DownloadUrls -Version $version -ArchiveName $archive

    $tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("romit-install-" + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tmpdir -Force | Out-Null
    try {
        $archivePath = Join-Path $tmpdir $archive
        $sumsPath = Join-Path $tmpdir 'SHA256SUMS'

        Write-InstallMsg "installing romit $version for $target"
        Invoke-WebRequest -Uri $urls.Archive -OutFile $archivePath -UseBasicParsing
        Invoke-WebRequest -Uri $urls.Sums -OutFile $sumsPath -UseBasicParsing
        Verify-FileChecksum -FilePath $archivePath -SumsPath $sumsPath

        $extractDir = Join-Path $tmpdir 'extract'
        Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force
        $bin = Get-ChildItem -Path $extractDir -Recurse -Filter 'romit.exe' | Select-Object -First 1
        if (-not $bin) {
            throw 'Archive did not contain romit.exe'
        }

        New-Item -ItemType Directory -Path $RomitInstallDir -Force | Out-Null
        Copy-Item $bin.FullName (Join-Path $RomitInstallDir 'romit.exe') -Force
        Write-InstallMsg "installed to $(Join-Path $RomitInstallDir 'romit.exe')"
        & (Join-Path $RomitInstallDir 'romit.exe') --version
        Add-RomitToUserPath
    }
    finally {
        Remove-Item $tmpdir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($Uninstall) {
    Uninstall-Romit
}
else {
    Install-Romit
}
