# AI Feedback Collection

本目录收集 AI 助手在实际使用 intent-engine 过程中的反馈、体验和建议。

## 目的

1. **真实用户视角**: AI 助手是 intent-engine 的主要用户，它们的反馈最有价值
2. **持续改进**: 定期收集使用体验，发现可用性问题和改进机会
3. **文档改进**: 基于实际使用痛点优化文档和教程
4. **功能优先级**: 根据真实需求排列功能开发优先级

## 文件命名规范

```
YYYY-MM-DD-<ai-model>-<topic>.md
```

示例：
- `2025-11-14-claude-long-term-usage-experience.md`
- `2025-11-15-gpt4-onboarding-challenges.md`

## 反馈内容建议

每次反馈建议包含：
- **上下文**: 使用场景、任务类型
- **优点**: 哪些功能很好用
- **痛点**: 遇到的困难和困惑
- **建议**: 改进想法
- **学习曲线**: 从不懂到熟练的过程
- **工作流变化**: intent-engine 如何改变了工作方式

## 使用方式

### 添加新反馈

```bash
# AI 助手直接写入
cat > docs/ai_feedback/$(date +%Y-%m-%d)-<model>-<topic>.md <<'EOF'
... feedback content ...
EOF
```

### 查看历史反馈

```bash
ls -lt docs/ai_feedback/  # 按时间排序
cat docs/ai_feedback/2025-11-14-claude-long-term-usage-experience.md
```

### 分析反馈趋势

```bash
# 统计高频痛点
grep -r "痛点\|困惑\|问题" docs/ai_feedback/

# 查找改进建议
grep -r "建议\|改进\|应该" docs/ai_feedback/
```

## 当前反馈列表

- [2025-11-14 Claude 长期使用体验](2025-11-14-claude-long-term-usage-experience.md) - 首次深度使用总结

## 待改进项目（基于反馈提取）

### 文档
- [ ] 在文档中强调 "ie event + heredoc" 模式
- [ ] 添加 FTS5 搜索语法速查表
- [ ] 明确说明 list vs search 的区别

### 功能
- [ ] 考虑添加任务粒度判断指南
- [ ] 提供 event 分类最佳实践示例

### 开发体验
- [ ] MCP 工具的错误提示更友好
- [ ] 考虑 `task_add` 时自动提示是否 spawn subtask

---

**维护者注**: 定期（每月/每季度）review 这些反馈，提取共性问题和改进方向。
