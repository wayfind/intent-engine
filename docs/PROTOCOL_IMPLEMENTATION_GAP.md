# Intent-Engine Protocol: Implementation Gap Analysis

**Date**: 2025-11-23
**Protocol Version**: 1.0
**Analysis Type**: Ultrathink Deep Dive

---

## Executive Summary

本文档对比 **Intent-Engine Protocol v1.0 规范**（理想状态）与**当前实现**（实际代码）之间的差异，识别已实现的功能、缺失的功能、以及偏差点。

### 总体评估

| 维度 | 评分 | 说明 |
|-----|------|------|
| **核心功能完整性** | 🟢 85% | 大部分核心功能已实现 |
| **协议合规性** | 🟡 70% | 基本合规，但有部分偏差 |
| **文档一致性** | 🟡 65% | 实现与规范部分不一致 |
| **生产就绪度** | 🟡 75% | 可用，但需补充功能 |

---

## 1. 消息格式对比

### 1.1 ✅ 协议包装器 (Protocol Wrapper)

**规范要求**:
```json
{
  "version": "1.0",
  "type": "message_type",
  "payload": { ... },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**实际实现**:
- ✅ **Dashboard (websocket.rs:22-52)**: 完全合规
  ```rust
  pub struct ProtocolMessage<T> {
      pub version: String,
      #[serde(rename = "type")]
      pub message_type: String,
      pub payload: T,
      pub timestamp: String,
  }
  ```

- ✅ **MCP Client (ws_client.rs:14-37)**: 完全合规
- ✅ **Web UI (app.js:318-325)**: 完全合规，带版本验证

**结论**: ✅ **已实现，完全合规**

---

## 2. 连接层消息类型

### 2.1 ✅ `hello` / `welcome`

**规范定义**:
- `hello`: Client → Server，建立连接后立即发送
- `welcome`: Server → Client，确认连接

**实际实现**:
- ✅ **数据结构存在** (websocket.rs:236-252)
  ```rust
  pub struct HelloPayload { entity_type, capabilities }
  pub struct WelcomePayload { capabilities, session_id }
  ```

- ❌ **未在实际连接流程中使用**
  - MCP Client 直接发送 `register`，跳过 `hello`
  - Web UI 未发送 `hello`
  - Dashboard 未发送 `welcome`

**差距**: 🔴 **规范定义但未实际使用**

---

### 2.2 ✅ `ping` / `pong`

**规范要求**:
- 每 30 秒心跳
- 90 秒超时（3 次未响应）

**实际实现**:

**Dashboard → MCP** (websocket.rs:311-323):
- ✅ 每 30 秒发送 `pong` 作为心跳
- ⚠️ 使用 `pong` 而非 `ping`（命名差异）

**Dashboard → Web UI** (websocket.rs:522-534):
- ✅ 每 30 秒发送 `ping`

**Web UI → Dashboard** (app.js:349-361):
- ✅ 接收 `ping`，响应 `pong`
- ✅ 90 秒心跳超时检测 (app.js:256-261)

**MCP Client**:
- ❌ 未实现 `ping`/`pong` 处理
- ⚠️ 仅依赖 WebSocket 层面的连接检测

**差距**: 🟡 **部分实现，命名不一致，MCP Client 缺失**

---

### 2.3 ⚠️ `goodbye`

**规范要求**: 优雅关闭通知

**实际实现**:
- ✅ **数据结构存在** (websocket.rs:254-260, ws_client.rs:49-54)
  ```rust
  pub struct GoodbyePayload { reason: Option<String> }
  ```

- ❌ **未在实际断开流程中使用**
  - 当前实现直接关闭 WebSocket，未发送 `goodbye`

**差距**: 🔴 **规范定义但未实际使用**

---

## 3. 注册层消息类型 (MCP Server Only)

### 3.1 ✅ `register` / `registered`

**规范要求**: MCP Server 向 Dashboard 注册项目

**实际实现**:

**MCP Client 发送 `register`** (ws_client.rs:146-156):
```rust
let register_msg = ProtocolMessage::new("register", project_info);
```
✅ **完全合规**

**Dashboard 处理 `register`** (websocket.rs:340-391):
- ✅ 解析 `ProjectInfo` payload
- ✅ 防御性检查（temp 目录）
- ✅ 存储到 `mcp_connections`
- ✅ 广播 `project_online` 到所有 Web UI
- ✅ 响应 `registered`

**MCP Client 接收 `registered`** (ws_client.rs:182-192):
```rust
"registered" => {
    tracing::info!("✓ Successfully registered with Dashboard");
}
```
✅ **完全合规**

**差距**: ✅ **已实现，完全合规**

---

### 3.2 🔴 路径验证规则

**规范要求** (Section 4.2.1):
- `path` 必须是绝对路径
- `path` 必须**不在**临时目录（防御层）

**实际实现**:

**MCP Client** (ws_client.rs:77-91):
```rust
let normalized_project_path = project_path.canonicalize()?;
let temp_dir = std::env::temp_dir().canonicalize()?;

