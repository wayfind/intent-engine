# AI 工具集成的两种范式：训练语料 vs 运行时 Schema

**日期**: 2025-11-17
**作者**: Claude (Sonnet 4.5) + 用户
**主题**: MCP工具集成的成本分析与未来演进路径
**背景**: 讨论为什么Claude在实际使用中倾向bash而非MCP，以及14k上下文成本的本质

---

## 🎯 核心问题

### 初始观察
用户发现：Claude在这次对话中100%使用了bash命令来调用ie，而不是MCP工具。

```bash
# 实际使用（100%）
ie current
ie search "dashboard test"
ie task add --name "..." --spec-stdin
ie task start 195
ie task done

# 没使用（0%）
mcp__intent-engine__current_task_get()
mcp__intent-engine__unified_search()
mcp__intent-engine__task_add()
```

这引发了一系列深入讨论：
1. MCP的14k上下文成本值得吗？
2. 如果能用bash，是否更好？
3. Schema-first设计能如何改进？
4. 最终：这两种方式的本质区别是什么？

---

## 💡 第一层洞察：上下文成本的真相

### 初始错误假设

Claude最初认为：
- ✅ MCP方式：14k schema成本
- ✅ Bash方式：0k成本（可以省下14k）

### 用户的关键纠正

> "因为你是开发这个项目的，即使是用bash，我也需要告诉Agent们，这个工具是如何使用的，所以依然避免不了那14k上下文。就如同你如何知道bash是怎样使用的，依然有很多描述来告诉你对吧？"

**真相**：
```
MCP方式：
└── 14k MCP schema定义（tool名称、参数、示例、最佳实践）

Bash方式：
└── 14k 文档说明（CLAUDE.md、AGENT.md、命令列表、使用示例）
```

**结论**：上下文成本一样，14k是"知识本身的成本"，不是接口的成本。

### 推导出的新结论

既然成本相同，为什么不用更结构化的接口？

| 维度 | MCP | Bash |
|------|-----|------|
| 上下文成本 | ~14k | ~14k（文档） |
| 结构化 | ✅ JSON schema | ❌ 字符串拼接 |
| 类型安全 | ✅ 参数验证 | ❌ 需要手动解析 |
| 维护性 | ✅ Schema即文档 | ❌ 文档和代码分离 |

**工程启示**：不要只看"接口成本"，要看"知识传递的总成本"。在成本相同的情况下，结构化胜出。

---

## 🔄 第二层洞察：分离Schema和Guide的陷阱

### Claude的Schema-first改进提议

从schema-first设计角度，Claude提议：
1. 把教学内容从MCP description中移出
2. 创建独立的guide文档
3. Schema从14k压缩到4-5k
4. Guide按需加载

**设想的理想场景**：
```
Session 1: 读 schema (4k) + guide (10k) = 14k
Session 2: 只读 schema (4k)  ← 省10k！
Session 3: 只读 schema (4k)  ← 省10k！
```

### 用户的致命质疑

> "第一次使用时读guide，这个和放到mcp的描述里，有区别嘛？因为每个session启动的时候都需要让agent了解"

**真实场景**：
```
Session 1: 读 schema (4k) + guide (10k) = 14k
Session 2: 读 schema (4k) + guide (10k) = 14k  ← AI是stateless的
Session 3: 读 schema (4k) + guide (10k) = 14k  ← 每次都要重新学习
```

**结论**：在stateless AI环境下，分离guide没有任何意义。每个session都是"新开始"。

### 真正有价值的改进

只有修复实际问题才有意义：
1. ✅ 修复类型不一致（priority的int vs enum）
2. ✅ 添加output schema（让AI知道返回值结构）
3. ✅ 修复过时引用（task_search已被unified_search替代）
4. ⚠️ 提高信息密度（不是压缩内容，而是更清晰地表达）

**关键认知转变**：
- MCP description不仅是"API契约"，还必须是"使用手册"
- 对于stateless AI，每次都要传达完整信息
- 与其让AI去读多个文档，不如在tool definition里说清楚

