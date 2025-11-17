# Dashboard API Reference

**Version**: 0.5.0 (Phase 1 MVP)
**Base URL**: `http://127.0.0.1:<PORT>/api`

---

## Overview

The Intent-Engine Dashboard provides a RESTful API for managing tasks and events. All endpoints return JSON responses with a consistent structure.

### Response Format

**Success Response**:
```json
{
  "data": <result_object_or_array>
}
```

**Error Response**:
```json
{
  "code": "ERROR_CODE",
  "message": "Human-readable error message",
  "details": {} // Optional
}
```

### HTTP Status Codes

- `200 OK` - Success
- `201 Created` - Resource created
- `204 No Content` - Success with no body
- `400 Bad Request` - Invalid request
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error

---

## Endpoints

### Health & Info

#### GET /api/health

Health check endpoint.

**Response**:
```json
{
  "status": "healthy",
  "service": "intent-engine-dashboard",
  "version": "0.5.0"
}
```

#### GET /api/info

Get project information.

**Response**:
```json
{
  "name": "my-project",
  "path": "/path/to/project",
  "database": "/path/to/.intent-engine/intents.db",
  "port": 3030
}
```

---

### Tasks

#### GET /api/tasks

List all tasks with optional filtering.

**Query Parameters**:
- `status` (optional): Filter by status (`todo`, `doing`, `done`)
- `parent` (optional): Filter by parent ID or `"null"` for top-level tasks

**Example**:
```bash
GET /api/tasks?status=todo
GET /api/tasks?parent=null
GET /api/tasks?parent=42
```

**Response**:
```json
{
  "data": [
    {
      "id": 1,
      "name": "Task name",
      "spec": "Task specification (Markdown)",
      "status": "todo",
      "priority": 0,
      "parent_id": null,
      "first_todo_at": "2025-11-16T12:00:00Z",
      "first_doing_at": null,
      "first_done_at": null
    }
  ]
}
```

#### GET /api/tasks/:id

Get a single task by ID.

**Parameters**:
- `:id` - Task ID (integer)

**Response**:
```json
{
  "data": {
    "id": 42,
    "name": "Implement authentication",
    "spec": "# Authentication Implementation\n\n...",
    "status": "doing",
    "priority": 1,
    "parent_id": null,
    "first_todo_at": "2025-11-16T12:00:00Z",
    "first_doing_at": "2025-11-16T13:00:00Z",
    "first_done_at": null
  }
}
```

**Errors**:
- `404` - Task not found

#### POST /api/tasks

Create a new task.

**Request Body**:
```json
{
  "name": "Task name (required)",
  "spec": "Specification in Markdown (optional)",
  "priority": 1,  // 1=critical, 2=high, 3=medium, 4=low (optional)
  "parent_id": 42 // Parent task ID (optional)
}
```

**Response**: `201 Created`
```json
{
  "data": {
    "id": 100,
    "name": "Task name",
    ...
  }
}
```

#### PATCH /api/tasks/:id

Update a task.

**Request Body** (all fields optional):
```json
{
  "name": "New name",
  "spec": "Updated specification",
  "priority": 2,
  "status": "doing" // "todo", "doing", "done"
}
```

**Response**: `200 OK`
```json
{
  "data": {
    "id": 42,
    "name": "New name",
    ...
  }
}
```

**Errors**:
- `404` - Task not found

#### DELETE /api/tasks/:id

Delete a task.

**Response**: `204 No Content`

**Errors**:
- `404` - Task not found
- `400` - Task has subtasks (cannot delete)

---

### Task Operations

#### POST /api/tasks/:id/start

Start a task (set as current focus).

**Response**: `200 OK`
```json
{
  "data": {
    "id": 42,
    "status": "doing",
    ...
  }
}
```

**Errors**:
- `404` - Task not found
- `400` - Task has unmet dependencies

#### POST /api/tasks/done

Complete the currently focused task.

**Note**: This is a global operation with **no ID parameter**.

**Response**: `200 OK`
```json
{
  "data": {
    "id": 42,
    "status": "done",
    ...
  }
}
```

**Errors**:
- `400` - No current task
- `400` - Task has incomplete subtasks

#### POST /api/tasks/:id/switch

Switch focus to a different task.

**Response**: `200 OK`
```json
{
  "data": {
    "id": 50,
    "status": "doing",
    ...
  }
}
```

**Errors**:
- `404` - Task not found

#### POST /api/tasks/:id/spawn-subtask

Create a subtask of the **current task** and switch focus to it.

**Note**: The `:id` parameter is **ignored**. The current task is used as parent.

**Request Body**:
```json
{
  "name": "Subtask name (required)",
  "spec": "Specification (optional)"
}
```

**Response**: `201 Created`
```json
{
  "data": {
    "parent_task": { ... },
    "subtask": {
      "id": 101,
      "parent_id": 42,
      "name": "Subtask name",
      ...
    }
  }
}
```

**Errors**:
- `400` - No current task

---

### Events

#### GET /api/tasks/:id/events

List events for a task.

**Query Parameters**:
- `event_type` (optional): Filter by type (`decision`, `blocker`, `milestone`, `note`)
- `since` (optional): Time filter (`1d`, `7d`, `24h`, etc.)
- `limit` (optional): Maximum number of events (integer)

