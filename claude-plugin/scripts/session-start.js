#!/usr/bin/env node
// Intent-Engine Session Start Hook
// Cross-platform Node.js implementation with auto-install

const { execSync, spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// === Parse stdin (session_id) ===

let sessionId = '';
try {
  const input = fs.readFileSync(0, 'utf8').trim();
  if (input) {
    const data = JSON.parse(input);
    sessionId = data.session_id || '';
  }
} catch {
  // Ignore parse errors
}

// === Set environment variable ===

if (process.env.CLAUDE_ENV_FILE && sessionId) {
  if (/^[a-zA-Z0-9_-]+$/.test(sessionId)) {
    try {
      // Use export syntax (works in bash/sh environments that Claude Code uses)
      fs.appendFileSync(
        process.env.CLAUDE_ENV_FILE,
        `export IE_SESSION_ID="${sessionId}"\n`
      );
    } catch {}
  }
}

process.env.IE_SESSION_ID = sessionId;

// === Utility functions ===

const isWin = process.platform === 'win32';

function commandExists(cmd) {
  try {
    execSync(isWin ? `where ${cmd}` : `command -v ${cmd}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

function verifyIeBinary(iePath) {
  // Actually run ie --version to verify it works
  try {
    const result = spawnSync(iePath, ['--version'], {
      encoding: 'utf8',
      timeout: 5000,
      stdio: ['ignore', 'pipe', 'pipe']
    });
    return result.status === 0;
  } catch {
    return false;
  }
}

function getNpmGlobalBinDir() {
  try {
    // Use npm config get prefix (more reliable than deprecated npm bin -g)
    const prefix = execSync('npm config get prefix', {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'ignore']
    }).trim();

    if (isWin) {
      // Windows: prefix itself contains the binaries
      return prefix;
    } else {
      // Unix: prefix/bin
      return path.join(prefix, 'bin');
    }
  } catch {
    return null;
  }
}

function findIeBinary() {
  // Method 1: Check if ie is in PATH and works
  try {
    const checkCmd = isWin ? 'where ie' : 'command -v ie';
    const result = execSync(checkCmd, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] });
    const iePath = result.trim().split('\n')[0];

    if (iePath && fs.existsSync(iePath) && verifyIeBinary(iePath)) {
      return iePath;
    }
  } catch {}

  // Method 2: Check npm global bin directory
  const npmBinDir = getNpmGlobalBinDir();
  if (npmBinDir) {
    const iePath = path.join(npmBinDir, isWin ? 'ie.cmd' : 'ie');
    if (fs.existsSync(iePath) && verifyIeBinary(iePath)) {
      return iePath;
    }
  }

  return null;
}

function installIe() {
  // Check if npm is available
  if (!commandExists('npm')) {
    console.log('npm not found. Cannot auto-install intent-engine.');
    return false;
  }

  console.log('Installing intent-engine via npm...');
  try {
    const result = spawnSync('npm', ['install', '-g', '@m3task/intent-engine'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 60000,
      shell: isWin // Use shell on Windows for better compatibility
    });

    if (result.status === 0) {
      console.log('intent-engine installed successfully.');
      return true;
    } else {
      const errorMsg = (result.stderr || result.stdout || 'Unknown error').slice(0, 300);
      console.log('Installation failed:', errorMsg);
      return false;
    }
  } catch (e) {
    console.log('Installation error:', e.message);
    return false;
  }
}

// === Main logic ===

// Find ie binary, install if needed
let iePath = findIeBinary();

if (!iePath) {
  // Try to install
  const installed = installIe();
  if (installed) {
    iePath = findIeBinary();
    if (!iePath) {
      console.log('Installation succeeded but ie binary not found or not working.');
    }
  }
}

if (!iePath) {
  console.log(`<system-reminder>
intent-engine (ie) not available.

Auto-install ${commandExists('npm') ? 'failed' : 'skipped (npm not found)'}.

Please install manually:
  npm install -g @m3task/intent-engine
  cargo install intent-engine
  brew install wayfind/tap/intent-engine
</system-reminder>`);
  process.exit(0);
}

// === Initialize project if needed ===

const projectDir = process.env.CLAUDE_PROJECT_DIR || process.cwd();
const ieDir = path.join(projectDir, '.intent-engine');

if (fs.existsSync(projectDir) && !fs.existsSync(ieDir)) {
  try {
    spawnSync(iePath, ['init'], {
      cwd: projectDir,
      stdio: 'ignore',
      timeout: 10000
    });
  } catch {}
}

// === Run ie status ===

try {
  const result = spawnSync(iePath, ['status'], {
    cwd: projectDir,
    encoding: 'utf8',
    timeout: 15000,
    env: { ...process.env, IE_SESSION_ID: sessionId }
  });

  if (result.stdout) {
    console.log(result.stdout);
  }
  // Always show stderr if present (might contain warnings)
  if (result.stderr) {
    console.error(result.stderr);
  }
} catch (e) {
  console.log('Failed to run ie status:', e.message);
}

// === Output system reminder ===

console.log(`
<system-reminder>
# Use \`ie plan\` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  - Session start: ie status
  - Create/update tasks: echo '{...}' | ie plan
  - Record decisions: ie log decision "..."
  - Details: ie --help
</system-reminder>`);
