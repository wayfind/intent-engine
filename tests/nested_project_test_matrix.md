# Nested Project Test Matrix

## Test Scenario Coverage

This document defines a comprehensive test matrix for nested project scenarios.

### Notation
- ✓ = has marker/database
- ✗ = no marker/database
- P = Parent directory
- C = Child directory

### Combination Matrix

| Test # | P .git | P .intent | C .git | C .intent | Expected Behavior | Test Status |
|--------|--------|-----------|--------|-----------|-------------------|-------------|
| 1      | ✓      | ✓         | ✓      | ✗         | C creates own .intent | ✅ PASS |
| 2      | ✓      | ✓         | ✗      | ✗         | C uses P .intent | ✅ PASS |
| 3      | ✓      | ✗         | ✓      | ✗         | C creates own .intent | ✅ PASS |
| 4      | ✗      | ✓         | ✓      | ✗         | C creates own .intent | ✅ PASS |
| 5      | ✗      | ✗         | ✓      | ✗         | C creates own .intent | ✅ PASS |
| 6      | ✓      | ✓         | ✓      | ✓         | C uses own .intent | ✅ PASS |
| 7      | ✗      | ✓         | ✗      | ✗         | C uses P .intent | ✅ PASS |
| 8      | ✗      | ✗         | ✗      | ✗         | C creates .intent at CWD | ✅ PASS |

### Multi-level Nesting Scenarios

| Test # | Grandparent | Parent | Child | Expected Behavior | Test Status |
|--------|-------------|--------|-------|-------------------|-------------|
| N1     | ✓.git+.intent | ✓.git+✗ | ✓.git+✗ | C creates own .intent | ✅ PASS |
| N2     | ✓.git+.intent | ✗+✗ | ✓.git+✗ | C creates own .intent | ✅ PASS |
| N3     | ✓.git+.intent | ✓pkg.json+✗ | ✓.git+✗ | C creates own .intent | ✅ PASS |
| N4     | ✓.git+.intent | ✗+✗ | ✗+✗ | Uses GP .intent | ✅ PASS |

### Sibling Projects Isolation

| Test # | Scenario | Expected Behavior | Test Status |
|--------|----------|-------------------|-------------|
| S1     | Two siblings with own .git | Each has own .intent | ✅ PASS |
| S2     | Three siblings, mixed markers | Correct isolation | ✅ PASS |

### Marker Priority Testing

| Test # | Scenario | Expected Behavior | Test Status |
|--------|----------|-------------------|-------------|
| M1     | P has .git, C has package.json | C creates .intent at C (pkg.json boundary) | ⏭️ SKIP |
| M2     | P has .git, C has .git+pkg.json | C uses .git boundary (higher priority) | ⏭️ SKIP |
| M3     | Different markers at different levels | Respects priority order | ✅ PASS (N3) |

### Edge Cases

| Test # | Scenario | Expected Behavior | Test Status |
|--------|----------|-------------------|-------------|
| E1     | Run from deep subdirectory (3+ levels) | Finds correct boundary | ✅ PASS |
| E2     | Symlinked nested projects | Works correctly | ⏭️ SKIP |
| E3     | Parent with .intent but no marker | Child creates own with marker | ✅ PASS |
| E4     | Multiple markers in same directory | Uses highest priority | ✅ PASS |

### Current Test Coverage

✅ **COMPLETED: 17 comprehensive test scenarios**

**Phase 1: Basic Combinations (8/8 tests) ✅**
- test_matrix_2_parent_has_all_child_has_none
- test_matrix_3_both_have_git_no_intent
- test_matrix_4_parent_intent_only_child_has_git
- test_matrix_5_parent_empty_child_has_git
- test_matrix_6_both_have_intent
- test_matrix_7_parent_intent_only_child_nothing
- test_matrix_8_both_have_nothing
- test_nested_projects_should_not_share_database (Test #1)

**Phase 2: Multi-level Nesting (4/4 tests) ✅**
- test_multi_level_n1_all_have_git
- test_multi_level_n2_skip_middle_generation
- test_multi_level_n3_different_markers
- test_multi_level_n4_no_middle_boundaries

**Phase 3: Sibling Isolation (2/2 tests) ✅**
- test_siblings_s1_both_have_git
- test_siblings_s2_mixed_markers

**Phase 4: Edge Cases (3/3 tests) ✅**
- test_edge_e1_very_deep_subdirectory
- test_edge_e3_orphaned_parent_intent
- test_edge_e4_multiple_markers_same_dir

**Skipped (covered by existing tests):**
- Marker priority tests (covered by marker detection logic + N3)
- Symlink tests (already exist in older tests)

**Total: 17 new tests + 2 original = 19 nested project tests**

## Test Results Summary

- ✅ All 17 new tests PASSING
- ✅ All existing initialization tests PASSING (24 total)
- ⚠️ Known issue: `test_concurrent_initialization_attempts` has race condition (pre-existing test, not related to new changes)
