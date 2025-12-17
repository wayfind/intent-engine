# Intent-Engine Web Dashboard Specification

**Version**: 0.1.0
**Last Updated**: 2025-11-16
**Status**: Design Draft

---

## 1. Overview

### 1.1 Design Goals

The Web Dashboard provides a visual interface for Intent-Engine, enabling users to:
- View task hierarchy and status at a glance
- Create and manage tasks via browser
- Track decision history and milestones
- Monitor project progress with real-time updates

### 1.2 Core Design Principles

1. **Database as Single Source of Truth**
   - No additional notification tables
   - Web and Claude Code both read/write to SQLite directly
   - Natural discovery through existing queries

2. **Simplicity First**
   - Minimal dependencies
   - Clear separation of concerns
   - Progressive enhancement (basic features → advanced)

3. **Loose Coupling**
   - Dashboard is optional component
   - Works independently of Claude Code
   - No modifications to core Intent-Engine logic

4. **Multi-Project Support**
   - Each project gets its own dashboard instance
   - Automatic port allocation and management
   - Project isolation and discovery

5. **Rich Markdown Rendering**
   - Beautiful rendering of task specifications
   - Syntax highlighting for code blocks
   - Support for tables, lists, and formatting

### 1.3 Architecture Diagram

```
┌─────────────────────┐         ┌──────────────────────┐
│   Claude Code       │         │   Web Dashboard      │
│                     │         │   (Browser)          │
│  ┌───────────────┐  │         │  ┌────────────────┐  │
│  │ MCP Tools     │  │         │  │  UI Components │  │
│  │ - task_add    │  │         │  │  - Task List   │  │
│  │ - task_start  │  │         │  │  - Timeline    │  │
│  │ - task_done   │  │         │  │  - Stats       │  │
│  └───────┬───────┘  │         │  └────────┬───────┘  │
└──────────┼──────────┘         └───────────┼──────────┘
           │                                 │
           │  Read/Write                     │  HTTP API
           │  (via Rust lib)                 │  REST + WS
           │                                 │
           └─────────┬───────────────────────┘
                     │
              ┌──────▼──────────┐
              │  SQLite Database│
              │                 │
              │  ┌───────────┐  │
              │  │  tasks    │  │
              │  │  events   │  │
              │  └───────────┘  │
              └─────────────────┘
```

---

## 2. Communication Architecture

### 2.1 Web → Claude Code (Passive Discovery)

**Mechanism**: Database writes + Natural queries

```
1. User creates task in Web Dashboard
   ↓
2. POST /api/tasks → Write to tasks table
   ↓
3. Later: Claude Code calls task_pick_next or task_list
   ↓
4. AI discovers new task naturally
   ↓
5. AI: "I see a new task #123 from Web Dashboard"
```

**Key Insight**: No push notification needed. Tasks are discovered through normal workflow.

**Edge Case Handling**:
- **Session Start**: Hook displays recent tasks
- **Mid-session**: User can ask "Any new tasks?"
- **Urgent tasks**: User tells Claude directly

### 2.2 Claude Code → Web (Active Push)

**Mechanism**: WebSocket events (optional)

```
1. MCP tool executed (e.g., task_start)
   ↓
2. Rust code: After DB write, broadcast WS event
   ↓
3. Web Dashboard receives event
   ↓
4. UI updates in real-time
```

**Fallback**: If WebSocket unavailable, Web polls via REST API

---

## 3. Data Model

### 3.1 Existing Tables (No Changes Needed)

**tasks** table:
```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    spec TEXT,
    status TEXT NOT NULL,  -- 'todo', 'doing', 'done'
    priority INTEGER DEFAULT 0,
    parent_id INTEGER,
    first_todo_at TEXT,
    first_doing_at TEXT,
    first_done_at TEXT,
    FOREIGN KEY (parent_id) REFERENCES tasks(id)
);
```

**events** table:
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    task_id INTEGER NOT NULL,
    log_type TEXT NOT NULL,  -- 'decision', 'blocker', 'milestone', 'note'
    discussion_data TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

### 3.2 Why No New Tables?

1. **Notifications Not Needed**: Discovery happens naturally
2. **Keep It Simple**: Fewer moving parts
3. **Single Source**: Database already contains all state

---

## 4. Web Dashboard Features

### 4.1 Core Views

#### 4.1.1 Task Overview
```
┌─────────────────────────────────────────────┐
│  Intent-Engine Dashboard                    │
├─────────────────────────────────────────────┤
│  Current Task: #42 Implement Auth System    │
│  Status: Doing  │  Priority: High           │
├─────────────────────────────────────────────┤
│  Task Tree:                                 │
│  ○ #40 User Management                      │
│    ○ #41 Design DB Schema                   │
│    → #42 Implement Auth System              │
│      ○ #43 JWT Token Generation             │
│    ○ #44 Add Permission System              │
│  ✓ #45 Setup Project                        │
├─────────────────────────────────────────────┤
│  [+ New Task]  [Filter]  [Search]           │
└─────────────────────────────────────────────┘
```

