# Intent-Engine: AI 速查手册

**用途**：人机协作的战略意图追踪。不是待办清单——是长期复杂工作的共享记忆层。

## 何时使用

当工作需要以下情况时创建任务：
- 多步骤或跨会话
- 大量上下文/规格说明
- 决策追踪（"为什么选择 X？"）
- 人机协作

## 核心命令 (v0.10.0)

### 1. 恢复上下文（总是第一步）
```bash
ie status              # 我在做什么？完整上下文恢复
ie status 42           # 查看特定任务上下文
```

### 2. 创建/更新/完成任务
```bash
# 所有任务操作通过 `ie plan` + JSON 标准输入
echo '{"tasks":[...]}' | ie plan
```

### 3. 记录事件
```bash
ie log decision "选择 X 而非 Y，因为..."
ie log blocker "卡在 API 限流"
ie log milestone "第一阶段完成"
ie log note "之后考虑缓存"
```

### 4. 搜索历史
```bash
ie search "todo doing"       # 查找未完成任务
ie search "JWT 认证"         # 全文搜索
ie search "decision"         # 查找决策
```

## 工作流模式

```bash
# 1. 创建带 spec 的任务（status:doing 必须有 spec）
echo '{"tasks":[{
  "name": "实现 OAuth2",
  "status": "doing",
  "spec": "## 目标\n用户通过 OAuth 认证\n\n## 方案\n使用 Passport.js"
}]}' | ie plan

# 2. 检查当前上下文
ie status

# 3. 记录关键决策
ie log decision "选择 Passport.js - 成熟库，文档好"

# 4. 分解为子任务（自动归属到聚焦任务）
echo '{"tasks":[
  {"name": "配置 Google OAuth", "status": "todo"},
  {"name": "配置 GitHub OAuth", "status": "todo"},
  {"name": "实现回调处理器", "status": "todo"}
]}' | ie plan

# 5. 开始处理子任务
echo '{"tasks":[{
  "name": "配置 Google OAuth",
  "status": "doing",
  "spec": "设置 Google Cloud Console，获取凭证"
}]}' | ie plan

# 6. 完成子任务
echo '{"tasks":[{"name": "配置 Google OAuth", "status": "done"}]}' | ie plan

# 7. 完成所有子任务后，再完成父任务
echo '{"tasks":[{"name": "实现 OAuth2", "status": "done"}]}' | ie plan
```

## JSON 任务格式

```json
{
  "tasks": [
    {
      "name": "任务名称（必填，唯一标识）",
      "status": "todo|doing|done",
      "spec": "描述（doing 时必填）",
      "priority": "critical|high|medium|low",
      "parent_id": null,
      "children": [...]
    }
  ]
}
```

### 关键字段

| 字段 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 唯一标识，用于更新 |
| `status` | 否 | `todo`（默认）、`doing`、`done` |
| `spec` | doing 时必填 | 目标 + 方案描述 |
| `priority` | 否 | `critical`、`high`、`medium`、`low` |
| `parent_id` | 否 | `null` = 根任务，省略 = 自动归属到聚焦任务 |
| `children` | 否 | 嵌套子任务数组 |

## 事件类型

| 类型 | 使用场景 |
|------|----------|
| `decision` | 选择 X 而非 Y，因为... |
| `blocker` | 卡住了，需要帮助或信息 |
| `milestone` | 完成重要阶段 |
| `note` | 一般观察 |

## 关键规则

1. **先 `ie status`** — 会话开始时必须运行（失忆恢复）
2. **`doing` 必须有 spec** — 开始前必须有目标 + 方案
3. **先完成子任务** — 父任务必须等所有子任务 `done` 后才能 `done`
4. **同名 = 更新** — `ie plan` 是幂等的，不会创建重复
5. **自动归属** — 新任务成为聚焦任务的子任务（用 `parent_id: null` 创建根任务）

## 常用模式

### 创建独立根任务
```bash
echo '{"tasks":[{
  "name": "不相关的 bug 修复",
  "status": "todo",
  "parent_id": null
}]}' | ie plan
```

### 带子任务的层级任务
```bash
echo '{"tasks":[{
  "name": "用户认证",
  "status": "doing",
  "spec": "完整认证系统",
  "children": [
    {"name": "JWT 令牌", "status": "todo"},
    {"name": "会话管理", "status": "todo"},
    {"name": "密码重置", "status": "todo"}
  ]
}]}' | ie plan
```

### 更新任务优先级
```bash
echo '{"tasks":[{
  "name": "已有任务",
  "priority": "critical"
}]}' | ie plan
```

### 查找未完成工作
```bash
ie search "todo doing"
```

## 反模式

| 不要 | 应该 |
|------|------|
| 没有 spec 就开始 `doing` | 总是包含目标 + 方案 |
| 忘记记录决策 | 立即 `ie log decision "..."` |
| 不检查焦点就创建任务 | 先 `ie status` |
| 子任务未完成就标记父任务完成 | 先完成所有子任务 |

## 会话工作流

```
会话开始：
  ie status                    # 恢复上下文

工作中：
  ie plan (创建/更新)          # 任务操作
  ie log decision "..."        # 记录选择

会话结束：
  ie plan (status:done)        # 完成已完成的工作
  ie status                    # 验证状态
```

## 理念

Intent-Engine 是 AI 的**外部大脑**：
- **ie status** = 失忆恢复
- **ie plan** = 分解持久化
- **ie log** = 决策透明
- **ie search** = 记忆检索

---

**完整文档**: [CLAUDE.md](../../../CLAUDE.md), [quickstart.md](quickstart.md)