if normalized_project_path.starts_with(&temp_dir) {
    tracing::warn!("Skipping Dashboard registration for temporary path");
    return Ok(()); // ❌ 静默跳过，不发送 register
}
```

**Dashboard** (websocket.rs:355-363):
```rust
let temp_dir = std::env::temp_dir().canonicalize()?;
if project_path.starts_with(&temp_dir) {
    tracing::warn!("Rejecting registration from temp directory");
    // ❌ 日志警告但继续处理，未拒绝注册
    // ❌ 未发送 registered{success: false} 响应
}
```

**差距**: 🔴 **验证逻辑存在，但响应不规范**
- Dashboard 应发送 `registered` with `success: false` + `error` 字段
- 实际只打印日志，未阻止注册

---

## 4. 状态同步层消息类型

### 4.1 ✅ `init`

**规范要求**: Dashboard → Web UI，连接后发送初始状态

**实际实现**:

**Dashboard 发送 `init`** (websocket.rs:493-519):
```rust
async fn handle_ui_socket(...) {
    // Send initial project list
    let projects_info = state.get_online_projects_with_current(...).await;
    send_protocol_message(&tx, "init", InitPayload { projects: projects_info })?;
}
```
✅ **完全合规**

**Web UI 接收 `init`** (app.js:337-340):
```javascript
case 'init':
    handleInitMessage(message.payload.projects);
```
✅ **完全合规**

**差距**: ✅ **已实现，完全合规**

---

### 4.2 ✅ `project_online`

**规范要求**: MCP Server 连接时，Dashboard 广播到所有 Web UI

**实际实现**:

**Dashboard 广播** (websocket.rs:379-388):
```rust
let msg = ProtocolMessage::new("project_online", ProjectOnlinePayload { project });
broadcast_to_ui_clients(&state, msg).await;
```
✅ **完全合规**

**Web UI 接收** (app.js:341-344):
```javascript
case 'project_online':
    handleProjectOnline(message.payload.project);
```
✅ **完全合规**

**差距**: ✅ **已实现，完全合规**

---

### 4.3 ✅ `project_offline`

**规范要求**: MCP Server 断开时，Dashboard 广播到所有 Web UI

**实际实现**:

**Dashboard 广播** (websocket.rs:438-447):
```rust
let msg = ProtocolMessage::new("project_offline",
    ProjectOfflinePayload { project_path });
broadcast_to_ui_clients(&state, msg).await;
```
✅ **完全合规**

**Web UI 接收** (app.js:345-348):
```javascript
case 'project_offline':
    handleProjectOffline(message.payload.project_path);