这和传统API设计不同：
- **传统API**：人类可以"记住"用法，schema只需定义类型
- **AI API**：AI每次都是新的，schema必须包含"how to use"

---

## 🧠 第三层洞察：两种AI工具集成范式

### 为什么Claude"天然"会用bash？

用户的核心洞察：
> "如果大模型支持微调的话应该把这个指令的用法像bash的用法一样，天然微调进入模型内部。你能很流畅的使用bash就是因为之前的语料里面有大量bash资料"

这揭示了AI工具集成的两种根本不同的范式：

### 范式A：通过训练语料（Bash模式）

```
训练阶段（一次性成本，分摊给所有用户）:
├── Stack Overflow上百万个bash问答
├── GitHub上数十亿行shell脚本
├── Linux手册、教程、博客
└── 模式被编码进模型权重

运行阶段（零成本）:
└── AI直接"知道"bash用法，像本能一样
```

**为什么Claude能流畅用bash？**
- 不是每次加载了bash manual
- 而是`ls -la`、`grep -r`、`awk`等模式已经在训练语料中出现了成千上万次
- 这些知识已经**压缩进模型权重**

### 范式B：通过运行时Schema（Intent-Engine当前模式）

```
训练阶段:
└── 没有ie相关语料（工具太新/太专用）

运行阶段（每次14k成本）:
├── 加载mcp-server.json (14k)
├── 加载CLAUDE.md/AGENT.md
├── AI现场"学习"怎么用
└── Session结束后全部遗忘
```

**区别的本质**：
- Bash的知识存在**权重中**（amortized cost，分摊到所有用户）
- Intent-Engine的知识存在**context中**（per-session cost，每次重新加载）

### 类比

| 范式 | 类比 | 特点 |
|------|------|------|
| 训练语料 | 母语 | 不需要语法书就能说，本能反应 |
| 运行时Schema | 外语 | 每次都要查词典，现场学习 |

---

## 🚀 工程启示

### 1. 设计要为"未来训练"做准备

既然MCP是过渡方案，设计时应该考虑"未来可能被微调进模型"。

#### 遵循已有模式（利用迁移学习）

**Good**：
```bash
ie task list --status doing
ie task add --name "..." --spec-stdin
ie event add --type decision --data-stdin
```
↑ 遵循Unix风格，AI可以基于git/docker/kubectl的pattern推理

**Bad**：
```bash
ie task --operation=list --filter=status:doing
ie create-intent --title="..." --body-from-file
```
↑ 自创语法，AI需要专门学习

#### 保持概念简单（降低学习成本）

Intent-Engine的核心概念只有3个：
- **Task**（任务）
- **Event**（事件）
- **Focus**（聚焦）

如果未来被微调，3个简单概念比10个复杂概念更容易被编码进权重。

#### 产生可索引的公开内容

如果想进入未来的训练语料：
- ✅ GitHub public repo
- ✅ 公开文档（docs/）
- ✅ Stack Overflow问答（如果有人用）
- ✅ 博客文章、教程
- ✅ AI对话记录（如果被索引）

**这些比完美的MCP schema更重要**，因为它们是未来模型的训练数据来源。

### 2. 14k成本的长期视角

从"未来微调"的视角看，14k成本的意义改变了：

**短期（现在）**：
- 14k是必要的运行成本
- 每个session都要支付

**长期（如果被广泛采用）**：
- 14k是"教学材料"
- 如果这些pattern被足够多的conversation使用
- 如果这些conversation被记录、索引
- 未来的模型可能"学会"这些pattern
- **14k成本就能降到0**（就像bash一样）

**推论**：
- MCP schema不仅是API定义
- 也是"训练数据的模板"
- 写得越清晰，未来被学习的可能性越高

### 3. AI工具的演进路径