**Features**:
- Tree view with expand/collapse
- Status icons: ○ (todo), → (doing), ✓ (done)
- Current task highlighted
- Click task to view details

#### 4.1.2 Task Detail Panel
```
┌─────────────────────────────────────────────┐
│  Task #42: Implement Auth System            │
├─────────────────────────────────────────────┤
│  Status: Doing                              │
│  Priority: High (p1)                        │
│  Parent: #40 User Management                │
│  Created: 2025-11-15 10:30                  │
│  Started: 2025-11-15 14:20                  │
├─────────────────────────────────────────────┤
│  Specification:                             │
│  # Authentication Implementation            │
│  - JWT-based authentication                 │
│  - Refresh token rotation                   │
│  - Rate limiting                            │
├─────────────────────────────────────────────┤
│  Events (3):                                │
│  [decision] Chose JWT over sessions         │
│  [milestone] Completed token generation     │
│  [note] Need to add rate limiting           │
├─────────────────────────────────────────────┤
│  Children (1):                              │
│  ○ #43 JWT Token Generation                 │
├─────────────────────────────────────────────┤
│  [Start] [Complete] [Edit] [Add Event]     │
└─────────────────────────────────────────────┘
```

#### 4.1.3 Event Timeline
```
┌─────────────────────────────────────────────┐
│  Recent Events                              │
├─────────────────────────────────────────────┤
│  14:45 [milestone] Task #42: JWT impl done  │
│  14:20 [decision] Task #42: Use bcrypt      │
│  13:30 [note] Task #40: Need API design     │
│  12:00 [blocker] Task #45: Missing deps     │
├─────────────────────────────────────────────┤
│  Filter: [All] [Decisions] [Blockers]       │
└─────────────────────────────────────────────┘
```

#### 4.1.4 Statistics Dashboard
```
┌─────────────────────────────────────────────┐
│  Project Statistics                         │
├─────────────────────────────────────────────┤
│  Total Tasks: 12                            │
│  Completed: 5 (42%)                         │
│  In Progress: 2                             │
│  Pending: 5                                 │
├─────────────────────────────────────────────┤
│  Priority Distribution:                     │
│  Critical: 1   High: 3   Medium: 6  Low: 2  │
├─────────────────────────────────────────────┤
│  Recent Activity (7 days):                  │
│  Tasks Created: 8                           │
│  Tasks Completed: 5                         │
│  Events Logged: 23                          │
└─────────────────────────────────────────────┘
```

### 4.2 User Interactions

#### 4.2.1 Create Task
```
Action: Click [+ New Task]
Form:
  - Name: [Required text input]
  - Specification: [Markdown editor]
  - Priority: [Dropdown: Critical/High/Medium/Low]
  - Parent Task: [Optional dropdown of existing tasks]

Submit → POST /api/tasks → Database write → Success message
```

#### 4.2.2 Update Task Status
```
From Task Detail:
  - Click [Start] → POST /api/tasks/:id/start
  - Click [Complete] → POST /api/tasks/:id/done

From Task List:
  - Drag-and-drop status column (future enhancement)
```

#### 4.2.3 Add Event
```
From Task Detail:
  Click [Add Event]
  Form:
    - Type: [Radio: Decision/Blocker/Milestone/Note]
    - Content: [Markdown textarea]

Submit → POST /api/events → Database write
```

---

## 5. Technical Stack

### 5.1 Backend

**Framework**: Rust + Axum

**Rationale**:
- Reuse existing Intent-Engine codebase
- High performance, low resource usage
- Built-in async/await for WebSocket
- Type safety

**Structure**:
```
src/
├── bin/
│   └── dashboard.rs        # Entry point: ie dashboard start
├── dashboard/
│   ├── mod.rs
│   ├── server.rs           # Axum HTTP server
│   ├── routes.rs           # REST API endpoints
│   ├── websocket.rs        # WebSocket handler (optional)
│   └── static.rs           # Serve frontend files
└── (existing modules)
    ├── task_manager.rs     # Reused for DB operations
    └── event_manager.rs
```

**Dependencies** (add to Cargo.toml):
```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio-tungstenite = "0.21"  # For WebSocket
```

### 5.2 Frontend

**Recommended: Option 1 - HTMX + TailwindCSS**

**Rationale**:
- Minimal JavaScript
- Server-driven UI updates
- Fast development
- Low bundle size

**Structure**:
```
static/
├── index.html
├── css/
│   └── tailwind.min.css
├── js/
│   └── htmx.min.js
└── favicon.ico
```

