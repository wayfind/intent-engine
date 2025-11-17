# Dashboard UI Regression Test Suite

**Version**: 1.0
**Last Updated**: 2025-11-16
**Purpose**: Complete UI testing checklist for Intent-Engine Dashboard
**Tool**: Playwright Browser Automation

---

## Prerequisites

1. Dashboard server running on port 3060: `ie dashboard start --port 3060`
2. Database initialized with test data (at least 10-20 tasks)
3. Playwright browser installed (run tests with MCP Playwright tools)

---

## Test Suite Overview

This test suite covers all major Dashboard features and should be executed:
- ✅ Before each release
- ✅ After any UI-related code changes
- ✅ After backend API changes that affect the frontend
- ✅ When debugging UI-related issues

**Total Test Cases**: 8 groups, ~25 individual checks

---

## 🧪 Test Case 1: Page Load and Basic Layout

**Objective**: Verify Dashboard loads correctly with proper layout

### Steps:
1. Navigate to `http://127.0.0.1:3060`
2. Wait for page to fully load

### Expected Results:
- ✅ Page loads without errors (HTTP 200)
- ✅ Page title is "Intent-Engine Dashboard"
- ✅ Header displays "Intent-Engine Dashboard"
- ✅ Project name displays correctly in header
- ✅ Three main sections visible:
  - Left sidebar (task list)
  - Main content area (task details)
  - Right sidebar (event history)
- ✅ Header buttons visible: "+ New Task", "Current Focus", "Pick Next"
- ✅ Search input visible in left sidebar
- ✅ Filter buttons visible: All, Todo, Doing, Done

