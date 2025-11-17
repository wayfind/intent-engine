# Intent-Engine Dashboard User Guide

**Version**: 0.5.0 (Phase 1 MVP)

---

## Overview

The Intent-Engine Dashboard is a web-based interface for visualizing and managing your tasks and events. It complements the CLI and MCP tools, providing a rich visual experience with Markdown rendering, real-time search, and intuitive task management.

### Key Features

- ✅ **Task Management**: Create, update, delete, and organize tasks
- ✅ **Markdown Support**: Rich text rendering with code highlighting
- ✅ **Hierarchical Tasks**: Parent-child relationships with subtasks
- ✅ **Event Tracking**: Record decisions, blockers, milestones, and notes
- ✅ **Smart Search**: Unified search across tasks and events
- ✅ **Focus-Driven Workflow**: Start, switch, and complete tasks
- ✅ **Multi-Project Support**: Manage multiple projects simultaneously

---

## Quick Start

### 1. Start the Dashboard

Navigate to your project directory and start the dashboard:

```bash
cd /path/to/your/project
ie dashboard start
```

The dashboard will automatically:
- Detect the project database (`.intent-engine/intents.db`)
- Register the project in `~/.intent-engine/projects.json`
- Allocate an available port (3030-3099)
- Display the URL to access

Example output:
```
Dashboard starting for project: my-project
  Port: 3030
  URL: http://127.0.0.1:3030
  Mode: background

✅ Dashboard server started successfully
   PID: 12345
   URL: http://127.0.0.1:3030

Tip: Use 'ie dashboard status' to check server status
Tip: Use 'ie dashboard stop' to stop the server
```

### 2. Access the Dashboard

Open your web browser and navigate to the URL shown (e.g., `http://127.0.0.1:3030`).

You'll see the Dashboard interface with:
- **Left Sidebar**: Task list with filters
- **Main Panel**: Task details and operations
- **Right Sidebar**: Event history

### 3. Stop the Dashboard

To stop the dashboard server:

```bash
ie dashboard stop
```

Or, if running in foreground mode (with `--foreground` flag):
- Press `Ctrl+C` in the terminal

---

## Dashboard Interface

### Header

The header contains:
- **Project Name**: Currently active project
- **New Task**: Create a new top-level task
- **Current Focus**: Jump to the currently focused task
- **Pick Next**: Get a smart recommendation for the next task

### Left Sidebar: Task List

#### Search Bar
- Type to search across tasks and events
- Results update as you type (300ms debounce)
- Supports full-text search with operators (`AND`, `OR`, `NOT`)

#### Filter Buttons
- **All**: Show all tasks
- **Todo**: Show only tasks in "todo" status
- **Doing**: Show only tasks in "doing" status
- **Done**: Show only completed tasks

#### Task Cards
Each task card displays:
- **Task ID**: `#42`
- **Task Name**: Truncated if too long
- **Status Badge**: Color-coded (yellow/blue/green)
- **Priority Badge**: Critical/High/Medium/Low (if set)
- **Parent Indicator**: "Parent: #10" if task has a parent

**Active Task**: The currently focused task is highlighted with a blue background.

### Main Panel: Task Details

When you select a task, the main panel shows:

#### Task Header
- **Task ID** and **Name**
- **Status Badge**: Current status
- **Priority Badge**: If set
- **Parent Link**: Clickable link to parent task

#### Action Buttons

**For Todo Tasks**:
- **▶ Start Task**: Begin working on this task (sets focus)

**For Doing Tasks**:
- **✓ Complete Task**: Mark task as done (focus-driven, no ID needed)
- **+ Spawn Subtask**: Create a child task and switch to it

**For All Tasks**:
- **⇄ Switch to This**: Change focus to this task
- **📝 Add Event**: Record a decision, blocker, milestone, or note
- **🗑 Delete**: Remove the task (only if no subtasks)

#### Specification Section
- Rendered Markdown with:
  - Headings, lists, tables
  - Code blocks with syntax highlighting
  - Blockquotes, links, images
  - Safe HTML (XSS protection via DOMPurify)

#### Metadata Section
Displays timestamps:
- **Created**: When task was first created (`first_todo_at`)
- **Started**: When task was first started (`first_doing_at`)
- **Completed**: When task was completed (`first_done_at`)

### Right Sidebar: Event History

Shows events related to the current task:

#### Event Types
- **💡 Decision**: Important decisions and their rationale
- **🚫 Blocker**: Problems preventing progress
- **🎯 Milestone**: Significant achievements
- **📝 Note**: General observations and comments

Each event card shows:
- **Type Badge**: Color-coded by type
- **Timestamp**: When the event was logged
- **Content**: Rendered Markdown

---

## Common Workflows

### Create a New Task

