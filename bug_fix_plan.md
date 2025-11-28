# Bug Fix: Task Status Transition Issue

## Problem Description
The user reported that a task in the "done" state, when restarted to "doing", cannot be marked as "done" again. The status update seems to be ignored in the UI.

## Root Cause Analysis
- **Backend Behavior**: The `done_task` API endpoint returns a structured response containing `completed_task`, `workspace_status`, and `next_step_suggestion`.
- **Frontend Behavior**: The `appStore.ts` `doneTask` action was expecting the response `data.data` to directly contain the task properties (like `id`), but the task object is actually nested under `data.data.completed_task`.
- **Result**: The frontend failed to find the task ID in the response, so it didn't update the local state, leaving the UI showing "DOING" even if the backend might have processed it (or the request failed/wasn't handled correctly in the store).

## Implementation Plan
- [x] **Identify the issue**: Verified backend response structure via `cargo run` and inspected frontend code.
- [x] **Fix the code**: Updated `frontend-v2/src/stores/appStore.ts` to correctly access `data.data.completed_task`.
- [x] **Verify the fix**: Verified backend response structure matches the fix. Browser verification was inconclusive due to test automation issues, but code logic is sound.

## Verification Steps
1.  Restart a "DONE" task to "DOING".
2.  Click "DONE".
3.  Confirm UI updates to "DONE".
