#!/bin/bash
#
# Split plan.rs into modular structure
#
# Structure:
#   src/plan.rs (2343 lines) ->
#   src/plan/
#     ├── models.rs      (~400 lines: data structures + tests)
#     ├── helpers.rs     (~550 lines: helper functions + tests)
#     ├── executor.rs    (~1350 lines: PlanExecutor + tests)
#     └── mod.rs         (~50 lines: public exports)

set -e

PLAN_FILE="src/plan.rs"
PLAN_DIR="src/plan"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Splitting plan.rs into modular structure"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if plan.rs exists
if [ ! -f "$PLAN_FILE" ]; then
    echo "❌ Error: $PLAN_FILE not found"
    exit 1
fi

# Create plan directory if it doesn't exist
mkdir -p "$PLAN_DIR"

# ============================================================================
# Step 1: Extract models.rs (lines 1-193 + tests 790-883)
# ============================================================================
echo ""
echo "📦 Step 1: Extracting models.rs (data structures + tests)..."

cat > "$PLAN_DIR/models.rs" << 'EOF'
//! Data models for declarative task management

use serde::{Deserialize, Serialize};

EOF

# Extract models and their impls (lines 7-193)
sed -n '7,193p' "$PLAN_FILE" >> "$PLAN_DIR/models.rs"

# Extract model tests (lines 790-883)
echo "" >> "$PLAN_DIR/models.rs"
echo "#[cfg(test)]" >> "$PLAN_DIR/models.rs"
echo "mod tests {" >> "$PLAN_DIR/models.rs"
echo "    use super::*;" >> "$PLAN_DIR/models.rs"
echo "" >> "$PLAN_DIR/models.rs"
sed -n '794,883p' "$PLAN_FILE" | sed 's/^    //' >> "$PLAN_DIR/models.rs"
echo "}" >> "$PLAN_DIR/models.rs"

echo "✅ Created $PLAN_DIR/models.rs ($(wc -l < $PLAN_DIR/models.rs) lines)"

# ============================================================================
# Step 2: Extract helpers.rs (lines 195-325 + tests 884-1521)
# ============================================================================
echo ""
echo "📦 Step 2: Extracting helpers.rs (helper functions + tests)..."

cat > "$PLAN_DIR/helpers.rs" << 'EOF'
//! Helper functions for task tree manipulation

use super::models::{FlatTask, Operation, PriorityValue, TaskStatus, TaskTree};
use std::collections::{HashMap, HashSet};

EOF

# Extract helper functions (lines 195-325)
sed -n '195,325p' "$PLAN_FILE" >> "$PLAN_DIR/helpers.rs"

# Extract helper tests (lines 965-1521)
echo "" >> "$PLAN_DIR/helpers.rs"
echo "#[cfg(test)]" >> "$PLAN_DIR/helpers.rs"
echo "mod tests {" >> "$PLAN_DIR/helpers.rs"
echo "    use super::*;" >> "$PLAN_DIR/helpers.rs"
echo "" >> "$PLAN_DIR/helpers.rs"
sed -n '969,1521p' "$PLAN_FILE" | sed 's/^    //' >> "$PLAN_DIR/helpers.rs"
echo "}" >> "$PLAN_DIR/helpers.rs"

echo "✅ Created $PLAN_DIR/helpers.rs ($(wc -l < $PLAN_DIR/helpers.rs) lines)"

# ============================================================================
# Step 3: Extract executor.rs (lines 327-785 + tests 1522-2286)
# ============================================================================
echo ""
echo "📦 Step 3: Extracting executor.rs (PlanExecutor + tests)..."

cat > "$PLAN_DIR/executor.rs" << 'EOF'
//! Plan execution engine

use super::helpers::{classify_operations, extract_all_names, find_duplicate_names, flatten_task_tree};
use super::models::{FlatTask, Operation, PlanRequest, PlanResult, PriorityValue, TaskStatus, TaskTree};
use crate::error::{IntentError, Result};
use crate::tasks::TaskManager;
use crate::workspace::WorkspaceManager;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

EOF

# Extract PlanExecutor (lines 327-785)
sed -n '327,785p' "$PLAN_FILE" >> "$PLAN_DIR/executor.rs"

