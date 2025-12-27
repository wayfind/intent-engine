#!/usr/bin/env node
// Intent-Engine Session Start Hook
// Cross-platform Node.js implementation

const { execSync, spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// === Parse stdin (session_id) ===

let sessionId = '';
try {
  // Read stdin synchronously (non-blocking if empty)
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
  // Validate session_id (alphanumeric, dash, underscore only)
  if (/^[a-zA-Z0-9_-]+$/.test(sessionId)) {
    try {
      fs.appendFileSync(
        process.env.CLAUDE_ENV_FILE,
        `export IE_SESSION_ID="${sessionId}"\n`
      );
    } catch {
      // Ignore write errors
    }
  }
}

// Set for current process
process.env.IE_SESSION_ID = sessionId;

// === Check if ie is installed ===

function commandExists(cmd) {
  try {
    const isWin = process.platform === 'win32';
    const checkCmd = isWin ? `where ${cmd}` : `command -v ${cmd}`;
    execSync(checkCmd, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

if (!commandExists('ie')) {
  console.log(`<system-reminder>
intent-engine (ie) not installed. Install via one of:
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
    spawnSync('ie', ['init'], {
      cwd: projectDir,
      stdio: 'ignore'
    });
  } catch {
    // Ignore init errors
  }
}

// === Run ie status ===

try {
  const result = spawnSync('ie', ['status'], {
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
} catch {
  // Ignore status errors
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
