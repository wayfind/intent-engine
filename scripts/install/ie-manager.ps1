#Requires -Version 5.1
<#
.SYNOPSIS
    ie CLI Manager - Install, Upgrade, Uninstall

.DESCRIPTION
    Manages the ie CLI installation from GitHub releases.

.PARAMETER Command
    The action to perform: install, upgrade, or uninstall

.PARAMETER Version
    Optional version to install (default: latest)

.EXAMPLE
    .\ie-manager.ps1 install
    .\ie-manager.ps1 install v0.10.10
    .\ie-manager.ps1 upgrade
    .\ie-manager.ps1 uninstall
#>

param(
    [Parameter(Position = 0)]
    [ValidateSet("install", "upgrade", "uninstall", "help")]
    [string]$Command = "help",

    [Parameter(Position = 1)]
    [string]$Version = "",

    [Alias("y")]
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$REPO = "wayfind/intent-engine"
$BINARY_NAME = "ie.exe"

# Ensure USERPROFILE is set (may be unset in some CI environments)
if (-not $env:USERPROFILE) {
    Write-Host "[ERROR] USERPROFILE environment variable is not set" -ForegroundColor Red
    exit 1
}

$INSTALL_DIR = Join-Path $env:USERPROFILE ".local\bin"
$DATA_DIR = Join-Path $env:USERPROFILE ".intent-engine"

function Write-Info($Message) {
    Write-Host "[INFO] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn($Message) {
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Err($Message) {
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-LatestVersion {
    try {
        $headers = @{}
        if ($env:GITHUB_TOKEN) {
            $headers["Authorization"] = "token $env:GITHUB_TOKEN"
        }
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -Headers $headers
        return $release.tag_name
    }
    catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        if ($statusCode -eq 403) {
            Write-Err "GitHub API rate limit exceeded. Set GITHUB_TOKEN env var or wait."
        }
        elseif ($statusCode -eq 404) {
            Write-Err "Release not found. Check repository: $REPO"
        }
        else {
            Write-Err "Failed to fetch latest version: $_"
        }
    }
}

function Get-CurrentVersion {
    # First check INSTALL_DIR
    $binaryPath = Join-Path $INSTALL_DIR $BINARY_NAME
    if (Test-Path $binaryPath) {
        try {
            $output = & $binaryPath --version 2>$null
            if ($output -match '(\d+\.\d+\.\d+)') {
                return "v$($Matches[1])"
            }
        }
        catch {}
    }

    # Also check PATH (consistent with bash version using command -v)
    $pathBinary = Get-Command "ie" -ErrorAction SilentlyContinue
    if ($pathBinary) {
        try {
            $output = & $pathBinary.Source --version 2>$null
            if ($output -match '(\d+\.\d+\.\d+)') {
                return "v$($Matches[1])"
            }
        }
        catch {}
    }

    return $null
}

# Compare semantic versions: returns 0 if equal, 1 if v1 > v2, -1 if v1 < v2
# Only compares numeric parts (ignores prerelease suffixes like -beta)
function Compare-SemVer {
    param([string]$v1, [string]$v2)

    # Strip 'v' prefix and prerelease suffix
    $v1 = ($v1 -replace '^v', '') -replace '-.*$', ''
    $v2 = ($v2 -replace '^v', '') -replace '-.*$', ''

    $parts1 = $v1 -split '\.'
    $parts2 = $v2 -split '\.'

    for ($i = 0; $i -lt 3; $i++) {
        # Parse as int, default to 0 if not numeric
        $n1 = 0
        $n2 = 0
        if ($i -lt $parts1.Count) {
            [int]::TryParse($parts1[$i], [ref]$n1) | Out-Null
        }
        if ($i -lt $parts2.Count) {
            [int]::TryParse($parts2[$i], [ref]$n2) | Out-Null
        }

        if ($n1 -gt $n2) { return 1 }
        if ($n1 -lt $n2) { return -1 }
    }
    return 0
}

function Install-Binary {
    param([string]$TargetVersion)

    if (-not $TargetVersion) {
        $TargetVersion = Get-LatestVersion
    }

    # Detect architecture using PROCESSOR_ARCHITECTURE
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { "x86_64" }
        "ARM64" { Write-Err "ARM64 Windows is not yet supported. Please use WSL or x86 emulation." }
        "x86"   { Write-Err "32-bit Windows is not supported" }
        default { Write-Err "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
    }

    $assetName = "intent-engine-windows-$arch.exe.zip"
    $downloadUrl = "https://github.com/$REPO/releases/download/$TargetVersion/$assetName"

    Write-Info "Downloading ie $TargetVersion for windows-$arch..."

    $tmpDir = Join-Path $env:TEMP "ie-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $zipPath = Join-Path $tmpDir $assetName

        # Download with retry - Enable TLS 1.2 and 1.3 (if available)
        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
        }
        catch {
            # TLS 1.3 not available on older systems, fall back to TLS 1.2
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        }

        $maxRetries = 3
        $retryDelay = 2
        $attempt = 1

        while ($attempt -le $maxRetries) {
            try {
                Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing
                break
            }
            catch {
                if ($attempt -eq $maxRetries) {
                    Write-Err "Failed to download from $downloadUrl after $maxRetries attempts: $_"
                }
                Write-Warn "Download failed, retrying ($($attempt + 1)/$maxRetries)..."
                Start-Sleep -Seconds $retryDelay
                $attempt++
            }
        }

        # Optional: Verify checksum if SHA256SUMS is available
        $checksumUrl = "https://github.com/$REPO/releases/download/$TargetVersion/SHA256SUMS"
        try {
            $checksumContent = (Invoke-WebRequest -Uri $checksumUrl -UseBasicParsing).Content
            Write-Info "Verifying checksum..."
            # Use .Contains() for exact substring match (no wildcard interpretation)
            # Match line ending with exact filename (format: "hash  filename" or "hash *filename")
            $matchLine = $checksumContent -split "`n" | Where-Object {
                $line = $_.Trim()
                $line.EndsWith($assetName) -or $line.EndsWith("*$assetName")
            } | Select-Object -First 1
            if ($matchLine) {
                $expectedHash = ($matchLine -split '\s+')[0]
                if ($expectedHash) {
                    $actualHash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
                    if ($actualHash -ne $expectedHash.ToLower()) {
                        Write-Err "Checksum mismatch! Expected: $expectedHash, Got: $actualHash"
                    }
                    Write-Info "Checksum verified"
                }
            }
            else {
                Write-Warn "Asset not found in SHA256SUMS - skipping verification"
            }
        }
        catch {
            # Checksum file not available, continue without verification
        }

        Write-Info "Extracting..."

        # Extract
        Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

        # Create install directory
        if (-not (Test-Path $INSTALL_DIR)) {
            New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
        }

        # Find and move binary
        $extractedBinary = Get-ChildItem -Path $tmpDir -Filter $BINARY_NAME -Recurse | Select-Object -First 1
        if (-not $extractedBinary) {
            Write-Err "Binary '$BINARY_NAME' not found in archive"
        }

        $destPath = Join-Path $INSTALL_DIR $BINARY_NAME
        Move-Item -Path $extractedBinary.FullName -Destination $destPath -Force

        Write-Info "Installed to $destPath"

        # Check PATH (exact match, not substring)
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $pathDirs = $userPath -split ';' | ForEach-Object { $_.TrimEnd('\') }
        $installDirNorm = $INSTALL_DIR.TrimEnd('\')
        if ($pathDirs -notcontains $installDirNorm) {
            Write-Warn "$INSTALL_DIR is not in your PATH"
            Write-Host ""
            Write-Host "Add it to your PATH by running:"
            Write-Host "  `$env:Path += `";$INSTALL_DIR`""
            Write-Host ""
            Write-Host "To make it permanent:"
            Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$INSTALL_DIR', 'User')"
            Write-Host ""
        }

        # Verify
        try {
            $ver = & $destPath --version 2>$null
            Write-Info "Successfully installed: $ver"
        }
        catch {}
    }
    finally {
        # Cleanup
        if (Test-Path $tmpDir) {
            Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-Install {
    param([string]$TargetVersion)

    $current = Get-CurrentVersion
    if ($current) {
        Write-Warn "ie is already installed ($current)"
        Write-Host "Use 'upgrade' to update or 'uninstall' first"
        exit 1
    }

    Install-Binary -TargetVersion $TargetVersion
    Write-Info "Installation complete!"
}

function Invoke-Upgrade {
    param([string]$TargetVersion)

    if (-not $TargetVersion) {
        $TargetVersion = Get-LatestVersion
    }

    $current = Get-CurrentVersion

    if (-not $current) {
        Write-Warn "ie is not installed. Running install instead..."
        Install-Binary -TargetVersion $TargetVersion
    }
    else {
        $cmpResult = Compare-SemVer -v1 $current -v2 $TargetVersion

        if ($cmpResult -eq 0) {
            Write-Info "Already at version $TargetVersion"
            exit 0
        }
        elseif ($cmpResult -eq 1) {
            Write-Warn "Current version ($current) is newer than target ($TargetVersion)"
            Write-Warn "Use explicit version to downgrade if intended"
            exit 0
        }

        Write-Info "Upgrading from $current to $TargetVersion..."
        Install-Binary -TargetVersion $TargetVersion
    }

    Write-Info "Upgrade complete!"
}

function Invoke-Uninstall {
    $binaryPath = Join-Path $INSTALL_DIR $BINARY_NAME
    $removed = $false

    # Remove binary
    if (Test-Path $binaryPath) {
        Remove-Item -Path $binaryPath -Force
        Write-Info "Removed $binaryPath"
        $removed = $true
    }
    else {
        Write-Warn "Binary not found at $binaryPath"

        # Check if installed elsewhere
        $altPath = (Get-Command "ie" -ErrorAction SilentlyContinue).Source
        if ($altPath) {
            Write-Warn "ie found at $altPath"
            Write-Host "If installed via other methods, use:"
            Write-Host "  npm uninstall -g @origintask/intent-engine"
            Write-Host "  cargo uninstall intent-engine"
        }
    }

    # Ask about data directory
    if (Test-Path $DATA_DIR) {
        Write-Host ""
        if ($Force) {
            # Force mode - remove without asking
            Remove-Item -Path $DATA_DIR -Recurse -Force
            Write-Info "Removed $DATA_DIR"
        }
        elseif ([Environment]::UserInteractive -and [Console]::IsInputRedirected -eq $false) {
            # Interactive mode - ask user
            $response = Read-Host "Remove data directory $DATA_DIR? [y/N]"
            if ($response -match '^[Yy]$') {
                Remove-Item -Path $DATA_DIR -Recurse -Force
                Write-Info "Removed $DATA_DIR"
            }
            else {
                Write-Info "Kept $DATA_DIR"
            }
        }
        else {
            # Non-interactive mode - keep data by default
            Write-Warn "Non-interactive mode. Use -Force to remove data, or manually: Remove-Item -Recurse '$DATA_DIR'"
            Write-Info "Kept $DATA_DIR"
        }
    }

    if ($removed) {
        Write-Info "Uninstall complete!"
    }
}

function Show-Help {
    Write-Host @"
ie CLI Manager - Install, Upgrade, Uninstall

Usage: .\ie-manager.ps1 <command> [version] [options]

Commands:
  install [version]   Install ie CLI (default: latest)
  upgrade [version]   Upgrade to specified version (default: latest)
  uninstall           Remove ie CLI and optionally data

Options:
  -Force, -y          Skip confirmation prompts (for automation)

Examples:
  .\ie-manager.ps1 install
  .\ie-manager.ps1 install v0.10.10
  .\ie-manager.ps1 upgrade
  .\ie-manager.ps1 uninstall
  .\ie-manager.ps1 uninstall -Force

Install directory: $INSTALL_DIR
Data directory:    $DATA_DIR
"@
}

# Main
switch ($Command) {
    "install"   { Invoke-Install -TargetVersion $Version }
    "upgrade"   { Invoke-Upgrade -TargetVersion $Version }
    "uninstall" { Invoke-Uninstall }
    "help"      { Show-Help }
    default     { Show-Help; exit 1 }
}