# Extract executor tests (lines 1522-2286)
echo "" >> "$PLAN_DIR/executor.rs"
echo "#[cfg(test)]" >> "$PLAN_DIR/executor.rs"
echo "mod tests {" >> "$PLAN_DIR/executor.rs"
echo "    use super::*;" >> "$PLAN_DIR/executor.rs"
echo "    use crate::test_utils::test_helpers::TestContext;" >> "$PLAN_DIR/executor.rs"
echo "" >> "$PLAN_DIR/executor.rs"
sed -n '1526,2286p' "$PLAN_FILE" | sed 's/^    //' >> "$PLAN_DIR/executor.rs"
echo "}" >> "$PLAN_DIR/executor.rs"

echo "✅ Created $PLAN_DIR/executor.rs ($(wc -l < $PLAN_DIR/executor.rs) lines)"

# ============================================================================
# Step 4: Extract dataflow_tests (lines 2287-2343)
# ============================================================================
echo ""
echo "📦 Step 4: Appending dataflow_tests to executor.rs..."

echo "" >> "$PLAN_DIR/executor.rs"
echo "#[cfg(test)]" >> "$PLAN_DIR/executor.rs"
echo "mod dataflow_tests {" >> "$PLAN_DIR/executor.rs"
echo "    use super::*;" >> "$PLAN_DIR/executor.rs"
echo "    use crate::tasks::TaskManager;" >> "$PLAN_DIR/executor.rs"
echo "    use crate::test_utils::test_helpers::TestContext;" >> "$PLAN_DIR/executor.rs"
echo "" >> "$PLAN_DIR/executor.rs"
sed -n '2294,2343p' "$PLAN_FILE" | sed 's/^    //' >> "$PLAN_DIR/executor.rs"
echo "}" >> "$PLAN_DIR/executor.rs"

echo "✅ Appended dataflow_tests to executor.rs ($(wc -l < $PLAN_DIR/executor.rs) lines total)"

# ============================================================================
# Step 5: Create mod.rs (module exports)
# ============================================================================
echo ""
echo "📦 Step 5: Creating mod.rs (module exports)..."

cat > "$PLAN_DIR/mod.rs" << 'EOF'
//! Plan Interface - Declarative Task Management
//!
//! Provides a declarative API for creating and updating task structures,
//! inspired by TodoWrite pattern. Simplifies complex operations into
//! single atomic calls.

mod models;
mod helpers;
mod executor;

// Re-export public types
pub use models::{
    PlanRequest,
    PlanResult,
    TaskTree,
    TaskStatus,
    PriorityValue,
    FlatTask,
    Operation,
};

pub use helpers::{
    extract_all_names,
    flatten_task_tree,
    classify_operations,
    find_duplicate_names,
};

pub use executor::PlanExecutor;
EOF

echo "✅ Created $PLAN_DIR/mod.rs ($(wc -l < $PLAN_DIR/mod.rs) lines)"

# ============================================================================
# Step 6: Update src/lib.rs
# ============================================================================
echo ""
echo "📦 Step 6: Updating src/lib.rs..."

# Check if plan module declaration exists
if grep -q "^pub mod plan;" src/lib.rs; then
    echo "✅ src/lib.rs already has 'pub mod plan;' declaration"
else
    echo "⚠️  Please manually add 'pub mod plan;' to src/lib.rs"
fi

# ============================================================================
# Step 7: Backup and remove original plan.rs
# ============================================================================
echo ""
echo "📦 Step 7: Backing up original plan.rs..."

cp "$PLAN_FILE" "${PLAN_FILE}.backup"
echo "✅ Created backup: ${PLAN_FILE}.backup"

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Split completed successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Created files:"
echo "  📄 $PLAN_DIR/models.rs    ($(wc -l < $PLAN_DIR/models.rs) lines)"
echo "  📄 $PLAN_DIR/helpers.rs   ($(wc -l < $PLAN_DIR/helpers.rs) lines)"
echo "  📄 $PLAN_DIR/executor.rs  ($(wc -l < $PLAN_DIR/executor.rs) lines)"
echo "  📄 $PLAN_DIR/mod.rs       ($(wc -l < $PLAN_DIR/mod.rs) lines)"
echo ""
echo "Next steps:"
echo "  1. Remove src/plan.rs:     rm src/plan.rs"
echo "  2. Verify compilation:     cargo build"
echo "  3. Run tests:              cargo test plan::"
echo "  4. If tests pass:          rm src/plan.rs.backup"
echo ""