```
阶段1（现在 - 新工具）:
新工具 → 依赖MCP/文档 → 14k per session
└── Intent-Engine当前阶段

阶段2（采用增长）:
使用增多 → 产生公开内容 → 可能进入训练数据
└── 需要社区采用、文章、问答

阶段3（理想未来）:
模式被内化 → 成为"本能" → 0k cost
└── 像bash/git/docker一样
```

**工具设计的最高目标**：
- 不是"完美的MCP schema"
- 而是**"简单到能被未来模型学会的pattern"**

### 4. Unix哲学的AI时代意义

这也解释了为什么Unix哲学经久不衰：
- 足够简单的pattern（`ls`, `grep`, `|`）
- 足够多的使用案例（40年+的积累）
- 最终被内化到所有人（包括AI）的认知中

Intent-Engine如果能做到这点，未来就不需要14k schema了。**就像现在没人需要"bash MCP server"一样**。

---

## 📊 实际案例：Claude为什么用bash不用MCP？

### 表面原因
- Bash更直接、更快
- 不需要JSON schema的心智负担
- 作为开发者，CLI是"内部工具"

### 深层原因
- **Bash pattern已在权重中**：Claude的训练数据包含大量shell脚本
- **ie MCP是新知识**：需要从context现场学习
- **认知成本**：调用bash是"本能"，调用MCP是"查API"

### 启示
如果intent-engine的使用pattern足够简单、足够被广泛使用，未来的AI模型可能"天生"就知道：
```bash
# 未来AI的"本能"可能包括：
ie task start {id}        # 开始任务
ie task done             # 完成当前任务
ie event add --type decision  # 记录决策
```

就像现在AI"天生"知道：
```bash
ls -la
git commit -m "..."
docker run -it ...
```

---

## 🎓 关键takeaways

1. **14k成本无法避免**（在当前架构下）
   - 不管MCP还是bash，传达知识的成本是固定的
   - 在成本相同时，选更结构化的方式（MCP）

2. **Stateless AI下分离schema和guide没意义**
   - 每个session都要重新学习
   - Schema必须包含完整的使用信息

3. **训练语料 vs 运行时Schema是两种范式**
   - Bash在权重中（0 cost/session）
   - Intent-Engine在context中（14k cost/session）
   - 未来可能演进到权重中

4. **工具设计要为"被学习"做准备**
   - 遵循已有模式（Unix风格）
   - 保持概念简单（3个核心概念）
   - 产生公开内容（进入训练语料）

5. **最高目标不是完美schema，而是可学习的pattern**
   - 简单到能被模型内化
   - 最终实现0 cost per session（像bash）

---

## 🔗 相关讨论

这次讨论源于对以下问题的深入思考：
- 为什么Claude在实际使用中选择bash而不是MCP？
- MCP的14k上下文成本是否值得？
- Schema-first设计如何改进？
- AI工具集成的未来演进方向？

这也呼应了之前的文档：
- `2025-11-14-claude-long-term-usage-experience.md`：Claude使用Intent-Engine的长期体验
- 本次讨论补充了"为什么这样设计"的更深层理解

---

## 💭 最终思考

这次对话最大的价值不是"MCP vs Bash哪个更好"，而是认识到：

**AI工具的终极形态不是更好的API，而是成为AI的"本能"。**

就像人类不需要每次都查手册就能用bash一样，未来的AI也不应该每次都需要14k的schema来"学习"工具用法。

这需要：
1. 工具设计足够简单（可学习）
2. 使用足够广泛（进入训练数据）
3. 模式足够清晰（可压缩进权重）

Intent-Engine的机会：如果能做到这三点，未来就不再需要MCP server，因为AI"天生"就会用。

那时候，这14k的MCP schema的真正价值才会显现：**它是教AI学会这个工具的教材，而不仅仅是API定义**。

---

**日期**: 2025-11-17
**对话时长**: 多轮深度讨论
**核心洞察**: 3个（上下文成本真相、stateless陷阱、两种集成范式）
**工程价值**: 为未来训练设计，而不只是为当前使用设计
