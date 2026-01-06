# Intent-Engine OpenCode Plugin

Cross-session task memory for OpenCode.

## Installation

### Quick Install (curl)

```bash
# Download plugin to OpenCode plugin directory
curl -fsSL https://raw.githubusercontent.com/wayfind/intent-engine/main/opencode-plugin/intent-engine.ts \
  -o ~/.config/opencode/plugin/intent-engine.ts
```

### From Source (symlink)

```bash
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Create symlink
mkdir -p ~/.config/opencode/plugin
ln -sf $(pwd)/opencode-plugin/intent-engine.ts ~/.config/opencode/plugin/
```

## Prerequisites

Intent-Engine CLI must be installed:

```bash
npm install -g @origintask/intent-engine
# or: cargo install intent-engine
# or: brew install wayfind/tap/intent-engine
```

## Features

- **Session Persistence**: `ie_status` restores context across sessions
- **Task Hierarchy**: `ie_plan` creates hierarchical task trees with `depends_on`
- **Decision Logging**: `ie_log` records decisions, blockers, milestones, notes
- **Full-Text Search**: `ie_search` finds past decisions and tasks
- **Visual Dashboard**: `ie dashboard` opens a web UI for task management

## Dashboard

Launch the visual dashboard for a complete overview of your tasks:

```bash
ie dashboard
```

![IE Dashboard](https://raw.githubusercontent.com/wayfind/intent-engine/main/docs/iedashboard.png)

The dashboard provides:
- **Task Navigator** — Hierarchical tree view with search
- **Task Detail** — Full spec with markdown rendering (diagrams, code blocks)
- **Decision Timeline** — Chronological log of all decisions and notes
- **Multi-project Support** — Switch between projects via tabs

## Workflow-Specific Patterns

### Bug Fix (reproduce→diagnose→fix→verify)
- FLAT task structure
- Heavy `note` events for investigation
- `blocker` when stuck
- `milestone` when root cause found

### Refactoring/Migration (analyze→design→migrate→verify)
- DEEP task hierarchy (phase→component→step)
- Sequential `depends_on` chain
- `milestone` after each component

### Feature Development (design→implement→integrate→test)
- PARALLEL branches (Backend || Frontend)
- Integration depends on BOTH branches
- Rich specs with diagrams

## License

MIT
