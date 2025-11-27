# Intent Engine Installation Script for Windows (PowerShell)
# This script installs intent-engine either from crates.io or builds from source

$ErrorActionPreference = "Stop"

# Function to print colored output
function Print-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Print-Warning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Print-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Check if running on Windows
Print-Info "Checking system compatibility..."
if (-not $IsWindows -and -not ($env:OS -like "Windows*")) {
    if ($PSVersionTable.PSVersion.Major -lt 6) {
        # PowerShell 5.x and earlier are Windows-only
        Print-Info "Platform: Windows"
    } else {
        Print-Error "This script is designed for Windows. Please use install.sh on Unix/Linux/macOS."
        exit 1
    }
} else {
    Print-Info "Platform: Windows"
}

# Check if Rust and Cargo are installed
Print-Info "Checking for Rust and Cargo..."
try {
    $cargoVersion = cargo --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo not found"
    }
    $rustVersion = rustc --version 2>&1
    Print-Info "Found: $rustVersion"
    Print-Info "Found: $cargoVersion"
} catch {
    Print-Error "Cargo is not installed!"
    Print-Info "Please install Rust and Cargo from https://rustup.rs/"
    Print-Info "Download and run: https://win.rustup.rs/"
    exit 1
}

# Determine installation method
$isSourceRepo = $false
if (Test-Path "Cargo.toml") {
    $cargoContent = Get-Content "Cargo.toml" -Raw
    if ($cargoContent -match 'name\s*=\s*"intent-engine"') {
        $isSourceRepo = $true
    }
}

if ($isSourceRepo) {
    Print-Info "Detected intent-engine source repository"
    Print-Info "Installing from source..."

    # Build and install from source
    try {
        cargo install --path . --force
        if ($LASTEXITCODE -ne 0) {
            throw "Installation failed"
        }
        Print-Info "Successfully installed intent-engine from source!"
    } catch {
        Print-Error "Installation from source failed!"
        Print-Error $_.Exception.Message
        exit 1
    }
} else {
    Print-Info "Installing from crates.io..."

    # Install from crates.io
    try {
        cargo install intent-engine --force
        if ($LASTEXITCODE -ne 0) {
            throw "Installation failed"
        }
        Print-Info "Successfully installed intent-engine from crates.io!"
    } catch {
        Print-Error "Installation from crates.io failed!"
        Print-Info "This might mean the package hasn't been published yet."
        Print-Info "Please clone the repository and run this script from within it."
        exit 1
    }
}

# Verify installation
Print-Info "Verifying installation..."
try {
    $intentEngineVersion = intent-engine --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Print-Info "intent-engine is installed: $intentEngineVersion"

        # Run doctor command to check system health
        Print-Info "Running system health check..."
        intent-engine doctor

        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Print-Info "Installation complete! 🎉"
            Print-Info "You can now use 'intent-engine' command"
            Print-Info "Try: intent-engine --help"
        } else {
            Print-Warning "Installation succeeded but health check failed"
            Print-Info "You may need to troubleshoot your environment"
        }
    } else {
        throw "Verification failed"
    }
} catch {
    Print-Error "Installation verification failed!"
    Print-Info "The binary may not be in your PATH"
    Print-Info "Please add the Cargo bin directory to your PATH"
    Print-Info "Typically located at: $env:USERPROFILE\.cargo\bin"
    Print-Info ""
    Print-Info "To add to PATH:"
    Print-Info '  1. Open System Properties > Environment Variables'
    Print-Info '  2. Edit the "Path" variable'
    Print-Info "  3. Add: $env:USERPROFILE\.cargo\bin"
    Print-Info ""
    Print-Info "Or run this in PowerShell (as Administrator):"
    Print-Info '  [Environment]::SetEnvironmentVariable("Path", $env:Path + ";$env:USERPROFILE\.cargo\bin", "User")'
    exit 1
}