1. Click **+ New Task** in the header
2. Fill in the form:
   - **Name** (required): Brief task description
   - **Specification** (optional): Detailed description in Markdown
   - **Priority** (optional): Critical/High/Medium/Low
   - **Parent Task ID** (optional): ID of parent task
3. Click **Create Task**

The new task appears in the task list and opens automatically.

### Work on a Task

#### Start a Task
1. Find the task in the list
2. Click to open details
3. Click **▶ Start Task**

The task status changes to "doing" and becomes the current focus.

#### Add Events During Work
While working on a task, record important information:

1. Click **📝 Add Event**
2. Select event type:
   - **Decision**: "Chose JWT over sessions because..."
   - **Blocker**: "Blocked by missing API keys"
   - **Milestone**: "Completed authentication module"
   - **Note**: "Found useful reference: https://..."
3. Write content in Markdown
4. Click **Add Event**

Events appear in the right sidebar chronologically.

#### Complete the Task
1. Ensure the task is your current focus
2. Click **✓ Complete Task**

The task status changes to "done" and focus is cleared.

**Note**: You cannot complete a task that has incomplete subtasks.

### Break Down a Large Task

For complex tasks, create subtasks:

1. Start the parent task (must be focused)
2. Click **+ Spawn Subtask**
3. Enter:
   - **Subtask Name**: Specific sub-goal
   - **Specification** (optional): Details
4. Submit

The subtask is created and **automatically becomes the new focus**.

### Search for Information

#### Quick Search
1. Type in the search bar (left sidebar)
2. View results as you type
3. Click a task to view details

#### Advanced Search
Use the API directly for advanced queries:
```bash
# Search with operators
curl "http://127.0.0.1:3030/api/search?query=JWT+AND+authentication"

# Search events only
curl "http://127.0.0.1:3030/api/search?query=blocker&include_tasks=false"
```

### Get Task Recommendations

Click **Pick Next** in the header to get a smart recommendation based on:
- **Depth-First**: Subtasks of current task (if any)
- **Priority**: Higher priority tasks first
- **Age**: Older tasks first (among same priority)

The recommended task opens automatically.

---

## Multi-Project Support

### Managing Multiple Projects

You can run dashboards for multiple projects simultaneously:

```bash
# Project A
cd /path/to/project-a
ie dashboard start
# Runs on port 3030

# Project B
cd /path/to/project-b
ie dashboard start
# Runs on port 3031 (auto-allocated)

# Project C
cd /path/to/project-c
ie dashboard start --port 3050
# Runs on specific port 3050
```

### Check Running Dashboards

```bash
ie dashboard list
```

Example output:
```
Active Dashboard Servers:

  project-a
    Path: /path/to/project-a
    Port: 3030
    URL:  http://127.0.0.1:3030
    PID:  12345

  project-b
    Path: /path/to/project-b
    Port: 3031
    URL:  http://127.0.0.1:3031
    PID:  12346
```

### Stop a Specific Dashboard

```bash
# Stop dashboard in current directory
cd /path/to/project-a
ie dashboard stop

# Or stop by port
ie dashboard stop --port 3030
```

### Stop All Dashboards

```bash
ie dashboard stop-all
```

---

## Tips and Best Practices

### Writing Good Specifications

Use Markdown to make specs readable:

````markdown
# Task: Implement User Authentication

## Objective
Add JWT-based authentication to the API.

## Requirements
- Login endpoint: `POST /auth/login`
- Token refresh: `POST /auth/refresh`
- Logout: `POST /auth/logout`

## Technical Notes
```javascript
// Example token payload
{
  "user_id": 123,
  "email": "user@example.com",
  "exp": 1234567890
}
```

## Acceptance Criteria
- [ ] Users can log in with email/password
- [ ] Tokens expire after 1 hour
- [ ] Refresh tokens work for 7 days
````

### Using Events Effectively

**Decisions**: Record the "why" behind choices
```markdown
**Decision**: Use HS256 for JWT signing

**Rationale**:
- Simpler than RS256 (no key pair management)
- Sufficient for our single-server setup
- Can upgrade to RS256 if we add microservices
```

**Blockers**: Be specific about what's blocking you
```markdown
**Blocked by**: Missing AWS credentials

**Details**:
- Need S3 access keys for file upload feature
- Requested from DevOps team (Ticket #456)
- ETA: 2 days
```

**Milestones**: Celebrate progress
```markdown
**Milestone**: Authentication module complete

**Achievement**:
- ✅ Login/logout working
- ✅ Token refresh implemented
- ✅ All tests passing (12/12)
- ✅ Documentation updated
```

### Keyboard Shortcuts (Future)

Phase 2+ will add keyboard shortcuts:
- `n`: New task
- `f`: Focus search
- `/`: Quick search
- `Escape`: Close modals

---

## Troubleshooting

### Dashboard Won't Start

**Error**: `Database not found`