**Alternative Options**:

| Option | Pros | Cons |
|--------|------|------|
| **React + TS** | Rich ecosystem, great dev tools | Heavier, more complex |
| **Svelte** | Lightweight, reactive | Less familiar |
| **Vue.js** | Gentle learning curve | Medium weight |

#### Markdown Rendering

**Requirement**: Task specifications and event notes are written in Markdown and must be rendered beautifully.

**Recommended Library**: `marked.js` + `highlight.js`

**Features Needed**:
- GitHub Flavored Markdown (GFM) support
- Syntax highlighting for code blocks
- Tables, task lists, strikethrough
- Auto-linking URLs
- XSS protection (sanitization)

**Setup**:
```html
<!-- Include libraries -->
<script src="https://cdn.jsdelivr.net/npm/marked@11.0.0/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/highlight.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github-dark.min.css">

<script>
// Configure marked with GFM and syntax highlighting
marked.setOptions({
  gfm: true,               // GitHub Flavored Markdown
  breaks: true,            // Line breaks become <br>
  sanitize: false,         // We'll use DOMPurify for sanitization
  highlight: function(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang }).value;
    }
    return hljs.highlightAuto(code).value;
  }
});

// XSS protection
const renderMarkdown = (markdown) => {
  const html = marked.parse(markdown);
  return DOMPurify.sanitize(html);
};
</script>
```

**Alternative**: `markdown-it` (more extensible, plugins)

**Usage in UI**:
```html
<!-- Task specification -->
<div class="spec-content prose prose-slate max-w-none">
  <div id="task-spec"></div>
</div>

<script>
const spec = task.spec; // From API
document.getElementById('task-spec').innerHTML = renderMarkdown(spec);
</script>
```

**Styling with Tailwind Prose**:
```html
<!-- Install @tailwindcss/typography -->
<div class="prose prose-slate dark:prose-invert max-w-none">
  <!-- Rendered markdown here -->
</div>
```

