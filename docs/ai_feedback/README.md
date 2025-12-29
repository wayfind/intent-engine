# AI Feedback Collection

This directory collects feedback, experiences, and suggestions from AI assistants using intent-engine in practice.

## Purpose

1. **Real User Perspective**: AI assistants are primary users of intent-engine, their feedback is most valuable
2. **Continuous Improvement**: Regularly collect usage experience, discover usability issues and improvement opportunities
3. **Documentation Improvement**: Optimize documentation and tutorials based on actual pain points
4. **Feature Prioritization**: Prioritize feature development based on real needs

## File Naming Convention

```
YYYY-MM-DD-<ai-model>-<topic>.md
```

Examples:
- `2025-11-14-claude-long-term-usage-experience.md`
- `2025-11-15-gpt4-onboarding-challenges.md`

## Feedback Content Suggestions

Each feedback should include:
- **Context**: Usage scenario, task type
- **Positives**: Which features work well
- **Pain Points**: Difficulties and confusions encountered
- **Suggestions**: Improvement ideas
- **Learning Curve**: Process from novice to proficient
- **Workflow Changes**: How intent-engine changed work methods

## Usage

### Add New Feedback

```bash
# AI assistant writes directly
cat > docs/ai_feedback/$(date +%Y-%m-%d)-<model>-<topic>.md <<'EOF'
... feedback content ...
EOF
```

### View Historical Feedback

```bash
ls -lt docs/ai_feedback/  # Sort by time
cat docs/ai_feedback/2025-11-14-claude-long-term-usage-experience.md
```

### Analyze Feedback Trends

```bash
# Count common pain points
grep -r "pain\|confusion\|issue" docs/ai_feedback/

# Find improvement suggestions
grep -r "suggest\|improve\|should" docs/ai_feedback/
```

## Current Feedback List

- [2025-11-14 Claude Long-term Usage Experience](2025-11-14-claude-long-term-usage-experience.md) - First deep usage summary

## Improvement Items (Extracted from Feedback)

### Documentation
- [ ] Emphasize `ie log` + heredoc pattern in documentation
- [ ] Add FTS5 search syntax quick reference
- [ ] Clarify the difference between status filter and full-text search

### Features
- [ ] Consider adding task granularity guide
- [ ] Provide event categorization best practices examples

### Developer Experience
- [ ] Improve CLI error messages
- [ ] Consider auto-suggesting subtask creation

---

**Maintainer Note**: Review these feedbacks regularly (monthly/quarterly) to extract common issues and improvement directions.
