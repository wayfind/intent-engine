# HTTP Shutdown Implementation Plan

**Goal**: Replace PID file mechanism with HTTP shutdown endpoint
**Version**: v0.11.0 (post v0.10.0 release)
**Complexity**: Medium-High (6-8 hours of work)

---

## 📋 Implementation Checklist

### Phase 1: Add HTTP Shutdown Endpoint ✅ (2-3 hours)

#### 1.1 Modify `src/dashboard/server.rs`
**Changes needed**:

```rust
use tokio::sync::oneshot;

pub struct DashboardServer {
    port: u16,
    db_path: PathBuf,
    project_name: String,
    project_path: PathBuf,
    shutdown_tx: Option<oneshot::Sender<()>>, // NEW: shutdown signal
}

impl DashboardServer {
    pub async fn run(self) -> Result<()> {
        // ... existing setup code ...

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // Store shutdown_tx in AppState
        let state = AppState {
            // ... existing fields ...
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(Some(shutdown_tx))),
        };

        // Build router with shutdown route
        let app = create_router(state);

        // Run server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .context("Server error")?;

        Ok(())
    }
}
```

#### 1.2 Update `AppState` structure
```rust
pub struct AppState {
    pub current_project: Arc<RwLock<ProjectContext>>,
    pub host_project: super::websocket::ProjectInfo,
    pub port: u16,
    pub ws_state: super::websocket::WebSocketState,
    pub shutdown_tx: Arc<tokio::sync::Mutex<Option<oneshot::Sender<()>>>>, // NEW
}
```

#### 1.3 Add shutdown handler in `src/dashboard/handlers.rs`
```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

/// Shutdown endpoint handler
/// POST /api/internal/shutdown
pub async fn shutdown_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Shutdown requested via HTTP");

    // Trigger shutdown signal
    let mut shutdown = state.shutdown_tx.lock().await;
    if let Some(tx) = shutdown.take() {
        if tx.send(()).is_ok() {
            tracing::info!("Shutdown signal sent successfully");
            Ok(Json(json!({
                "status": "ok",
                "message": "Dashboard is shutting down"
            })))
        } else {
            tracing::error!("Failed to send shutdown signal");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        tracing::warn!("Shutdown already initiated");
        Err(StatusCode::CONFLICT)
    }
}
```

#### 1.4 Add route in `src/dashboard/routes.rs`
```rust
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // ... existing routes ...

        // Internal routes (CLI → Dashboard communication)
        .route("/internal/cli-notify", post(handlers::handle_cli_notification))
        .route("/internal/shutdown", post(handlers::shutdown_handler)) // NEW
}
```

---

### Phase 2: Modify Stop Command 🔄 (1-2 hours)

#### 2.1 Update `src/cli_handlers/dashboard.rs` - Stop command
**Before**:
```rust
DashboardCommands::Stop { all } => {
    // Check PID file → kill process
    match pid::read_pid()? {
        Some(daemon_pid) => {
            pid::kill_process(daemon_pid)?;
            pid::delete_pid_file()?;
        }
        None => {
            println!("Dashboard not running");
        }
    }
}
```

**After**:
```rust
DashboardCommands::Stop { all } => {
    let port = 11391;

    // Check if Dashboard is running via HTTP health check
    if !check_dashboard_health(port).await {
        println!("Dashboard not running");
        return Ok(());
    }

    // Send shutdown request
    println!("Stopping Dashboard...");
    match send_shutdown_request(port).await {
        Ok(_) => {
            // Wait for shutdown to complete
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Verify shutdown
            if !check_dashboard_health(port).await {
                println!("✓ Dashboard stopped successfully");
            } else {
                eprintln!("⚠ Dashboard may still be running");
            }
        }
        Err(e) => {
            eprintln!("Failed to stop Dashboard: {}", e);
            eprintln!("\nManual stop instructions:");
            eprintln!("  Unix:    lsof -ti:{} | xargs kill", port);
            eprintln!("  Windows: netstat -ano | findstr :{}", port);
        }
    }

    Ok(())
}

async fn send_shutdown_request(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/api/internal/shutdown", port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| IntentError::OtherError(anyhow::anyhow!("Failed to send shutdown request: {}", e)))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(IntentError::OtherError(anyhow::anyhow!(
            "Shutdown request failed with status: {}",
            response.status()
        )))
    }
}
```