**Features**:
- Headings styled properly (# → h1, ## → h2, etc.)
- Code blocks with syntax highlighting
- Tables with borders
- Lists with proper indentation
- Blockquotes styled
- Links colored and underlined

**Security Considerations**:
```javascript
// IMPORTANT: Sanitize markdown to prevent XSS
import DOMPurify from 'dompurify';

const renderSafeMarkdown = (markdown) => {
  const rawHtml = marked.parse(markdown);
  return DOMPurify.sanitize(rawHtml, {
    ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'u', 'h1', 'h2', 'h3',
                   'ul', 'ol', 'li', 'code', 'pre', 'blockquote',
                   'a', 'table', 'thead', 'tbody', 'tr', 'th', 'td'],
    ALLOWED_ATTR: ['href', 'class', 'id']
  });
};
```

**Example Rendered Output**:

Input Markdown:
````markdown
# Authentication Implementation

## Overview
Implement JWT-based authentication with the following features:

- User login/logout
- Token refresh mechanism
- Role-based access control

## Technical Decisions

### Token Storage
We chose **httpOnly cookies** over localStorage because:
1. Better security (immune to XSS)
2. Automatic transmission
3. Built-in expiration

### Code Example
```rust
use jsonwebtoken::{encode, decode, Header, Validation};

fn create_token(user_id: i64) -> String {
    let claims = Claims { user_id, exp: ... };
    encode(&Header::default(), &claims, &secret()).unwrap()
}
```

## Next Steps
- [ ] Implement refresh token rotation
- [ ] Add rate limiting
- [x] Basic JWT generation
````

Rendered HTML (styled):
```
[Rendered output with:]
- Large, bold "Authentication Implementation" heading
- Medium "Overview" and "Technical Decisions" headings
- Bullet list with proper indentation
- Bold "httpOnly cookies" text
- Numbered sub-list
- Syntax-highlighted Rust code block with dark theme
- Task list with checkboxes (some checked, some unchecked)
```

**Performance Optimization**:
```javascript
// Cache rendered markdown
const markdownCache = new Map();

function renderMarkdownCached(markdown) {
  if (markdownCache.has(markdown)) {
    return markdownCache.get(markdown);
  }
  const html = renderSafeMarkdown(markdown);
  markdownCache.set(markdown, html);
  return html;
}
```

**Mobile Responsiveness**:
```html
<!-- Use prose-sm on mobile -->
<div class="prose prose-sm sm:prose lg:prose-lg
            prose-slate dark:prose-invert
            max-w-none">
  <!-- Markdown content -->
</div>
```

**Dependency Summary**:
```json
{
  "dependencies": {
    "marked": "^11.0.0",
    "highlight.js": "^11.9.0",
    "dompurify": "^3.0.8"
  }
}
```

Or via CDN (for HTMX option):
```html
<script src="https://cdn.jsdelivr.net/npm/marked@11.0.0/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/highlight.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/dompurify@3.0.8/dist/purify.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github-dark.min.css">
```

### 5.3 Communication

#### REST API
- **Purpose**: CRUD operations
- **Format**: JSON
- **Authentication**: Optional token header

#### WebSocket (Optional)
- **Purpose**: Real-time UI updates
- **Protocol**: Text frames with JSON messages
- **Events**: task.*, event.*

---

## 6. API Specification

### 6.1 REST Endpoints

#### Tasks

**GET /api/tasks**
```json
Query params:
  ?status=todo|doing|done
  ?parent=<task_id>  (null for top-level)
  ?sort=created_at|priority|id
  ?limit=50

Response: 200 OK
[
  {
    "id": 42,
    "name": "Implement Auth",
    "spec": "# Specification...",
    "status": "doing",
    "priority": 1,
    "parent_id": 40,
    "first_todo_at": "2025-11-15T10:30:00Z",
    "first_doing_at": "2025-11-15T14:20:00Z",
    "first_done_at": null
  }
]
```

**GET /api/tasks/:id**
```json
Response: 200 OK
{
  "task": { /* task object */ },
  "children": [ /* child tasks */ ],
  "events": [ /* recent events */ ]
}
```

**POST /api/tasks**
```json
Request:
{
  "name": "New Task",
  "spec": "# Specification",
  "priority": 2,
  "parent_id": 40  // optional
}

Response: 201 Created
{
  "id": 43,
  "name": "New Task",
  ...
}
```

**PUT /api/tasks/:id**
```json
Request:
{
  "name": "Updated Name",  // optional
  "spec": "...",           // optional
  "priority": 1            // optional
}

Response: 200 OK
{ /* updated task */ }
```

**DELETE /api/tasks/:id**
```json
Response: 204 No Content
```

#### Task Actions

**POST /api/tasks/:id/start**
```json
Response: 200 OK
{
  "task": { /* task with status=doing */ },
  "previous_task": { /* previous current task if any */ }
}
```

**POST /api/tasks/:id/done**
```json
Response: 200 OK
{
  "completed_task": { /* task with status=done */ },
  "next_suggestion": { /* pick_next result */ }
}
```

**POST /api/tasks/:id/spawn-subtask**
```json
Request:
{
  "name": "Subtask Name",
  "spec": "..."
}

Response: 201 Created
{ /* created subtask */ }
```

#### Events

**GET /api/events**
```json
Query params:
  ?task_id=<id>
  ?type=decision|blocker|milestone|note
  ?since=7d (format: Nd, Nh, Nm)
  ?limit=50

Response: 200 OK
{
  "events": [
    {
      "id": 100,
      "task_id": 42,
      "log_type": "decision",
      "discussion_data": "Chose JWT because...",
      "timestamp": "2025-11-15T14:30:00Z"
    }
  ],
  "total_count": 123
}
```

**POST /api/events**
```json
Request:
{
  "task_id": 42,
  "type": "decision",
  "data": "Chose approach X because..."
}

Response: 201 Created
{ /* created event */ }
```

#### Current Task

**GET /api/current**
```json
Response: 200 OK
{
  "current_task_id": 42,
  "task": { /* task object */ }
}

Or if no current task:
{
  "current_task_id": null,
  "task": null
}
```

#### Statistics

**GET /api/stats**
```json
Query params:
  ?since=7d

Response: 200 OK
{
  "total_tasks": 12,
  "by_status": {
    "todo": 5,
    "doing": 2,
    "done": 5
  },
  "by_priority": {
    "0": 2,  // critical
    "1": 3,  // high
    "2": 6,  // medium
    "3": 1   // low
  },
  "recent_activity": {
    "tasks_created": 8,
    "tasks_completed": 5,
    "events_logged": 23
  }
}
```

### 6.2 WebSocket Protocol (Optional)

**Connection**: `ws://localhost:11391/ws`

**Client → Server (Subscribe)**:
```json
{
  "type": "subscribe",
  "events": ["task.*", "event.*"]
}
```

**Server → Client (Events)**:
```json
// Task created
{
  "type": "task.created",
  "payload": { /* task object */ },
  "timestamp": "2025-11-15T14:30:00Z"
}

// Task updated
{
  "type": "task.updated",
  "payload": {
    "id": 42,
    "changes": { "status": "doing" }
  },
  "timestamp": "..."
}

// Event added
{
  "type": "event.added",
  "payload": { /* event object */ },
  "timestamp": "..."
}
```

---

## 7. Daemon Process Design & Multi-Project Support

### 7.1 Multi-Project Architecture

**Problem**: Claude Code can run in multiple project directories simultaneously. Each project needs its own dashboard instance to avoid conflicts.

**Solution**: Global registry + Automatic port allocation

```
~/.intent-engine/
├── projects/
│   ├── project-a/
│   │   ├── intents.db
│   │   └── dashboard.pid
│   └── project-b/
│       ├── intents.db
│       └── dashboard.pid
└── dashboard-global.log

Note: projects.json (Global registry) removed in v0.6.0
      Replaced by WebSocket-based in-memory state
```

### 7.2 Project Registry

> **DEPRECATED**: File-based registry (`~/.intent-engine/projects.json`) was removed in v0.6.0

**Current Implementation** (v0.6.0+):
- Projects are tracked via **WebSocket connections** (in-memory state)
- Dashboard runs on fixed port **11391**
- MCP clients register by establishing WebSocket connection
- State is ephemeral - persists only while Dashboard and MCP are running
- Query active projects via `/api/projects` HTTP endpoint

**Legacy System** (v0.5.x and earlier):
- Used `~/.intent-engine/projects.json` for persistent registry
- Supported dynamic port allocation (11391, 11392, ...)
- File-based synchronization across multiple dashboards

**Migration**: No action required - WebSocket registration is automatic when MCP connects to Dashboard

### 7.3 Project Identification

**Option 1: Automatic Detection (Recommended)**
```bash
# In /home/user/projects/intent-engine/
ie dashboard start

# Automatically:
# 1. Detect project root (has .intent-engine/ or .git/)
# 2. Use project name from directory name
# 3. Use fixed port 11391 (or custom port if specified)
# 4. Start WebSocket server for MCP connections
```

**Option 2: Explicit Project Name**
```bash
ie dashboard start --project my-project
```

**Project Root Detection Logic**:
1. Look for `.intent-engine/` directory (Intent-Engine project marker)
2. Fall back to `.git/` directory (Git repository)
3. Use directory name as project name
4. Normalize name (lowercase, replace spaces)

### 7.4 Port Management Strategy

**Fixed Port Strategy (Current Implementation)**:
```
Default port: 11391
Custom ports: Available via --port flag
Multi-project: Requires manual port specification for additional projects
```

**Rationale**:
- Simplicity: Single well-known port for default usage
- Predictability: Always know where the dashboard is running
- Flexibility: Custom ports available when needed via --port flag

### 7.5 CLI Commands

**Start Dashboard**:
```bash
ie dashboard start [OPTIONS]

Options:
  --project <NAME>      Project name (default: auto-detect)
  --port <PORT>         Force specific port (default: 11391 by default)
  --host <HOST>         Bind address (default: 127.0.0.1)
  --websocket           Enable WebSocket support (default: false)
  --cors                Enable CORS for development (default: false)
  --daemon              Run in background (default: false)

Examples:
  # Auto-detect project, use default port 11391
  ie dashboard start

  # Specify project name
  ie dashboard start --project cortex

  # Force specific port
  ie dashboard start --port 8080

  # Background daemon with WebSocket
  ie dashboard start --daemon --websocket
```

**Behavior**:
1. Detect project root and name
2. Check if dashboard already running for this project
3. If not, allocate port from registry
4. Start server on allocated port
5. Update registry
6. Print access URL

**Output Example**:
```
✓ Detected project: intent-engine
✓ Allocated port: 11391
✓ Dashboard starting...
✓ Running on http://127.0.0.1:11391

Open in browser or use:
  ie dashboard open
```

**Stop Dashboard**:
```bash
ie dashboard stop [OPTIONS]

Options:
  --project <NAME>      Project to stop (default: current directory)
  --all                 Stop all running dashboards

Examples:
  ie dashboard stop                    # Stop current project
  ie dashboard stop --project cortex   # Stop specific project
  ie dashboard stop --all              # Stop all projects
```

**Behavior**:
1. Lookup project in registry
2. Send SIGTERM to process (PID from registry)
3. Wait for graceful shutdown (max 5s)
4. Remove from registry
5. Free up port

**Status Check**:
```bash
ie dashboard status [OPTIONS]

Options:
  --all                 Show all running dashboards

Examples:
  ie dashboard status            # Current project
  ie dashboard status --all      # All projects
```

**Output Example**:
```
Current Project: intent-engine
  URL: http://127.0.0.1:11391
  PID: 12345
  Port: 11391
  Uptime: 2h 15m
  WebSocket: Enabled
  Database: /home/user/projects/intent-engine/.intent-engine/intents.db

Other Running Dashboards:
  cortex
    URL: http://127.0.0.1:11392
    PID: 12346
    Uptime: 1h 30m
```

**List All Projects**:
```bash
ie dashboard list

Output:
  Running Dashboards:
  ● intent-engine  http://127.0.0.1:11391  (2h 15m)
  ● cortex         http://127.0.0.1:11392  (1h 30m)

  Stopped Projects:
  ○ old-project    (last active: 2 days ago)
```

**Open in Browser**:
```bash
ie dashboard open [--project <NAME>]

# Opens default browser to dashboard URL
# Uses current project if --project not specified
```

### 7.6 Configuration

**Global Config**: `~/.intent-engine/dashboard-config.toml`

```toml
[server]
host = "127.0.0.1"
default_port = 11391
enable_websocket = false

[projects]
auto_detect = true              # Auto-detect project root
max_concurrent = 10             # Max simultaneous dashboards

[cors]
enabled = false
allowed_origins = ["http://localhost:5173"]

[logging]
level = "info"
file = "~/.intent-engine/dashboard-global.log"

[security]
enable_auth = false
api_key = ""

[ui]
theme = "auto"                  # auto, light, dark
default_view = "tasks"          # tasks, timeline, stats
```

**Per-Project Config** (optional): `.intent-engine/dashboard.toml`

```toml
[server]
# Override global settings for this project
port = 8080                     # Force specific port
enable_websocket = true

[ui]
theme = "dark"
```

### 7.7 Process Management

**PID File**: Per-project
```
.intent-engine/dashboard.pid
```

**Lock File**: Prevent multiple instances
```
.intent-engine/dashboard.lock
```

**Startup Flow**:
```
1. Detect project root
2. Check lock file
   - If exists: Read PID, check if process alive
   - If alive: Error "Already running"
   - If dead: Remove lock, continue
3. Load global registry
4. Allocate port (or use configured port)
5. Check port availability
6. Start Axum server
7. Write PID file and lock
8. Register in global registry
9. Setup signal handlers
```

**Shutdown Flow**:
```
1. Receive SIGTERM/SIGINT
2. Log shutdown initiation
3. Close all WebSocket connections
4. Stop accepting new HTTP connections
5. Wait for in-flight requests (max 5s)
6. Close database connections
7. Remove PID and lock files
8. Unregister from global registry
9. Exit
```

### 7.8 Conflict Resolution

**Port Already in Use**:
```rust
// Try allocated port
if !is_port_available(allocated_port) {
    // Try next 10 ports
    for port in allocated_port + 1..allocated_port + 11 {
        if is_port_available(port) {
            return port;
        }
    }
    // Give up
    error!("No available ports in range");
}
```

**Project Already Running**:
```bash
$ ie dashboard start
Error: Dashboard already running for this project
  URL: http://127.0.0.1:11391
  PID: 12345

Tip: Use 'ie dashboard stop' to stop it first
     Or 'ie dashboard open' to open existing dashboard
```

**Stale PID Files**:
```rust
// On startup, check if PID is still alive
if pid_file.exists() {
    let pid = read_pid();
    if !is_process_alive(pid) {
        // Process died, clean up
        remove_pid_file();
        remove_from_registry();
    }
}
```

### 7.9 Example: Multi-Project Workflow

```bash
# Terminal 1: Working on intent-engine
cd ~/projects/intent-engine
ie dashboard start
# ✓ Dashboard running on http://127.0.0.1:11391

# Terminal 2: Working on cortex
cd ~/projects/cortex
ie dashboard start
# ✓ Dashboard running on http://127.0.0.1:11392

# Terminal 3: Check all running dashboards
ie dashboard status --all
# intent-engine  http://127.0.0.1:11391  (running)
# cortex         http://127.0.0.1:11392  (running)

# Stop specific project
cd ~/projects/cortex
ie dashboard stop
# ✓ Stopped dashboard for cortex

# Or stop all
ie dashboard stop --all
# ✓ Stopped intent-engine
# ✓ Stopped cortex
```

---

## 8. Implementation Roadmap

### Phase 1: MVP (Week 1-2)

**Goal**: Basic working dashboard

**Tasks**:
- [x] **Multi-Project Infrastructure** _(v0.6.0)_
  - ~~Global project registry (`~/.intent-engine/projects.json`)~~ → WebSocket-based in-memory state
  - Project root auto-detection logic ✓
  - Port management system (fixed port 11391 with --port override) ✓
  - PID/lock file management per project ✓
- [ ] **Backend Server**
  - Setup Axum server with static file serving
  - Implement project context middleware
  - Database path resolution per project
- [ ] **Core REST API endpoints**:
  - GET /api/tasks
  - GET /api/tasks/:id
  - POST /api/tasks
  - POST /api/tasks/:id/start
  - POST /api/tasks/:id/done
- [ ] **CLI Commands**:
  - `ie dashboard start` (with auto-detection)
  - `ie dashboard stop`
  - `ie dashboard status`
  - `ie dashboard list`
- [ ] **Simple Frontend**:
  - Task list view (plain HTML)
  - Create task form
  - Task detail view
  - **Basic Markdown rendering** (marked.js + syntax highlighting)

**Deliverable**: Functional dashboard showing tasks, can create/start/complete, supports multiple projects

### Phase 2: Core Features (Week 3-4)

**Tasks**:
- [ ] **UI Components**
  - Task tree visualization (parent-child)
  - Event timeline view
  - Event creation UI
  - Task filtering (by status, priority)
  - Search functionality
  - Current task highlighting
  - Responsive layout (mobile-friendly)
- [ ] **Markdown Enhancement**
  - Tailwind Prose styling for rendered markdown
  - Code block syntax highlighting (highlight.js)
  - Support for tables, task lists, blockquotes
  - XSS protection with DOMPurify
  - Markdown caching for performance
- [ ] **Multi-Project UI**
  - Project switcher dropdown
  - Show current project name in header
  - `ie dashboard open` command

**Deliverable**: Full-featured dashboard with beautiful markdown rendering

### Phase 3: Real-time Updates (Week 5)

**Tasks**:
- [ ] **WebSocket Infrastructure**
  - WebSocket server implementation
  - Per-project WebSocket rooms
  - Broadcast events on task/event mutations
  - Frontend WebSocket client
  - Auto-reconnect logic
  - Fallback to polling if WS unavailable
- [ ] **Cross-Project Broadcasting** (optional)
  - Dashboard list page shows all running projects
  - Real-time status updates across projects

**Deliverable**: Real-time UI updates when changes occur

### Phase 4: Polish & Enhancements (Week 6+)

**Tasks**:
- [ ] **Advanced Features**
  - Statistics dashboard
  - Task dependency visualization
  - Batch operations (mark multiple as done)
  - Export functionality (JSON, Markdown)
  - Keyboard shortcuts
  - Task templates
  - Drag-and-drop reordering
- [ ] **Theme & Styling**
  - Dark mode toggle
  - Multiple syntax highlighting themes
  - Custom markdown CSS themes
  - Accessibility improvements (ARIA labels)
- [ ] **Multi-Project Enhancements**
  - Global dashboard (view all projects at once)
  - Cross-project task search
  - Project import/export
  - Project archiving

**Deliverable**: Production-ready dashboard with advanced features

---

## 9. User Scenarios

### Scenario 1: Web Creates Task → Claude Discovers

1. **Web Dashboard**: User creates task "Implement caching"
   - POST /api/tasks → Writes to DB
   - UI shows new task in list

2. **Claude Code**: User asks "What should I work on?"
   - AI: Calls `task_pick_next`
   - TaskManager queries DB, finds new task
   - AI: "I recommend task #50: Implement caching (created via Web)"

3. **Claude Code**: AI starts working
   - Calls `task_start(50)`
   - Database updated

4. **Web Dashboard** (if WebSocket enabled):
   - Receives `task.started` event
   - UI updates task status to "doing"

### Scenario 2: Session Start Overview

1. **User**: Starts Claude Code
2. **SessionStart Hook**: Runs
   ```bash
   ie task list --status todo --limit 5 --sort created_at desc
   ```
3. **Output**:
   ```
   Recent Tasks:
   ○ #52 Add unit tests (created 10m ago)
   ○ #51 Fix bug in parser (created 1h ago)
   ○ #50 Implement caching (created 2h ago)
   ```
4. **User**: Sees overview, decides which to work on

### Scenario 3: Urgent Task Mid-Session

1. **Web Dashboard**: User creates urgent task "Fix production bug"
   - Sets priority to Critical (p0)

2. **Claude Code**: Currently working on different task

3. **User**: Notices in Web, tells Claude:
   - "Check for any critical tasks"

4. **Claude**: Calls `task_list(status=todo, sort=priority)`
   - Finds critical task
   - "I see a critical task #53: Fix production bug. Should I switch to it?"

5. **User**: Confirms, Claude switches context

---

## 10. Security Considerations

### 10.1 Network Access

**Current Binding** (v0.6.8+): `0.0.0.0` (all network interfaces)

**Rationale**: Enables access from external devices (e.g., Windows host accessing WSL dashboard) for development convenience

⚠️ **Security Warning**: Dashboard is accessible from local network without authentication. Only use on trusted networks.

**Production Deployment**: If deploying on untrusted networks, use:
- Reverse proxy (nginx) with authentication
- VPN access
- SSH tunnel
- Firewall rules to restrict access

### 10.2 Optional Authentication

**Future Feature**: API token authentication

```toml
[security]
enable_auth = true
api_key = "randomly-generated-token"
```

**Headers**:
```
Authorization: Bearer <api_key>
```

### 10.3 CORS Configuration

**Development Mode**:
```toml
[cors]
enabled = true
allowed_origins = ["http://localhost:5173"]  # Vite dev server
```

**Production**: Disable CORS (same-origin policy)

---

## 11. Testing Strategy

### 11.1 Backend Testing

**Unit Tests** (Rust):
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_get_tasks_endpoint() {
        // Setup test DB
        // Make HTTP request
        // Assert response
    }
}
```

**Integration Tests**:
- Test all REST endpoints
- Test WebSocket events
- Test concurrent access to DB

### 11.2 Frontend Testing

**Manual Testing Checklist**:
- [ ] Create task with various fields
- [ ] Start/complete task
- [ ] Add events of each type
- [ ] Filter tasks by status
- [ ] Search functionality
- [ ] Responsive layout on mobile
- [ ] WebSocket reconnection

**Automated** (Future):
- Playwright/Cypress end-to-end tests

---

## 12. Deployment

### 12.1 Development Setup

```bash
# Terminal 1: Start dashboard
cd intent-engine
cargo run --bin ie dashboard start --websocket

