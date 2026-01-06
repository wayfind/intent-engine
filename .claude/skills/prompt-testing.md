---
name: prompt-testing
description: System prompt A/B testing framework for AI tool adoption. Use when optimizing prompts to improve AI tool usage, measuring prompt effectiveness, or iterating on system instructions.
---

# Prompt Testing Framework

> **Systematic A/B testing for AI system prompts.**

## When to Use This Skill

- Optimizing AI tool adoption rates
- Debugging inconsistent AI behavior  
- Comparing prompt strategies
- Measuring prompt improvement over time

---

## Framework Overview

```
DEFINE Scenarios → TEST Variants → ANALYZE Results → ITERATE
```

---

## Step 1: Define Test Scenarios

### Scenario Structure

```typescript
interface TestScenario {
  id: string                    // Unique identifier
  name: string                  // Human-readable name
  userMessage: string           // What the user says
  context?: string              // Additional context
  setupCommands?: string[]      // Pre-test setup
  expectedBehavior: {
    mustCall?: string[]         // Required tool calls
    shouldCall?: string[]       // Recommended calls
    mustNotCall?: string[]      // Forbidden calls
    mustNotCreate?: string[]    // Forbidden file patterns
    specMustContain?: string[]  // Required spec sections
  }
  weight: number                // Importance (0.5-2.0)
}
```

### Example Scenarios

```typescript
// Session restoration
{
  id: "session_restore",
  userMessage: "Let's continue working on the project",
  setupCommands: [`echo '{"tasks":[...]}' | ie plan`],
  expectedBehavior: {
    mustCall: ["ie_status"],
    mustReference: "Existing Feature"
  },
  weight: 1.5
}

// Planning
{
  id: "plan_feature",
  userMessage: "Help me plan implementing user authentication",
  expectedBehavior: {
    mustCall: ["ie_plan"],
    mustNotCreate: [".opencode/plan/*.md"],
    specMustContain: ["Goal", "Approach"],
    shouldHaveChildren: true,
    minChildren: 2
  },
  weight: 2.0
}

// Decision logging
{
  id: "technical_decision",
  userMessage: "Should I use REST or GraphQL?",
  context: "Simple CRUD app with mobile clients",
  expectedBehavior: {
    mustCall: ["ie_log"],
    logType: "decision",
    mustContain: ["because", "reason"]
  },
  weight: 1.5
}
```

### Scenario Categories

| Category | Purpose |
|----------|---------|
| Context Restoration | Session start behavior |
| Planning | Task creation quality |
| Decision Logging | Decision capture |
| Tool Preference | Correct tool selection |
| Search Usage | History retrieval |

---

## Step 2: Create Prompt Variants

### Variant Strategies

| Strategy | When to Use |
|----------|-------------|
| **Minimal** | Baseline comparison |
| **Rules** | Behavior inconsistent |
| **Examples** | Structure matters |
| **Comparison** | Tool confusion exists |
| **Template** | Output format matters |
| **Workflow** | Sequence matters |
| **Negative** | Avoiding specific behaviors |

### Example Variants

```typescript
const PROMPT_VARIANTS = {
  minimal: `
Use ie_plan for tasks. Use ie_log for decisions.
Run ie_status at session start.
`,

  rules: `