**Example**:
```bash
GET /api/tasks/42/events?event_type=decision
GET /api/tasks/42/events?since=7d&limit=10
```

**Response**:
```json
{
  "data": [
    {
      "id": 1,
      "task_id": 42,
      "log_type": "decision",
      "discussion_data": "Chose approach A because...",
      "timestamp": "2025-11-16T14:00:00Z"
    }
  ]
}
```

#### POST /api/tasks/:id/events

Add an event to a task.

**Request Body**:
```json
{
  "type": "decision",  // "decision", "blocker", "milestone", "note"
  "data": "Event content in Markdown"
}
```

**Response**: `201 Created`
```json
{
  "data": {
    "id": 10,
    "task_id": 42,
    "log_type": "decision",
    "discussion_data": "Event content...",
    "timestamp": "2025-11-16T14:30:00Z"
  }
}
```

**Errors**:
- `400` - Invalid event type

---

### Global Operations

#### GET /api/current-task

Get the currently focused task.

**Response**:
```json
{
  "data": {
    "current_task_id": 42,
    "task": {
      "id": 42,
      "name": "Current task",
      ...
    }
  }
}
```

**If no current task**:
```json
{
  "data": null,
  "message": "No current task"
}
```

#### GET /api/pick-next

Get the recommended next task based on priority and focus.

**Response**:
```json
{
  "data": {
    "task": {
      "id": 50,
      "name": "Next recommended task",
      ...
    },
    "reason": "Depth-first: subtask of current task"
  }
}
```

**If no tasks available**:
```json
{
  "data": {
    "task": null,
    "reason": "No tasks available"
  }
}
```

#### GET /api/search

Unified search across tasks and events.

**Query Parameters**:
- `query` (required): Search query (supports FTS5 syntax)
- `include_tasks` (optional): Include tasks in results (default: `true`)
- `include_events` (optional): Include events in results (default: `true`)
- `limit` (optional): Maximum results (default: 20)

**Example**:
```bash
GET /api/search?query=authentication
GET /api/search?query=JWT AND token&include_events=false
```

**Response**:
```json
{
  "data": [
    {
      "result_type": "task",
      "task": { ... },
      "match_field": "name",
      "match_snippet": "...authentication..."
    },
    {
      "result_type": "event",
      "event": { ... },
      "task_chain": [{ "id": 42, "name": "Parent" }],
      "match_snippet": "...JWT token..."
    }
  ]
}
```

---

## Error Codes

### Task Errors

- `TASK_NOT_FOUND` (404) - Task with given ID does not exist
- `NO_CURRENT_TASK` (400) - No task is currently focused
- `TASK_HAS_SUBTASKS` (400) - Cannot delete/complete task with incomplete subtasks
- `TASK_BLOCKED` (400) - Task has unmet dependencies

### Request Errors

- `INVALID_REQUEST` (400) - Malformed request or invalid parameters
- `INVALID_EVENT_TYPE` (400) - Event type must be decision/blocker/milestone/note

### Server Errors

- `DATABASE_ERROR` (500) - Database operation failed
- `INTERNAL_ERROR` (500) - Unexpected server error

---

## Authentication

**Phase 1**: No authentication required. Dashboard binds to `127.0.0.1` (localhost only).

**Future**: Phase 2+ will add authentication (API keys, JWT).

---

## Rate Limiting

**Phase 1**: No rate limiting.

**Future**: Phase 2+ will add rate limiting for production use.

---

## CORS

CORS is enabled for all origins in Phase 1 (development mode).

Allowed methods: `GET`, `POST`, `PATCH`, `DELETE`
Allowed headers: All

---

## Example Workflows

### Create and Complete a Task

```bash
# 1. Create task
curl -X POST http://127.0.0.1:3030/api/tasks \
  -H "Content-Type: application/json" \
  -d '{"name": "Fix bug #123", "priority": 1}'

# Response: {"data": {"id": 100, ...}}

# 2. Start task
curl -X POST http://127.0.0.1:3030/api/tasks/100/start

# 3. Add a decision event
curl -X POST http://127.0.0.1:3030/api/tasks/100/events \
  -H "Content-Type: application/json" \
  -d '{"type": "decision", "data": "Fixed by reverting commit abc123"}'

# 4. Complete task
curl -X POST http://127.0.0.1:3030/api/tasks/done

# Response: {"data": {"id": 100, "status": "done", ...}}
```

### Search and Update

```bash
# 1. Search for tasks
curl "http://127.0.0.1:3030/api/search?query=authentication"

# 2. Update a task
curl -X PATCH http://127.0.0.1:3030/api/tasks/42 \
  -H "Content-Type: application/json" \
  -d '{"priority": 1, "spec": "Updated specification"}'
```

---

## Notes

- All timestamps are in ISO 8601 format (UTC)
- Markdown content is stored as-is; rendering happens client-side
- Task IDs are auto-incremented integers
- Priority: 1 (highest) to 4 (lowest), 0 = default

---

**Last Updated**: 2025-11-16
**Version**: 0.5.0