```
✅ **完全合规**

**差距**: ✅ **已实现，完全合规**

---

## 5. 实时数据层 (Future - v1.1+)

### 5.1 🔴 `event_update`

**规范状态**: Reserved for future use (v1.1+)

**实际实现**: ❌ **未实现**

---

### 5.2 🔴 `task_update`

**规范状态**: Reserved for future use (v1.1+)

**实际实现**: ❌ **未实现**

---

## 6. 错误处理

### 6.1 ⚠️ `error` 消息类型

**规范要求** (Section 4.5.1):
```json
{
  "version": "1.0",
  "type": "error",
  "payload": {
    "code": "error_code",
    "message": "Human-readable error",
    "details": { ... }
  }
}
```

**实际实现**:
- ✅ Dashboard 有版本验证 (websocket.rs:64-74)
- ❌ **未定义 `ErrorPayload` 数据结构**
- ❌ **未发送标准化 `error` 消息**
  - 当前只打印日志 `tracing::warn!(...)`
  - 未通过 WebSocket 通知客户端错误

**差距**: 🔴 **未实现标准错误响应**

---

## 7. 状态恢复机制

### 7.1 ✅ Dashboard 重启恢复

**规范要求** (Section 5.1):
1. MCP Server 检测连接丢失 → RECONNECTING
2. 指数退避重连
3. 重连后重新发送 `hello` + `register`
4. Dashboard 从注册重建内存状态

**实际实现**:

**MCP Client 重连** (ws_client.rs:93-120):
```rust
loop {
    match connect_and_run(...).await {
        Ok(()) => { /* normal close */ }
        Err(e) => { /* connection error */ }
    }

    // Exponential backoff
    let delay_idx = attempt.min(RECONNECT_DELAYS.len() - 1);
    let delay_secs = RECONNECT_DELAYS[delay_idx];
    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
    attempt += 1;
}
```

✅ **符合规范：无限重连 + 指数退避**

**Dashboard 重建状态** (websocket.rs:340-391):
- ✅ 每次 `register` 消息都重新注册
- ✅ 内存状态 `mcp_connections` 完全从注册重建
- ✅ 无持久化状态依赖（无 Registry 文件）

**差距**: ✅ **已实现，符合规范**

---

### 7.2 ✅ MCP Server 重启恢复

**规范要求** (Section 5.2):
1. Dashboard 检测 WebSocket 关闭 → 广播 `project_offline`
2. Web UI 显示项目为灰色
3. MCP Server 重启 → 重连 → 发送 `register`
4. Dashboard 广播 `project_online`

**实际实现**:

**Dashboard 检测断开** (websocket.rs:421-447):
```rust
recv_task.await {
    // Connection closed
    if let Some(path) = project_path.as_ref() {
        state.remove_mcp_connection(path).await;

        let msg = ProtocolMessage::new("project_offline",
            ProjectOfflinePayload { project_path: path.clone() });
        broadcast_to_ui_clients(&state, msg).await;
    }
}
```

✅ **符合规范**

**差距**: ✅ **已实现，符合规范**

---

### 7.3 ⚠️ Web UI 刷新恢复

**规范要求** (Section 5.3):
1. Web UI 从 `localStorage` 读取项目列表（历史）
2. 连接 Dashboard → 发送 `hello`
3. Dashboard 发送 `init`（当前在线项目）
4. Web UI 合并：localStorage + init

**实际实现**:

**Web UI** (app.js:73-104):
```javascript
function loadProjectsFromStorage() {
    const stored = localStorage.getItem(PROJECT_STORAGE_KEY);
    return stored ? JSON.parse(stored) : [];
}
```
✅ localStorage 存储项目历史

**连接流程** (app.js:188-200):
```javascript
dashboardWebSocket.onopen = async () => {
    // ❌ 未发送 hello
    console.log('✓ Waiting for WebSocket init message...');
};
```
❌ **跳过 `hello` 握手**

**接收 init** (app.js:367-387):
```javascript
function handleInitMessage(projects) {
    // Clear online projects
    onlineProjects.clear();

    // Add all projects from init
    projects.forEach(p => {
        onlineProjects.set(p.path, p);
        addProjectToStorage(p);
    });

    renderProjectTabs();
}
```
✅ 合并在线状态

**差距**: 🟡 **功能实现但跳过 `hello` 握手**

---

## 8. 重连策略

### 8.1 ✅ 指数退避 + 抖动

**规范要求** (Section 3.3):
```
delays = [1, 2, 4, 8, 16, 32] seconds (capped at 32s)
actual_delay = base_delay + random(0, 1000ms)
max_attempts = unlimited
```

**实际实现**:

**MCP Client** (ws_client.rs:66-120):
```rust
const RECONNECT_DELAYS: &[u64] = &[1, 2, 4, 8, 16, 32];

let delay_idx = attempt.min(RECONNECT_DELAYS.len() - 1);
let delay_secs = RECONNECT_DELAYS[delay_idx];
```
✅ 指数退避，32 秒封顶
❌ **未实现抖动 (jitter)**

**Web UI** (app.js:11, 231-246):
```javascript
const WS_RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 32000];

