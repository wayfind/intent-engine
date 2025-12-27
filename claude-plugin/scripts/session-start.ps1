# Intent-Engine Session Start Hook
# Compatible with: Windows PowerShell 5.1+, PowerShell Core 7+
#Requires -Version 5.1

$ErrorActionPreference = "SilentlyContinue"

# === Helper Functions ===

function Write-DebugLog {
    param([string]$Message)
    # Uncomment for debugging: Write-Host "[ie-hook] $Message" -ForegroundColor Gray
}

function Get-SessionIdFromJson {
    param([string]$JsonInput)

    if ([string]::IsNullOrWhiteSpace($JsonInput)) {
        return ""
    }

    try {
        $obj = $JsonInput | ConvertFrom-Json
        if ($obj.session_id) {
            return $obj.session_id
        }
    }
    catch {
        Write-DebugLog "JSON parse failed: $_"
    }

    return ""
}

function Test-Command {
    param([string]$Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

# === Main Logic ===

# Read stdin if available
$input = ""
if ([Console]::IsInputRedirected) {
    try {
        $input = [Console]::In.ReadToEnd()
        Write-DebugLog "Read stdin: $($input.Length) chars"
    }
    catch {
        Write-DebugLog "Failed to read stdin: $_"
    }
}

# Parse session_id
$sessionId = Get-SessionIdFromJson $input
Write-DebugLog "Parsed session_id: $sessionId"

# Set session environment variable
if ($env:CLAUDE_ENV_FILE -and $sessionId) {
    # Validate session_id (alphanumeric, dash, underscore only)
    if ($sessionId -match '^[a-zA-Z0-9_-]+$') {
        try {
            # For PowerShell env file, use PowerShell syntax
            Add-Content -Path $env:CLAUDE_ENV_FILE -Value "`$env:IE_SESSION_ID = `"$sessionId`""
            Write-DebugLog "Wrote session_id to CLAUDE_ENV_FILE"
        }
        catch {
            Write-DebugLog "Failed to write env file: $_"
        }
    }
}

# === Auto-install ie if not found ===

# Cache file to avoid repeated install attempts
$InstallAttemptedFile = Join-Path $env:USERPROFILE ".intent-engine\.install-attempted"

function Install-IntentEngine {
    Write-Host "🔧 Installing intent-engine..." -ForegroundColor Cyan

    # Try npm first (fastest, no compiler needed)
    if (Test-Command "npm") {
        Write-Host "   → Using npm..." -ForegroundColor Gray
        $result = npm install -g "@m3task/intent-engine" 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "   ✓ Installed via npm" -ForegroundColor Green
            # Refresh PATH to find newly installed command
            $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                        [System.Environment]::GetEnvironmentVariable("Path", "User")
            return $true
        }
    }

    # Try cargo (for Rust developers)
    if (Test-Command "cargo") {
        Write-Host "   → Using cargo (this may take a few minutes)..." -ForegroundColor Gray
        $result = cargo install intent-engine 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "   ✓ Installed via cargo" -ForegroundColor Green
            return $true
        }
    }

    Write-Host "   ✗ Installation failed" -ForegroundColor Red
    return $false
}

if (-not (Test-Command "ie")) {
    $shouldInstall = $true

    # Check if we already tried to install (and failed) recently
    if (Test-Path $InstallAttemptedFile) {
        $fileAge = (Get-Date) - (Get-Item $InstallAttemptedFile).LastWriteTime
        if ($fileAge.TotalHours -lt 1) {
            $shouldInstall = $false
            Write-DebugLog "Skipping install - attempted recently"
        } else {
            Remove-Item $InstallAttemptedFile -Force -ErrorAction SilentlyContinue
        }
    }

    if ($shouldInstall) {
        $parentDir = Split-Path $InstallAttemptedFile -Parent
        if (-not (Test-Path $parentDir)) {
            New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
        }

        if (Install-IntentEngine) {
            Remove-Item $InstallAttemptedFile -Force -ErrorAction SilentlyContinue
        } else {
            New-Item -ItemType File -Path $InstallAttemptedFile -Force | Out-Null
        }
    }
}

# Check if ie is available now
if (-not (Test-Command "ie")) {
    Write-Output @"
<system-reminder>
intent-engine (ie) not installed. Install via one of:
  npm install -g @m3task/intent-engine
  cargo install intent-engine
</system-reminder>
"@
    exit 0
}

# === Initialize project if needed ===

$projectDir = if ($env:CLAUDE_PROJECT_DIR) { $env:CLAUDE_PROJECT_DIR } else { Get-Location }

if ((Test-Path $projectDir) -and -not (Test-Path "$projectDir\.intent-engine")) {
    Write-DebugLog "Initializing ie in $projectDir"
    Push-Location $projectDir
    try {
        ie init 2>&1 | Out-Null
    }
    catch { }
    Pop-Location
}

# === Run ie status ===

if (Test-Path $projectDir) {
    Set-Location $projectDir
}

# Set session_id environment variable
$env:IE_SESSION_ID = $sessionId

# Run status
try {
    $statusOutput = ie status 2>&1
    if ($statusOutput) {
        Write-Output $statusOutput
    }
}
catch {
    Write-DebugLog "ie status failed: $_"
}

# === Output system reminder ===

Write-Output @"

<system-reminder>
# Use ``ie plan`` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  - Session start: ie status
  - Create/update tasks: echo '{...}' | ie plan
  - Record decisions: ie log decision "..."
  - Details: ie --help
</system-reminder>
"@

exit 0
