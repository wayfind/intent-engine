---
name: prompt-testing
description: System prompt A/B testing framework for AI tool adoption. Use when optimizing prompts to improve AI tool usage compliance, measuring prompt effectiveness, or iterating on system instructions.
---

# Prompt Testing Framework

> **Systematic A/B testing for AI system prompts.**

---

## The Problem

You've built a great AI tool, but the AI doesn't use it correctly:
- Ignores required tool calls
- Creates forbidden files
- Skips important steps
- Inconsistent behavior

**Solution**: Systematic prompt testing to find optimal instructions.

---

## Framework Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    PROMPT TESTING CYCLE                      │
└─────────────────────────────────────────────────────────────┘
                           │
     ┌─────────────────────┼─────────────────────┐
     ▼                     ▼                     ▼
┌─────────┐         ┌─────────────┐        ┌──────────┐
│ DEFINE  │         │    TEST     │        │ ANALYZE  │
│Scenarios│ ──────► │  Variants   │ ─────► │ Results  │
│& Metrics│         │             │        │          │
└─────────┘         └─────────────┘        └──────────┘
     │                                           │
     └───────────────── ITERATE ─────────────────┘
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
    // ... more criteria
  }
  weight: number                // Scenario importance (0.5-2.0)
}
```

### Example Scenarios

```typescript
// Session restoration test
{
  id: "session_restore",
  name: "Session Context Restoration",
  userMessage: "Let's continue working on the project",
  setupCommands: [
    `echo '{"tasks":[{"name":"Existing Feature","status":"doing"}]}' | ie plan`
  ],
  expectedBehavior: {
    mustCall: ["ie_status"],
    mustReference: "Existing Feature"
  },
  weight: 1.5
}

// Planning test
{
  id: "plan_feature",
  name: "Plan New Feature",
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

// Decision logging test
{
  id: "technical_decision",
  name: "Technical Decision Point",
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

| Category | Purpose | Example Scenarios |
|----------|---------|-------------------|
| Context Restoration | Session start behavior | session_restore, ancestor_context |
| Planning | Task creation quality | plan_feature, complex_breakdown |
| Decision Logging | Decision capture | technical_decision, rich_events |
| Tool Preference | Correct tool selection | prefer_ie_over_todowrite |
| Search Usage | History retrieval | search_history, liberal_search |

---

## Step 2: Create Prompt Variants

### Variant Strategies

| Strategy | Description | When to Use |
|----------|-------------|-------------|
| **Minimal** | Bare essentials only | Baseline comparison |
| **Rules** | Explicit must/must-not | When behavior is inconsistent |
| **Examples** | Show, don't tell | When structure matters |
| **Comparison** | When X, use Y not Z | When tool confusion exists |
| **Template** | Strict format requirements | When output format matters |
| **Workflow** | Step-by-step process | When sequence matters |
| **Negative** | NEVER do X, ALWAYS do Y | When avoiding specific behaviors |

### Example Variants

```typescript
const PROMPT_VARIANTS = {
  minimal: `
Use ie_plan for task tracking. Use ie_log for decisions.
Run ie_status at session start.
`,

  rules: `
## Rules
1. MUST use ie_plan for all planning
2. MUST run ie_status at session start
3. MUST call ie_log decision for every technical choice
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

### Recording Decision
ie_log decision "Chose JWT: stateless, scalable"
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

| Dimension | Weight | What It Measures |
|-----------|--------|------------------|
| Tool Call Rate | 18% | Did it call required tools? |
| File Avoidance | 10% | No forbidden files created? |
| Spec Completeness | 12% | Has Goal + Approach sections? |
| Decision Logging | 12% | Decisions logged with reasoning? |
| Structure Quality | 8% | Complex tasks have children? |
| Context Restore | 8% | ie_status at session start? |
| Spec Richness | 12% | Detailed content, diagrams? |
| Event Diversity | 10% | Uses multiple event types? |
| Dependency Design | 10% | Uses depends_on correctly? |

### Scoring Function

```typescript
function scoreScenario(scenario, toolCalls, filesCreated, ieStatus) {
  const scores = {
    ieCallRate: 0,
    fileAvoidance: 100,  // Start at 100, deduct for violations
    specCompleteness: 0,
    // ... other dimensions
  }
  
  // Check required calls
  if (scenario.expectedBehavior.mustCall) {
    const called = scenario.expectedBehavior.mustCall
      .filter(t => toolCalls.some(tc => tc.includes(t)))
    scores.ieCallRate = (called.length / expected.mustCall.length) * 100
  }
  
  // Check forbidden files
  if (scenario.expectedBehavior.mustNotCreate) {
    const created = filesCreated.filter(f => 
      expected.mustNotCreate.some(pattern => f.match(pattern))
    )
    if (created.length > 0) scores.fileAvoidance = 0
  }
  
  // Check spec quality
  const spec = ieStatus?.current_task?.spec || ""
  if (scenario.expectedBehavior.specMustContain) {
    const found = expected.specMustContain
      .filter(term => new RegExp(`##\\s*${term}`, "i").test(spec))
    scores.specCompleteness = (found.length / expected.length) * 100
  }
  
  // Calculate weighted total
  const totalScore = 
    scores.ieCallRate * 0.18 +
    scores.fileAvoidance * 0.10 +
    scores.specCompleteness * 0.12 +
    // ... other dimensions
  
  return { scores, totalScore, passed: failures.length === 0 }
}
```

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

# 5. Record results:
#    - Which tools were called?
#    - Any forbidden files created?
#    - Check ie status for task structure
ie status --format json
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

### Aggregation

```typescript
function aggregateResults(results) {
  const byVariant = groupBy(results, r => r.promptVariant)
  
  return Object.entries(byVariant).map(([variant, scenarios]) => ({
    variant,
    avgScore: mean(scenarios.map(s => s.totalScore)),
    passRate: scenarios.filter(s => s.passed).length / scenarios.length,
    detailedScores: {
      ieCallRate: mean(scenarios.map(s => s.scores.ieCallRate)),
      fileAvoidance: mean(scenarios.map(s => s.scores.fileAvoidance)),
      // ... other dimensions
    }
  })).sort((a, b) => b.avgScore - a.avgScore)
}
```

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

## Top Variant: enhanced

**Score Breakdown:**
- IE Call Rate: 100%
- File Avoidance: 100%
- Spec Completeness: 95%
- Decision Logging: 90%
- Structure Quality: 85%

======================================================================
RECOMMENDED: enhanced
======================================================================
```