**Solution**: Make sure you're in an Intent-Engine project directory:
```bash
# Initialize project first
ie setup

# Then start dashboard
ie dashboard start
```

**Error**: `Port 3030 already in use`

**Solution**: Use a different port:
```bash
ie dashboard start --port 3040
```

Or stop the existing dashboard:
```bash
ie dashboard stop
```

### Page Not Loading

1. **Check server is running**:
   ```bash
   ie dashboard status
   ```

2. **Check firewall**:
   - Dashboard binds to `127.0.0.1` (localhost only)
   - No external access by default

3. **Try another browser**:
   - Tested: Chrome, Firefox, Safari, Edge

### Task Not Appearing

1. **Refresh the page**: No auto-reload in Phase 1
2. **Check filters**: Make sure you're not filtering it out
3. **Search for it**: Use the search bar to find the task

### Can't Complete Task

**Error**: "Task has incomplete subtasks"

**Solution**: Complete all subtasks first, then the parent:
```bash
# Via CLI
ie task list --parent 42  # Find subtasks
ie task start 43           # Start subtask
# ... work on it ...
ie task done               # Complete subtask
ie task start 42           # Back to parent
ie task done               # Now you can complete parent
```

### Performance Issues

**Problem**: Slow page load with 500+ tasks

**Solution** (Phase 2+):
- Enable pagination
- Use filters to reduce visible tasks
- Archive completed tasks

**Workaround** (Phase 1):
- Use filters (Todo/Doing) instead of "All"
- Search for specific tasks instead of browsing
- Clean up old completed tasks

---

## Limitations (Phase 1)

### Known Limitations

1. **No Real-Time Updates**
   - Manual page refresh required
   - Phase 2+ will add WebSocket support

2. **No Authentication**
   - Localhost only (127.0.0.1)
   - Not suitable for multi-user setups
   - Phase 2+ will add API keys and JWT

3. **Basic Error Handling**
   - Errors shown via browser alerts
   - Phase 2+ will add toast notifications

4. **No Undo/Redo**
   - Deletes are permanent
   - Be careful when deleting tasks

5. **Performance with Large Datasets**
   - No pagination (loads all tasks)
   - May slow down with 1000+ tasks
   - Phase 2+ will add virtual scrolling

### Workarounds

- Use CLI for bulk operations
- Regularly archive completed tasks
- Use search instead of browsing large lists

---

## Browser Compatibility

### Tested Browsers

- ✅ Chrome 120+
- ✅ Firefox 121+
- ✅ Safari 17+
- ✅ Edge 120+

### Required Features

- ES6 JavaScript
- Fetch API
- CSS Grid
- LocalStorage (for future features)

---

## Integration with CLI and MCP

### Data Sync

All three interfaces (Dashboard, CLI, MCP) share the same database:

```
.intent-engine/intents.db
         ↓
    ┌────┴────┬─────┬────────┐
    ↓         ↓     ↓        ↓
Dashboard    CLI   MCP    External
```

Changes made in one interface are immediately visible in others (after refresh for Dashboard).

### Example Workflow

```bash
# Create task via CLI
ie task add "Implement feature X"

# Refresh Dashboard → Task appears

# Start task via Dashboard (click button)

# Add event via MCP (from Claude)
ie event add --type decision "Using approach A"

# Refresh Dashboard → Event appears

# Complete via CLI
ie task done

# Refresh Dashboard → Task marked done
```

---

## Getting Help

### Documentation

- **API Reference**: `docs/dashboard-api-reference.md`
- **Architecture**: `docs/web-dashboard-spec.md`
- **CLI Guide**: `docs/*/guide/command-reference-full.md`

### Support

- **Issues**: https://github.com/wayfind/intent-engine/issues
- **Discussions**: GitHub Discussions

---

## Appendix: Example Screenshots (Text)

### Main Dashboard View
```
┌──────────────────────────────────────────────────────────────────┐
│ Intent-Engine Dashboard          [+] New  [Focus]  [Pick Next]  │
├──────────────────────────────────────────────────────────────────┤
│  Search...         │                                             │
│ ┌────────────────┐ │  Task #42: Implement Authentication        │
│ │All│Todo│Doing│ │ │  Status: doing    Priority: high           │
│ └────────────────┘ │                                             │
│                    │  [✓ Complete] [+ Spawn] [📝 Event]          │
│ #42 Implement A... │                                             │
│ doing | high       │  ## Specification                           │
│                    │  Add JWT-based authentication...            │
│ #43 Setup DB       │                                             │
│ todo               │  ## Metadata                                │
│                    │  Created: 2025-11-16 12:00                  │
│ #44 Write Tests    │  Started: 2025-11-16 13:00                  │
│ todo               │                                             │
└────────────────────┴─────────────────────────────────────────────┘
```

---

**Last Updated**: 2025-11-16
**Version**: 0.5.0