---

### Phase 3: Simplify Start Command 🧹 (1 hour)

#### 3.1 Remove PID checks from start command
**Remove**:
- Line 341-356: PID file check for daemon mode
- All calls to `pid::write_pid()`
- All calls to `pid::cleanup_stale_pid()`

**Keep only**:
- HTTP health check (Line 359-364)
- Port availability check (Line 367-372)

**Simplified start logic**:
```rust
DashboardCommands::Start { port, browser, daemon } => {
    let allocated_port = port.unwrap_or(11391);

    // Check if already running (HTTP health check)
    if check_dashboard_health(allocated_port).await {
        println!("Dashboard already running:");
        println!("  Port: {}", allocated_port);
        println!("  URL: http://127.0.0.1:{}", allocated_port);
        return Ok(());
    }

    // Check if port is available
    if std::net::TcpListener::bind(("0.0.0.0", allocated_port)).is_err() {
        return Err(IntentError::InvalidInput(format!(
            "Port {} is already in use by another program",
            allocated_port
        )));
    }

    // Start Dashboard (daemon or foreground)
    if daemon {
        start_daemon_mode(allocated_port, project_path, db_path, project_name, browser).await?;
    } else {
        start_foreground_mode(allocated_port, project_path, db_path, project_name, browser).await?;
    }

    Ok(())
}
```

---

### Phase 4: Delete PID Code 🗑️ (30 minutes)

#### 4.1 Delete files
```bash
rm src/dashboard/pid.rs
```

#### 4.2 Update `src/dashboard/mod.rs`
Remove:
```rust
pub mod pid;  // DELETE THIS LINE
```

#### 4.3 Remove PID imports from `src/cli_handlers/dashboard.rs`
Remove:
```rust
use crate::dashboard::pid;  // DELETE
```

#### 4.4 Remove all PID-related function calls
Search and remove:
- `pid::read_pid()`
- `pid::write_pid()`
- `pid::delete_pid_file()`
- `pid::cleanup_stale_pid()`
- `pid::is_process_running()`
- `pid::kill_process()`

---

### Phase 5: Update Tests 🧪 (1-2 hours)

#### 5.1 Delete PID tests
Tests to delete in `src/dashboard/pid.rs`:
- `test_write_and_read_pid`
- `test_cleanup_stale_pid`
- `test_read_nonexistent_pid`
- `test_delete_pid_file`
- `test_is_process_running`

#### 5.2 Add new HTTP shutdown tests
**In `tests/dashboard_integration_tests.rs`**:
```rust
#[tokio::test]
async fn test_shutdown_endpoint() {
    // Start Dashboard in background
    let handle = tokio::spawn(async {
        // ... start dashboard ...
    });

    // Send shutdown request
    let response = reqwest::post("http://127.0.0.1:11391/api/internal/shutdown")
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Wait for graceful shutdown
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify Dashboard is stopped
    assert!(!check_dashboard_health(11391).await);
}
```

#### 5.3 Update existing Dashboard tests
Update any tests that relied on PID file checks to use HTTP health checks instead.

---

### Phase 6: Documentation Updates 📚 (30 minutes)

#### 6.1 Update MIGRATION_v0.10.0.md
Add section explaining the change from PID files to HTTP shutdown:

```markdown
### Removed: PID File Management

**Before (v0.9.x)**:
- Dashboard tracked via `~/.intent-engine/dashboard.pid`
- Stop command read PID and killed process

**After (v0.11.0)**:
- Dashboard uses HTTP health checks (port-based)
- Stop command sends `POST /api/internal/shutdown`
- Simpler, no file system dependencies
- No race conditions in tests
```