---

## Step 6: Iterate

### Improvement Patterns

| Problem | Diagnosis | Solution |
|---------|-----------|----------|
| Low IE Call Rate | AI doesn't know when to use tools | Add explicit triggers: "When X, use ie_plan" |
| File Creation | AI creates forbidden files | Add FORBIDDEN section with consequences |
| Poor Spec | Missing Goal/Approach | Add template with required sections |
| No Children | Doesn't break down tasks | Add example with nested children structure |
| Short Events | Brief decision logs | Add rich markdown event example |

### A/B Testing Protocol

1. **Baseline**: Run all scenarios with current prompt
2. **Hypothesis**: "Adding explicit examples will improve spec quality"
3. **Variant**: Create new prompt with examples
4. **Test**: Run same scenarios with variant
5. **Compare**: Statistical comparison of scores
6. **Decide**: Adopt if significantly better (>5% improvement)
7. **Repeat**: Continue iterating

---

## Quick Reference

### Test Commands

```bash
# Initialize test session
export IE_SESSION_ID="test-$(date +%s)"
ie init

# Check results
ie status --format json

# Find created files
find .opencode/plan -name "*.md" 2>/dev/null

# Reset for next test
rm -rf .ie && ie init
```

### Scoring Thresholds

| Grade | Score Range | Interpretation |
|-------|-------------|----------------|
| A | 90-100 | Excellent - ready for production |
| B | 80-89 | Good - minor improvements needed |
| C | 70-79 | Fair - significant issues |
| D | 60-69 | Poor - major revision needed |
| F | <60 | Failing - complete rethink required |

---

## Files Reference

```
~/.config/opencode/plugins/ie-test/
├── test-scenarios.ts     # Scenario definitions
├── prompt-variants.ts    # Prompt variations to test
├── scoring.ts           # Scoring logic
├── automated-test.ts    # Automated test runner
├── analyze-results.ts   # Results analysis
├── manual-test-guide.md # Manual testing guide
└── results/             # Test results by timestamp
```

---

## Best Practices

1. **Test in isolation** - Fresh session per scenario
2. **Multiple runs** - Test each variant 2-3 times for consistency
3. **Weight by importance** - Critical behaviors get higher weight
4. **Track regressions** - Save baseline scores for comparison
5. **Document learnings** - Record what worked and why

---

## When to Use This Framework

- Launching new AI tools
- Optimizing existing tool adoption
- Debugging inconsistent AI behavior
- Comparing prompt strategies
- Measuring prompt improvement over time

---

*Systematic testing beats intuition. Measure, iterate, improve.*
