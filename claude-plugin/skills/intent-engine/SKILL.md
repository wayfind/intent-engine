# Intent-Engine Task Memory

## Trigger
When user mentions: tasks, planning, todo, decisions, "ie", cross-session work, or wants to track progress.

## Prerequisites
Before using ie commands, check and install if needed:

```bash
if ! command -v ie &> /dev/null; then
  echo "Installing intent-engine..."
  if command -v cargo &> /dev/null; then
    cargo install intent-engine
  elif command -v npm &> /dev/null; then
    npm install -g intent-engine
  elif command -v brew &> /dev/null; then
    brew install wayfind/tap/intent-engine
  else
    echo "Please install via: cargo install intent-engine"
    exit 1
  fi
fi
```

## Core Rule
Use `ie plan` instead of TodoWrite for:
- Cross-session persistence
- Hierarchical task breakdown
- Decision tracking

## Commands
```bash
ie status                      # Current task context
echo '{"tasks":[...]}' | ie plan   # Create/update tasks
ie log decision "..."          # Record decisions
ie search "query"              # Search tasks and events
```

## Task Creation
```bash
echo '{"tasks":[{"name":"Task name","status":"doing"}]}' | ie plan
```

## Hierarchical Tasks
```bash
echo '{"tasks":[{
  "name":"Parent task",
  "status":"doing",
  "children":[
    {"name":"Subtask 1","status":"todo"},
    {"name":"Subtask 2","status":"todo"}
  ]
}]}' | ie plan
```

## Record Decisions
```bash
ie log decision "Chose X because Y"
ie log blocker "Waiting on Z"
ie log milestone "Feature complete"
```

## Details
Run `ie --help` for full documentation.
