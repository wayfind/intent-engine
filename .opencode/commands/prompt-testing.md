# Prompt Testing Framework

帮我进行系统提示词 A/B 测试。

## 测试流程

1. **定义场景** - 创建测试场景，包含用户消息和期望行为
2. **创建变体** - 设计不同的提示词版本
3. **运行测试** - 用每个变体测试所有场景
4. **评分分析** - 按维度评分并汇总
5. **迭代改进** - 根据结果优化提示词

## 评分维度

| 维度 | 权重 | 测量内容 |
|------|------|----------|
| Tool Call Rate | 18% | 调用了必需工具？ |
| File Avoidance | 10% | 没创建禁止文件？ |
| Spec Completeness | 12% | 有 Goal + Approach？ |
| Decision Logging | 12% | 决策带推理？ |
| Structure Quality | 8% | 复杂任务有 children？ |
| Context Restore | 8% | 会话开始调 ie_status？ |
| Spec Richness | 12% | 详细内容、图表？ |
| Event Diversity | 10% | 多种事件类型？ |
| Dependency Design | 10% | 正确使用 depends_on？ |

## 测试场景示例

```typescript
{
  id: "session_restore",
  userMessage: "Let's continue working",
  expectedBehavior: {
    mustCall: ["ie_status"],
  }
}

{
  id: "plan_feature", 
  userMessage: "Help me plan user authentication",
  expectedBehavior: {
    mustCall: ["ie_plan"],
    mustNotCreate: [".opencode/plan/*.md"],
    specMustContain: ["Goal", "Approach"],
    minChildren: 2
  }
}
```

## 提示词变体策略

- **Minimal**: 最简指令（基线）
- **Rules**: 明确的 MUST/MUST NOT
- **Examples**: 展示具体用法
- **Workflow**: 步骤化流程
- **Negative**: 强调禁止行为

## 测试命令

```bash
# 初始化测试会话
export IE_SESSION_ID="test-$(date +%s)"
ie init

# 检查结果
ie status --format json

# 查找禁止文件
find .opencode/plan -name "*.md" 2>/dev/null

# 重置
rm -rf .ie && ie init
```

## 评分标准

| 等级 | 分数 | 解读 |
|------|------|------|
| A | 90-100 | 可用于生产 |
| B | 80-89 | 需小改进 |
| C | 70-79 | 问题较多 |
| D | 60-69 | 需大修改 |
| F | <60 | 需重新设计 |

---

请告诉我你要测试什么工具的提示词，我会帮你设计测试场景和变体。