const baseDelay = WS_RECONNECT_DELAYS[delayIndex];
const jitter = baseDelay * 0.25 * (Math.random() * 2 - 1);
const delay = Math.max(0, baseDelay + jitter);
```
✅ 指数退避 + ±25% 抖动
✅ 无限重连

**差距**: 🟡 **Web UI 完全合规，MCP Client 缺少抖动**

---

## 9. 协议版本

### 9.1 ✅ 版本协商

**规范要求** (Section 6.1):
1. Client 发送版本号
2. Server 检查兼容性
3. 不兼容 → 发送 `error` with `unsupported_version`

**实际实现**:

**Dashboard 验证** (websocket.rs:64-74):
```rust
pub fn from_json(json: &str) -> Result<Self, String> {
    let msg: Self = serde_json::from_str(json)?;

    let expected_major = PROTOCOL_VERSION.split('.').next().unwrap_or("1");
    let received_major = msg.version.split('.').next().unwrap_or("0");

    if expected_major != received_major {
        return Err(format!(
            "Protocol version mismatch: expected {}, got {}",
            PROTOCOL_VERSION, msg.version
        ));
    }

    Ok(msg)
}
```
✅ 主版本号验证
❌ **未发送 `error` 消息，只返回 Rust `Err`**

**Web UI 验证** (app.js:327-333):
```javascript
const expectedMajor = PROTOCOL_VERSION.split('.')[0];
const receivedMajor = message.version.split('.')[0];
if (expectedMajor !== receivedMajor) {
    console.error(`Protocol version mismatch: expected ${PROTOCOL_VERSION}, got ${message.version}`);
    return;
}
```
✅ 主版本号验证
❌ **未通知服务器版本不兼容**

**差距**: 🟡 **验证逻辑存在，但未发送标准错误响应**

---

## 10. 安全性

### 10.1 ⚠️ 认证

**规范现状** (Section 8.1):
- Current: 无认证（仅 localhost）
- Future: Token-based auth in `hello` message

**实际实现**:
- ❌ **无认证机制**
- ⚠️ 仅绑定 `127.0.0.1`（localhost-only）

**差距**: 🟡 **符合当前规范（无认证），但未来需实现**

---

### 10.2 ✅ 路径验证

**规范要求** (Section 8.2):
- 拒绝临时目录路径
- 验证绝对路径

**实际实现**:
- ✅ MCP Client 检查临时目录 (ws_client.rs:77-91)
- ⚠️ Dashboard 检查但未拒绝 (websocket.rs:355-363)

**差距**: 🟡 **部分实现，Dashboard 端未拒绝**

---

## 11. 核心差距总结

### 11.1 🔴 严重差距（影响合规性）

| 编号 | 差距描述 | 规范章节 | 优先级 |
|-----|---------|---------|--------|
| G1 | `hello`/`welcome` 握手未实现 | 4.1.1, 4.1.2 | P1 |
| G2 | `goodbye` 消息未使用 | 4.1.4 | P2 |
| G3 | 标准 `error` 消息未实现 | 4.5 | P1 |
| G4 | MCP Client 未处理 `ping`/`pong` | 4.1.3 | P2 |
| G5 | MCP Client 重连无抖动 | 3.3 | P3 |

---

### 11.2 🟡 中等差距（影响可靠性）

| 编号 | 差距描述 | 规范章节 | 优先级 |
|-----|---------|---------|--------|
| M1 | Dashboard 路径验证不拒绝 | 4.2.1, 8.2 | P2 |
| M2 | 版本不匹配未发送 error | 6.1 | P3 |
| M3 | Dashboard→MCP 使用 `pong` 而非 `ping` | 4.1.3 | P3 |

---

### 11.3 🟢 轻微差距（不影响核心功能）

| 编号 | 差距描述 | 规范章节 | 优先级 |
|-----|---------|---------|--------|
| L1 | `event_update` 未实现 | 4.4.1 (Future) | P4 |
| L2 | `task_update` 未实现 | 4.4.2 (Future) | P4 |
| L3 | 无认证机制 | 8.1 (Future) | P4 |

---

## 12. 兼容性评估

### 12.1 当前实现能否互操作？

✅ **YES** - 核心流程可工作：
1. MCP Server 连接 + 注册 → ✅
2. Dashboard 接收注册 + 广播 → ✅
3. Web UI 接收状态 + 显示 → ✅
4. 断线重连 + 状态恢复 → ✅

---

### 12.2 与规范的兼容程度？

🟡 **70% 合规**：
- ✅ 核心消息格式合规
- ✅ 核心状态同步合规
- ⚠️ 握手协议未完全实现
- ⚠️ 错误处理不规范
- ❌ 部分消息类型未使用

---

## 13. 推荐修复优先级

### Phase 1: 关键合规性修复 (P0-P1)

#### Fix 1.1: 实现标准 `error` 消息
- **文件**: `src/dashboard/websocket.rs`
- **添加**: `ErrorPayload` 结构体
- **修改**: 所有错误情况发送 `error` 消息

#### Fix 1.2: 实现 `hello`/`welcome` 握手
- **文件**:
  - `src/dashboard/websocket.rs` (Dashboard)
  - `src/mcp/ws_client.rs` (MCP Client)
  - `static/js/app.js` (Web UI)
- **流程**:
  1. Client 连接后发送 `hello`
  2. Server 验证版本 + 发送 `welcome`
  3. 握手成功后才允许 `register` / `init`

---

### Phase 2: 可靠性增强 (P2)

#### Fix 2.1: 实现 `goodbye` 优雅关闭
- **场景**:
  - Dashboard 关闭前广播 `goodbye`
  - MCP Client 断开前发送 `goodbye`
- **好处**: Client 可区分"主动关闭"vs"连接中断"

#### Fix 2.2: MCP Client 实现 `ping`/`pong`
- **当前**: 仅 Dashboard 发送心跳
- **改进**: MCP Client 也处理 `ping` → 响应 `pong`

#### Fix 2.3: 路径验证强制拒绝
- **文件**: `src/dashboard/websocket.rs:355-363`
- **修改**: 临时目录路径 → 发送 `registered{success:false}`

---

### Phase 3: 优化和未来准备 (P3-P4)

#### Fix 3.1: MCP Client 重连加抖动
- **文件**: `src/mcp/ws_client.rs:114-118`
- **添加**: ±1000ms 随机抖动

#### Fix 3.2: 版本不匹配发送 `error`
- **当前**: 只打印日志
- **改进**: 发送 `error{code: "unsupported_version"}`

#### Fix 3.3: 预留 v1.1 实时同步
- **准备**: `event_update` / `task_update` 结构体
- **时机**: 后续版本实现

---

## 14. 测试缺口

根据规范 Section 9.1，以下测试**应该存在但当前缺失**：

### 缺失的协议合规性测试

| 测试编号 | 测试名称 | 覆盖章节 | 状态 |
|---------|---------|---------|------|
| T1 | 连接握手测试 (`hello`/`welcome`) | 4.1.1, 4.1.2 | ❌ 无 |
| T2 | 心跳测试 (30s `ping`/`pong`) | 4.1.3 | ❌ 无 |
| T3 | 重连指数退避测试 | 3.3 | ❌ 无 |
| T4 | Dashboard 重启恢复测试 | 5.1 | ❌ 无 |
| T5 | 广播测试（多 Web UI） | 4.3 | ❌ 无 |
| T6 | 版本不兼容测试 | 6.1 | ❌ 无 |
| T7 | 路径验证拒绝测试 | 4.2.1, 8.2 | ❌ 无 |

---

## 15. 架构偏差分析

### 15.1 ✅ 符合规范的设计

1. **无状态 Dashboard**
   - 规范: Dashboard 不持久化状态（无 Registry 文件）
   - 实现: ✅ 完全依赖内存 `mcp_connections`

2. **单一真相源（WebSocket）**
   - 规范: WebSocket 是实时状态的唯一来源
   - 实现: ✅ `get_online_projects_with_current()` 统一数据源

3. **星型拓扑**
   - 规范: Dashboard 为中心，MCP + Web UI 为客户端
   - 实现: ✅ 符合

---

### 15.2 ⚠️ 偏离规范的设计

1. **LocalStorage 心跳机制**
   - 规范: 未定义
   - 实现: Web UI 每 30 秒轮询 `/api/health` 检测离线项目
   - 分析: **规范外功能**，但不违反协议（HTTP API 独立于 WebSocket）

2. **`pong` 作为心跳**
   - 规范: Dashboard → Client 发送 `ping`，Client → Dashboard 响应 `pong`
   - 实现: Dashboard → MCP Client 直接发送 `pong`（而非 `ping`）
   - 分析: **语义偏差**，但不影响连接检测

---

## 16. 向后兼容性路径

如果按优先级修复上述差距，如何保证向后兼容？

### 策略 1: 渐进式握手协议

```
// 兼容旧实现（无 hello）
if first_message.type == "register" {
    // 旧客户端，直接处理 register
    handle_register(payload);
}
if first_message.type == "hello" {
    // 新客户端，先 welcome，再允许 register
    send_welcome();
    wait_for_register();
}
```

### 策略 2: 版本协商宽容模式

```
// v1.0 客户端连接 v1.1 服务器
if client_version == "1.0" && server_version == "1.1" {
    // 降级到 v1.0 特性集
    disable_event_update();
    disable_task_update();
}
```

---

## 17. 生产就绪度清单

| 检查项 | 状态 | 备注 |
|--------|------|------|
| 消息格式标准化 | ✅ | 完全合规 |
| 连接握手完整性 | 🔴 | 缺少 `hello`/`welcome` |
| 心跳机制 | 🟡 | 部分实现 |
| 重连机制 | 🟡 | 有重连但无抖动 |
| 错误处理 | 🔴 | 无标准 `error` 消息 |
| 状态恢复 | ✅ | Dashboard 重启、MCP 重启均正常 |
| 版本管理 | 🟡 | 验证存在但无标准错误响应 |
| 安全性 | 🟡 | Localhost-only，符合当前规范 |
| 测试覆盖 | 🔴 | 缺少协议合规性测试 |
| 文档一致性 | 🟡 | 规范存在，实现部分偏差 |

**总体评分**: 🟡 **75/100** - 可用，但需改进

---

## 18. 建议行动计划

### Milestone 1: 协议合规性 (2-3 周)
- [ ] 实现 `hello`/`welcome` 握手
- [ ] 实现标准 `error` 消息
- [ ] 实现 `goodbye` 优雅关闭
- [ ] MCP Client 添加 `ping`/`pong` 处理
- [ ] MCP Client 重连加抖动

### Milestone 2: 测试覆盖 (1-2 周)
- [ ] 添加协议合规性测试套件
- [ ] 添加 Mock Dashboard / Mock Client
- [ ] 添加场景测试（重连、恢复、广播）

### Milestone 3: 文档同步 (1 周)
- [ ] 更新 PROTOCOL_GAP_ANALYSIS.md（标记已修复）
- [ ] 更新 PROTOCOL_MIGRATION_PLAN.md（调整时间表）
- [ ] 添加协议合规性徽章到 README

---

## 19. 总结

### 核心发现

1. **✅ 已实现的核心价值**:
   - 项目注册和状态同步正常工作
   - 重连机制稳定可靠
   - 无状态 Dashboard 设计优秀

2. **🔴 关键缺失**:
   - 连接握手协议（`hello`/`welcome`）未实现
   - 标准错误响应缺失
   - 协议合规性测试缺失

3. **🟡 可优化点**:
   - 心跳机制命名不一致
   - 重连抖动缺失
   - 路径验证不强制拒绝

### 战略建议

**当前状态**: **可用于生产，但不完全合规**

**建议路径**:
1. **短期（1 个月）**: 修复 P0-P1 差距 + 添加测试
2. **中期（2-3 个月）**: 完整协议合规 + 文档同步
3. **长期（6 个月）**: v1.1 实时同步特性

**风险评估**:
- 🟢 **低风险**: 当前实现稳定，核心流程可用
- 🟡 **中风险**: 缺少标准化错误处理可能导致调试困难
- 🔴 **高风险**: 无协议测试，未来版本升级可能引入不兼容

---

**文档版本**: 1.0
**作者**: Claude (Ultrathink 模式)
**审核状态**: 待人工审核