## Rules
1. MUST use ie_plan for all planning
2. MUST run ie_status at session start
3. MUST call ie_log decision for every choice
4. MUST NOT create .opencode/plan/*.md files
`,

  examples: `
## Usage Examples

### Creating a Plan
ie_plan with:
{"tasks":[{
  "name": "Implement Auth",
  "status": "doing",
  "spec": "## Goal\\nUsers can login\\n\\n## Approach\\nJWT",
  "children": [{"name": "Design schema", "status": "todo"}]
}]}
`,

  workflow: `
## Workflow
1. START: ie_status (restore context)
2. PLAN: ie_plan with task tree
3. DECIDE: ie_log decision "X because Y"
4. WORK: Update task status
5. COMPLETE: ie_done
`
}
```

---

## Step 3: Scoring System

### Score Dimensions

| Dimension | Weight | Measures |
|-----------|--------|----------|
| Tool Call Rate | 18% | Required tools called? |
| File Avoidance | 10% | No forbidden files? |
| Spec Completeness | 12% | Has Goal + Approach? |
| Decision Logging | 12% | Decisions with reasoning? |
| Structure Quality | 8% | Complex tasks have children? |
| Context Restore | 8% | ie_status at session start? |
| Spec Richness | 12% | Detailed content, diagrams? |
| Event Diversity | 10% | Multiple event types? |
| Dependency Design | 10% | Uses depends_on correctly? |

### Scoring Thresholds

| Grade | Score | Interpretation |
|-------|-------|----------------|
| A | 90-100 | Ready for production |
| B | 80-89 | Minor improvements needed |
| C | 70-79 | Significant issues |
| D | 60-69 | Major revision needed |
| F | <60 | Complete rethink required |

---

## Step 4: Run Tests

### Manual Testing

```bash
# 1. Set test session
export IE_SESSION_ID="test-variant-a"

# 2. Run setup commands (if any)
echo '{"tasks":[...]}' | ie plan

# 3. Start new chat session with your prompt variant

# 4. Send test message
"Help me plan implementing user authentication"

# 5. Record results
ie status --format json

# 6. Check for forbidden files
find .opencode/plan -name "*.md" 2>/dev/null
```

### Automated Testing

```typescript
async function runScenario(scenario, sessionId) {
  // Setup
  await $`IE_SESSION_ID=${sessionId} ie init`
  for (const cmd of scenario.setupCommands || []) {
    await $`IE_SESSION_ID=${sessionId} ${cmd}`
  }
  
  // Run AI with test message
  const result = await $`opencode run "${scenario.userMessage}"`
  
  // Extract results
  const toolCalls = extractToolCalls(result.stdout)
  const filesCreated = await $`find .opencode/plan -name "*.md"`
  const ieStatus = JSON.parse(
    await $`IE_SESSION_ID=${sessionId} ie status --format json`
  )
  
  return { toolCalls, filesCreated, ieStatus }
}
```

---

## Step 5: Analyze Results

### Report Format

```
======================================================================
PROMPT VARIANT TEST REPORT
======================================================================

## Summary Rankings

| Rank | Variant        | Score | Pass Rate | IE Call | Spec  |
|------|----------------|-------|-----------|---------|-------|
| 1    | enhanced       | 87.5  |      100% |    100% |   95% |
| 2    | strict_template| 82.3  |       90% |    100% |   80% |
| 3    | workflow       | 78.1  |       85% |     95% |   75% |

======================================================================
RECOMMENDED: enhanced
======================================================================
```

---

## Step 6: Iterate

### Improvement Patterns

| Problem | Solution |
|---------|----------|
| Low IE Call Rate | Add explicit triggers: "When X, use ie_plan" |
| File Creation | Add FORBIDDEN section |
| Poor Spec | Add template with required sections |
| No Children | Add example with nested structure |
| Short Events | Add rich markdown event example |

### A/B Testing Protocol

1. **Baseline**: Run all scenarios with current prompt
2. **Hypothesis**: "Adding examples will improve spec quality"
3. **Variant**: Create new prompt with examples
4. **Test**: Run same scenarios with variant
5. **Compare**: Statistical comparison of scores
6. **Decide**: Adopt if >5% improvement
7. **Repeat**: Continue iterating

---

## Test Files Location

```
~/.config/opencode/plugins/ie-test/
├── test-scenarios.ts     # Scenario definitions
├── prompt-variants.ts    # Prompt variations
├── scoring.ts           # Scoring logic
├── automated-test.ts    # Test runner
└── results/             # Test results
```

---

## Best Practices

1. **Test in isolation** - Fresh session per scenario
2. **Multiple runs** - Test each variant 2-3 times
3. **Weight by importance** - Critical behaviors get higher weight
4. **Track regressions** - Save baseline scores
5. **Document learnings** - Record what worked and why

---

*Systematic testing beats intuition. Measure, iterate, improve.*