# Terminal 2: Start frontend dev server (if using framework)
cd static
npm run dev

# Browser
open http://localhost:11391
```

### 12.2 Production Build

```bash
# Build Rust binary
cargo build --release --bin ie

# Build frontend (if using framework)
cd static && npm run build

# Copy static files to release
cp -r static/dist/* target/release/static/

# Run
./target/release/ie dashboard start --daemon
```

### 12.3 Auto-Start (Optional)

**systemd unit** (Linux):
```ini
[Unit]
Description=Intent-Engine Dashboard
After=network.target

[Service]
Type=simple
User=%i
ExecStart=/usr/local/bin/ie dashboard start --daemon
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

---

## 13. Future Enhancements

### 13.1 Collaboration Features
- Multi-user support
- Real-time collaborative editing
- Comments on tasks/events

### 13.2 Advanced Visualization
- Gantt chart timeline
- Dependency graph (D3.js)
- Burndown charts

### 13.3 Integrations
- GitHub Issues sync
- Slack notifications
- Calendar integration

### 13.4 AI Features
- Task auto-categorization
- Smart task suggestions
- Natural language task creation

---

## 14. Appendix

### 14.1 Example HTML Template (MVP)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Intent-Engine Dashboard</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-100">
    <div class="container mx-auto p-4">
        <h1 class="text-3xl font-bold mb-6">Intent-Engine Dashboard</h1>

        <!-- Task List -->
        <div id="task-list"
             hx-get="/api/tasks"
             hx-trigger="load"
             class="bg-white rounded-lg shadow p-4">
            Loading tasks...
        </div>

        <!-- Create Task Form -->
        <div class="mt-4 bg-white rounded-lg shadow p-4">
            <h2 class="text-xl font-semibold mb-3">Create Task</h2>
            <form hx-post="/api/tasks"
                  hx-target="#task-list"
                  hx-swap="outerHTML">
                <input type="text" name="name"
                       placeholder="Task name"
                       class="border p-2 w-full mb-2">
                <textarea name="spec"
                          placeholder="Specification (Markdown)"
                          class="border p-2 w-full mb-2"></textarea>
                <button type="submit"
                        class="bg-blue-500 text-white px-4 py-2 rounded">
                    Create
                </button>
            </form>
        </div>
    </div>
</body>
</html>
```

### 14.2 Example Rust Server Code

```rust
use axum::{
    routing::{get, post},
    Router,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Task {
    id: i64,
    name: String,
    spec: Option<String>,
    status: String,
    priority: i64,
}

async fn get_tasks() -> Json<Vec<Task>> {
    // Query from TaskManager
    Json(vec![])
}

async fn create_task(Json(payload): Json<CreateTaskRequest>) -> Json<Task> {
    // Use TaskManager to create
    Json(Task {
        id: 1,
        name: payload.name,
        spec: payload.spec,
        status: "todo".into(),
        priority: 0,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/tasks", get(get_tasks).post(create_task));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:11391")
        .await
        .unwrap();

    println!("Dashboard running on http://127.0.0.1:11391");
    axum::serve(listener, app).await.unwrap();
}
```

---

## 15. Questions & Decisions

### 15.1 Open Questions

1. **Frontend Framework**: HTMX vs React?
   - **Recommendation**: Start with HTMX for simplicity
   - Can migrate to React later if needed

2. **WebSocket**: Essential or optional?
   - **Recommendation**: Optional, implement in Phase 3
   - Most use cases work fine with polling

3. **Authentication**: Needed for local use?
   - **Recommendation**: Not for v1, add later if exposing to network

### 15.2 Decisions Made

1. ✅ No notification table - Use natural discovery
2. ✅ Database as single source of truth
3. ✅ Rust + Axum for backend (consistency with main project)
4. ✅ REST API primary, WebSocket optional
5. ✅ Local-only by default (127.0.0.1)

---

**End of Specification**