#### 6.2 Update README.md
Update Dashboard management examples to reflect new behavior.

---

## 🧪 Testing Strategy

### Manual Testing Steps

1. **Start Dashboard (foreground)**
   ```bash
   ie dashboard start
   # Should start without PID file
   ```

2. **Start Dashboard (daemon)**
   ```bash
   ie dashboard start --daemon
   # Should start in background
   ```

3. **Stop Dashboard**
   ```bash
   ie dashboard stop
   # Should send HTTP request and gracefully shutdown
   ```

4. **Check Status**
   ```bash
   ie dashboard status
   # Should use HTTP health check only
   ```

5. **Test edge cases**
   - Stop when not running
   - Start when already running
   - Port already in use by another program

### Automated Testing
```bash
# Run all tests
cargo test --all

# Specific Dashboard tests
cargo test dashboard

# Integration tests
cargo test --test dashboard_integration_tests
```

---

## 📊 Risk Assessment

### Low Risk
- ✅ HTTP shutdown is standard practice
- ✅ Simpler architecture (less code = fewer bugs)
- ✅ No file system dependencies
- ✅ No race conditions

### Medium Risk
- ⚠️ Requires careful testing of graceful shutdown
- ⚠️ Network errors need proper handling
- ⚠️ Backward compatibility concerns (users upgrading from v0.10.0)

### Mitigation
- Comprehensive testing (manual + automated)
- Clear migration guide
- Fallback to manual stop instructions if HTTP fails
- Version bump to 0.11.0 (indicating feature change)

---

## 🚀 Rollout Plan

### Option A: Include in v0.10.0 (Not Recommended)
- ❌ Too risky for current release
- ❌ Requires extensive testing
- ❌ Delays v0.10.0 release

### Option B: Separate v0.11.0 Release (Recommended)
1. **Complete v0.10.0 release first**
   - Fix current blockers
   - Keep PID mechanism (with documented workarounds)
   - Release as stable

2. **Develop v0.11.0 in separate branch**
   - Implement HTTP shutdown
   - Thorough testing
   - Update documentation

3. **Release v0.11.0 after stabilization**
   - Clear migration guide
   - Breaking change notice
   - Community feedback period

---

## 📦 Dependencies to Add

```toml
# Cargo.toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }  # For stop command HTTP client
tokio = { version = "1", features = ["sync"] }      # For oneshot channel (already included)
```

---

## ⏱️ Time Estimate

| Phase | Estimated Time | Risk Level |
|-------|----------------|------------|
| Phase 1: HTTP Endpoint | 2-3 hours | Low |
| Phase 2: Stop Command | 1-2 hours | Medium |
| Phase 3: Start Command | 1 hour | Low |
| Phase 4: Delete PID Code | 30 min | Low |
| Phase 5: Update Tests | 1-2 hours | Medium |
| Phase 6: Documentation | 30 min | Low |
| **Total** | **6-9 hours** | **Medium** |

---

## 🎯 Recommendation

**For v0.10.0 (current release)**:
- ✅ Keep PID mechanism
- ✅ Fix test race condition with serial execution
- ✅ Document issue in KNOWN_ISSUES.md
- ✅ Plan HTTP shutdown for v0.11.0

**For v0.11.0 (next release)**:
- ✅ Implement HTTP shutdown properly
- ✅ Extensive testing
- ✅ Clean architecture

**Reason**: The current PID mechanism works (with workaround), and rushing this change could introduce new bugs right before release. Better to do it properly in the next version.

---

## 💬 Discussion

**Question for user**: Do you want to:
1. **Proceed with full implementation now** (6-9 hours of work, delays v0.10.0)
2. **Release v0.10.0 first, then implement in v0.11.0** (safer, more time for testing)
3. **Implement a minimal version now** (just shutdown endpoint, keep some PID code as fallback)

What's your preference?
