# Intent-Engine Session Start Hook (Windows)
# Behavior: Check ie availability, show warning if missing, run status if available
# NO auto-install - consistent with Linux/macOS behavior

$ErrorActionPreference = "SilentlyContinue"

# === Parse stdin (session_id) ===

$sessionId = ""
if ([Console]::IsInputRedirected) {
    try {
        $input = [Console]::In.ReadToEnd()
        if ($input) {
            $obj = $input | ConvertFrom-Json
            if ($obj.session_id) {
                $sessionId = $obj.session_id
            }
        }
    } catch { }
}

# === Set environment variable ===

if ($env:CLAUDE_ENV_FILE -and $sessionId -match '^[a-zA-Z0-9_-]+$') {
    try {
        Add-Content -Path $env:CLAUDE_ENV_FILE -Value "`$env:IE_SESSION_ID = `"$sessionId`""
    } catch { }
}

$env:IE_SESSION_ID = $sessionId

# === Check if ie is installed ===

$ieCmd = Get-Command "ie" -ErrorAction SilentlyContinue
if (-not $ieCmd) {
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
    Push-Location $projectDir
    try { ie init 2>&1 | Out-Null } catch { }
    Pop-Location
}

# === Run ie status ===

if (Test-Path $projectDir) {
    Set-Location $projectDir
}

try {
    $statusOutput = ie status 2>&1
    if ($statusOutput) {
        Write-Output $statusOutput
    }
} catch { }

# === Output system reminder ===

Write-Output @"

<system-reminder>
# Use `ie plan` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  - Session start: ie status
  - Create/update tasks: echo '{...}' | ie plan
  - Record decisions: ie log decision "..."
  - Details: ie --help
</system-reminder>
"@

exit 0
