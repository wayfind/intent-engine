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
      fs.appendFileSync(
        process.env.CLAUDE_ENV_FILE,
        `export IE_SESSION_ID="${sessionId}"\n`
      );
    } catch {}
  }
}

process.env.IE_SESSION_ID = sessionId;

// === Find or install ie ===

function findIeBinary() {
  // Method 1: Check if ie is in PATH
  try {
    const isWin = process.platform === 'win32';
    const checkCmd = isWin ? 'where ie' : 'command -v ie';
    const result = execSync(checkCmd, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] });
    const iePath = result.trim().split('\n')[0];
    if (iePath && fs.existsSync(iePath)) {
      return iePath;
    }
  } catch {}

  // Method 2: Check npm global bin directory
  try {
    const npmBinDir = execSync('npm bin -g', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] }).trim();
    const isWin = process.platform === 'win32';
    const iePath = path.join(npmBinDir, isWin ? 'ie.cmd' : 'ie');
    if (fs.existsSync(iePath)) {
      return iePath;
    }
  } catch {}

  return null;
}

function installIe() {
  console.log('Installing intent-engine via npm...');
  try {
    const result = spawnSync('npm', ['install', '-g', '@m3task/intent-engine'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 60000 // 60 second timeout
    });

    if (result.status === 0) {
      console.log('intent-engine installed successfully.');
      return true;
    } else {
      if (result.stderr) {
        console.error('Installation failed:', result.stderr.slice(0, 200));
      }
      return false;
    }
  } catch (e) {
    console.error('Installation error:', e.message);
    return false;
  }
}

// Find ie binary, install if needed
let iePath = findIeBinary();

if (!iePath) {
  // Try to install
  if (installIe()) {
    iePath = findIeBinary();
  }
}

if (!iePath) {
  console.log(`<system-reminder>
intent-engine (ie) not installed and auto-install failed.
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
      stdio: 'ignore'
    });
  } catch {}
}

// === Run ie status ===

try {
  const result = spawnSync(iePath, ['status'], {
    cwd: projectDir,
    encoding: 'utf8',
    env: { ...process.env, IE_SESSION_ID: sessionId }
  });

  if (result.stdout) {
    console.log(result.stdout);
  }
  if (result.stderr && !result.stdout) {
    console.error(result.stderr);
  }
} catch {}

// === Output system reminder ===

console.log(`
<system-reminder>
# Use \`ie plan\` instead of TodoWrite (cross-session, hierarchical, decision-tracking)

  - Session start: ie status
  - Create/update tasks: echo '{...}' | ie plan
  - Record decisions: ie log decision "..."
  - Details: ie --help
</system-reminder>`);