### Known Issues:
- ⚠️ **P2**: favicon.ico returns 404 (cosmetic only, doesn't affect functionality)

### Playwright Example:
```javascript
await browser_navigate({url: "http://127.0.0.1:3060"});
await browser_snapshot(); // Verify layout
```

---

## 🧪 Test Case 2: Task List Display

**Objective**: Verify task list loads and displays tasks correctly

### Steps:
1. Ensure page is loaded (from Test Case 1)
2. Observe left sidebar task list

### Expected Results:
- ✅ Task cards display in left sidebar
- ✅ Each task card shows:
  - Task name
  - Status badge (todo/doing/done) with correct color
  - Priority badge (if set)
  - Parent indicator (if subtask)
- ✅ Status badge colors:
  - `todo`: Yellow background (#fef3c7), brown text
  - `doing`: Blue background (#dbeafe), dark blue text
  - `done`: Green background (#d1fae5), dark green text
- ✅ Task cards are clickable
- ✅ Active task (if any) has blue highlight

### Playwright Example:
```javascript
const snapshot = await browser_snapshot();
// Verify task cards exist in snapshot
```

---

## 🧪 Test Case 3: Task Filtering

**Objective**: Verify filter buttons work correctly

### Steps:
1. Note total task count (e.g., 81 tasks)
2. Click "Todo" filter button
3. Observe filtered tasks
4. Click "Doing" filter button
5. Observe filtered tasks
6. Click "Done" filter button
7. Observe filtered tasks
8. Click "All" filter button
9. Verify all tasks are back

### Expected Results:
- ✅ "Todo" filter: Shows only tasks with status=todo
- ✅ "Doing" filter: Shows only tasks with status=doing
- ✅ "Done" filter: Shows only tasks with status=done
- ✅ "All" filter: Shows all tasks regardless of status
- ✅ Active filter button has indigo background (#f0f9ff)
- ✅ Inactive filter buttons have gray background
- ✅ Task count updates correctly for each filter

### Playwright Example:
```javascript
await browser_click({element: "Todo filter button", ref: "#filter-todo"});
await browser_snapshot(); // Verify only todo tasks shown

await browser_click({element: "All filter button", ref: "#filter-all"});
await browser_snapshot(); // Verify all tasks shown
```

---

## 🧪 Test Case 4: Task Details View

**Objective**: Verify clicking a task displays its details correctly

### Steps:
1. Click on a task card in the left sidebar
2. Observe main content area

### Expected Results:
- ✅ Task details display in main content area
- ✅ Task name shown as heading
- ✅ Status and priority badges visible
- ✅ Spec content rendered as Markdown with:
  - Headers (#, ##, ###)
  - Code blocks with syntax highlighting
  - Lists (bullets, numbered)
  - Links
  - Blockquotes
- ✅ XSS protection active (DOMPurify sanitizes HTML)
- ✅ Action buttons visible based on status:
  - "Start Task" (if status=todo)
  - "Complete Task" (if status=doing)
  - "Spawn Subtask" (if status=doing)
  - "Delete Task" (always)
- ✅ Timestamps shown (created_at, updated_at)
- ✅ Parent task info shown (if subtask)
- ✅ Subtask count shown (if has children)

### Playwright Example:
```javascript
// Click first task in list
await browser_click({element: "First task card", ref: ".task-card:first-child"});
await browser_snapshot(); // Verify task details displayed
```

---

## 🧪 Test Case 5: Event History

**Objective**: Verify event history displays correctly

### Steps:
1. Click on a task that has events
2. Observe right sidebar (event history panel)

### Expected Results:
- ✅ Event history panel shows events for selected task
- ✅ Events sorted by timestamp (newest first)
- ✅ Each event shows:
  - Event type badge (decision/blocker/milestone/note)
  - Timestamp (relative time)
  - Event data rendered as Markdown
- ✅ Event type badge colors:
  - `decision`: Blue
  - `blocker`: Red
  - `milestone`: Green
  - `note`: Gray
- ✅ "Add Event" button visible
- ✅ If no events: "Select a task to view its event history"

### Playwright Example:
```javascript
await browser_click({element: "Task with events", ref: ".task-card:nth-child(2)"});
// Verify event history in snapshot
```

---

## 🧪 Test Case 6: Task Operations

**Objective**: Verify task operations work correctly

### Test 6A: Start Task
1. Filter to show "Todo" tasks
2. Click on a todo task
3. Click "Start Task" button
4. Verify task status changes to "doing"
5. Verify task appears in "Current Focus"

### Test 6B: Complete Task
1. Ensure a task is in "doing" status (from Test 6A)
2. Click "Complete Task" button
3. Verify task status changes to "done"
4. Verify "Current Focus" is cleared

### Test 6C: Spawn Subtask
1. Start a task (status=doing)
2. Click "Spawn Subtask" button
3. Fill in modal form:
   - Name: "Test Subtask"
   - Spec: "Test specification"
4. Click "Create Subtask"
5. Verify subtask is created
6. Verify subtask is now current task
7. Verify parent task shows subtask count

### Test 6D: Delete Task
1. Click on a task (preferably a test task)
2. Click "Delete Task" button
3. Confirm deletion in browser dialog
4. Verify task is removed from list

### Expected Results:
- ✅ All operations execute without errors
- ✅ UI updates immediately after operation
- ✅ Status badges update correctly
- ✅ Current task indicator updates
- ✅ Event history is created for state changes

### Playwright Example:
```javascript
// Start task
await browser_click({element: "Start Task button", ref: "button:has-text('Start Task')"});
await browser_snapshot(); // Verify status=doing

// Complete task
await browser_click({element: "Complete Task button", ref: "button:has-text('Complete Task')"});
await browser_snapshot(); // Verify status=done
```

---

## 🧪 Test Case 7: Search Functionality ⭐

**Objective**: Verify search filters tasks correctly

### Steps:
1. Note initial task count (e.g., 81 tasks)
2. Click in search input box
3. Type "Dashboard" (slowly, one character at a time)
4. Observe task list updates
5. Clear search input
6. Verify all tasks return

### Expected Results:
- ✅ Search input accepts text
- ✅ Task list filters as you type (debounced ~300ms)
- ✅ Only matching tasks shown (searches task name + spec + events)
- ✅ Search result count < initial count
- ✅ Search highlights are shown (if implemented)
- ✅ Clearing search restores full task list
- ✅ No JavaScript errors in console
- ✅ Search status message shown ("Searching...", "X results found")

### Known Fixes:
- ✅ **v1.0**: Changed from `onkeyup` to `oninput` event (more reliable)
- ✅ **v1.0**: Enhanced error handling in handleSearch function

### Playwright Example:
```javascript
// Type search query
await browser_type({
    element: "Search input",
    ref: "#search-input",
    text: "Dashboard",
    slowly: true
});

// Wait for debounce
await browser_wait_for({time: 0.5});

// Verify filtered results
const snapshot = await browser_snapshot();
// Expect fewer tasks shown

// Clear search
await browser_click({element: "Search input", ref: "#search-input"});
await browser_evaluate({
    function: "() => document.getElementById('search-input').value = ''"
});
await browser_type({element: "Search input", ref: "#search-input", text: " ", submit: false});
```

---

## 🧪 Test Case 8: Header Button Functions

**Objective**: Verify header buttons work correctly

### Test 8A: "New Task" Modal
1. Click "+ New Task" button
2. Verify modal appears
3. Fill in form:
   - Name: "Test Task from UI"
   - Spec: "Test specification"
   - Priority: "High"
4. Click "Create Task"
5. Verify modal closes
6. Verify new task appears in list

### Test 8B: "Current Focus" Button
1. Ensure a task is focused (status=doing)
2. Click "Current Focus" button
3. Verify current task is displayed in main area
4. Verify task is highlighted in left sidebar

### Test 8C: "Pick Next" Button
1. Ensure there are pending tasks
2. Click "Pick Next" button
3. Verify recommendation modal/message appears
4. Verify recommended task is shown with reason

### Expected Results:
- ✅ Modals open and close correctly
- ✅ Form validation works (required fields)
- ✅ New task creation succeeds
- ✅ Current focus navigation works
- ✅ Pick next recommendation is sensible (depth-first strategy)

### Playwright Example:
```javascript
// Open new task modal
await browser_click({element: "New Task button", ref: "button:has-text('New Task')"});
await browser_snapshot(); // Verify modal visible

// Fill form
await browser_fill_form({
    fields: [
        {name: "Task name", type: "textbox", ref: "input[name='name']", value: "Test Task"},
        {name: "Spec", type: "textbox", ref: "textarea[name='spec']", value: "Test spec"}
    ]
});

// Submit
await browser_click({element: "Create button", ref: "button[type='submit']"});
```

---

## 📋 Quick Checklist (TL;DR)

Use this checklist for rapid manual verification:

- [ ] Page loads without errors
- [ ] Layout shows 3 sections (task list, details, events)
- [ ] Task list displays tasks
- [ ] Filter buttons work (All/Todo/Doing/Done)
- [ ] Clicking task shows details
- [ ] Markdown renders correctly
- [ ] Event history displays
- [ ] Start task works
- [ ] Complete task works
- [ ] Spawn subtask works
- [ ] Delete task works
- [ ] Search filters tasks correctly ⭐
- [ ] Clear search restores full list
- [ ] New task modal works
- [ ] Current focus button works
- [ ] Pick next button works
- [ ] No console errors (except favicon 404)

---

## 🐛 Known Issues

| ID | Priority | Description | Status | Workaround |
|----|----------|-------------|--------|------------|
| UI-001 | P2 | favicon.ico returns 404 | Open | None needed (cosmetic) |
| UI-002 | P0 | Search onkeyup not reliable | **Fixed v1.0** | Changed to oninput |
| UI-003 | P0 | Search error handling missing | **Fixed v1.0** | Added try/catch + validation |

---

## 🔧 Running Tests with Playwright

### Automated Test (Full Suite)

```javascript
// Test 1: Page Load
await browser_navigate({url: "http://127.0.0.1:3060"});
const layout = await browser_snapshot();
// Verify: "Intent-Engine Dashboard" in layout

// Test 2: Task List
// Verify: task cards exist in snapshot

// Test 3: Filtering
await browser_click({element: "Todo filter", ref: "#filter-todo"});
await browser_snapshot();
await browser_click({element: "All filter", ref: "#filter-all"});

// Test 4: Task Details
await browser_click({element: "First task", ref: ".task-card:first-child"});
await browser_snapshot();
// Verify: task name, spec, status badge visible

// Test 5: Events
// Verify: event list in right sidebar

// Test 6: Operations
await browser_click({element: "Start button", ref: "button:has-text('Start Task')"});
await browser_snapshot();
// Verify: status changed to "doing"

// Test 7: Search ⭐
await browser_type({element: "Search", ref: "#search-input", text: "Dashboard", slowly: true});
await browser_wait_for({time: 0.5});
const searchResults = await browser_snapshot();
// Verify: fewer tasks shown

// Clear search
await browser_evaluate({
    function: "() => { document.getElementById('search-input').value = ''; document.getElementById('search-input').dispatchEvent(new Event('input')); }"
});
await browser_wait_for({time: 0.5});
const allTasks = await browser_snapshot();
// Verify: all tasks restored

// Test 8: Header Buttons
await browser_click({element: "Pick Next", ref: "button:has-text('Pick Next')"});
await browser_snapshot();
// Verify: recommendation shown
```

### Console Error Check

```javascript
const messages = await browser_console_messages({onlyErrors: true});
// Should only see favicon 404, no other errors
```

---

## 📝 Maintenance Notes

**When to update this test suite:**
1. New UI features added → Add new test cases
2. UI bugs fixed → Update "Known Issues" section
3. API changes affecting frontend → Update test expectations
4. Major refactoring → Re-verify all test cases

**Test data requirements:**
- Minimum 20 tasks with varied statuses (todo/doing/done)
- At least 3 tasks with events (decisions, blockers, notes)
- At least 2 parent-child task relationships
- Tasks with different priorities
- Tasks with Markdown specs (headers, code blocks, lists)

---

**End of Test Suite**
For questions or issues, see `docs/dashboard-user-guide.md`
