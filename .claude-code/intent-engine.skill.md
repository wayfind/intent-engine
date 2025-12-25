# Intent-Engine Skill (v0.10)

AI 长期任务记忆系统 - 跨 session 持久化、层级任务、决策记录

## 触发条件

当以下情况出现时，使用 ie 而不是 TodoWrite：

- 用户说"帮我实现 X 功能"（多步骤工作）
- 用户说"继续昨天的工作"（跨 session）
- 用户问"之前为什么这么决定"（决策历史）
- 任务需要保留超过当前会话
- 任务有 3+ 个子步骤

**简单规则**：会丢了可惜 → 用 ie，用完即弃 → 用 TodoWrite

---

## 标准工作流

### 1. Session 开始：恢复上下文

```bash
ie status
```

返回：当前聚焦任务、祖先链、兄弟任务、子孙任务

### 2. 创建/更新任务：声明式操作

```bash
# 创建任务
echo '{"tasks":[{"name":"实现用户认证","status":"doing"}]}' | ie plan

# 创建带子任务的层级结构
echo '{"tasks":[{
  "name":"实现用户认证",
  "status":"doing",
  "children":[
    {"name":"设计数据库模式","status":"todo"},
    {"name":"实现登录API","status":"todo"}
  ]
}]}' | ie plan

# 更新任务状态
echo '{"tasks":[{"name":"设计数据库模式","status":"doing"}]}' | ie plan

# 完成任务
echo '{"tasks":[{"name":"设计数据库模式","status":"done"}]}' | ie plan
```

**关键特性**：
- 幂等操作：相同 name 会更新而非重复创建
- 自动聚焦：status:"doing" 的任务自动成为当前聚焦任务
- 自动父子：有聚焦任务时，新建的根任务自动成为其子任务

### 3. 记录决策：ie log

```bash
ie log decision "选择 JWT 而非 Session，因为需要支持移动端"
ie log blocker "等待第三方 API 密钥"
ie log milestone "核心功能完成"
ie log note "考虑后续添加缓存优化"
```

### 4. 搜索历史：ie search

```bash
ie search "todo doing"           # 查找所有未完成任务
ie search "authentication"       # 全文搜索任务和事件
ie search "decision JWT"         # 搜索特定决策
```

### 5. 查看任意任务上下文

```bash
ie status 42                     # 查看任务 42 的完整上下文（不改变聚焦）
ie status 42 -e                  # 包含事件历史
```

---

## 常用模式

### 模式 A：新功能开发

```bash
# 1. 创建任务并开始
echo '{"tasks":[{
  "name":"实现支付功能",
  "spec":"支持微信、支付宝支付",
  "status":"doing",
  "children":[
    {"name":"集成微信支付SDK"},
    {"name":"集成支付宝SDK"},
    {"name":"实现统一支付接口"}
  ]
}]}' | ie plan

# 2. 记录设计决策
ie log decision "采用策略模式统一支付接口"

# 3. 逐个完成子任务
echo '{"tasks":[{"name":"集成微信支付SDK","status":"doing"}]}' | ie plan
# ... 工作 ...
echo '{"tasks":[{"name":"集成微信支付SDK","status":"done"}]}' | ie plan

# 4. 所有子任务完成后，完成父任务
echo '{"tasks":[{"name":"实现支付功能","status":"done"}]}' | ie plan
```

### 模式 B：跨 Session 继续工作

```bash
# Session 开始时
ie status

# 看到聚焦任务和子任务列表，继续工作
echo '{"tasks":[{"name":"当前子任务","status":"doing"}]}' | ie plan
```

### 模式 C：处理阻塞

```bash
# 记录阻塞
ie log blocker "需要产品经理确认需求细节"

# 切换到其他任务
echo '{"tasks":[{"name":"另一个任务","status":"doing"}]}' | ie plan

# 阻塞解除后回来
ie search "todo doing"
echo '{"tasks":[{"name":"之前被阻塞的任务","status":"doing"}]}' | ie plan
```

### 模式 D：回顾决策历史

```bash
# 搜索相关决策
ie search "decision 认证"

# 查看特定任务的完整历史
ie status 42 -e
```

---

## ie vs TodoWrite 对比

| 能力 | TodoWrite | ie |
|------|-----------|-----|
| 持久化 | Session 内 | 永久 |
| 层级任务 | 否 | 是 |
| 决策记录 | 否 | ie log |
| 跨 Session | 否 | ie status |
| 搜索历史 | 否 | ie search |
| 可视化 | 否 | ie dashboard |

---

## 命令速查

| 命令 | 用途 |
|------|------|
| `ie status [id]` | 查看任务上下文（Session 开始必用）|
| `ie plan` | 创建/更新/完成任务（stdin JSON）|
| `ie log <type> <msg>` | 记录 decision/blocker/milestone/note |
| `ie search <query>` | 搜索任务和事件 |
| `ie dashboard start` | 启动 Web 可视化界面 |

---

## 注意事项

1. **ie plan 是幂等的**：相同 name 会更新，不会重复创建
2. **父任务完成前提**：所有子任务必须先完成
3. **自动聚焦**：status:"doing" 自动设为当前任务
4. **自动父子**：有聚焦任务时，新任务自动成为其子任务
5. **JSON 格式**：ie plan 从 stdin 读取 JSON
