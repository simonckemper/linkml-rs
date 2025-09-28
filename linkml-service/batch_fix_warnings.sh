#!/bin/bash
# Batch fix LinkML warnings

echo "=== Fixing LinkML Service Warnings ==="

# 1. Fix unnecessary Result wrapping
echo "Step 1: Fixing unnecessary Result wrapping..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::unnecessary_wraps 2>/dev/null

# 2. Fix must_use attributes
echo "Step 2: Adding #[must_use] attributes..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::must_use_candidate 2>/dev/null

# 3. Fix redundant closures
echo "Step 3: Fixing redundant closures..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::redundant_closure_for_method_calls 2>/dev/null

# 4. Fix clone inefficiencies
echo "Step 4: Fixing clone inefficiencies..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::assigning_clones 2>/dev/null

# 5. Fix PathBuf to Path conversions
echo "Step 5: Fixing PathBuf to Path conversions..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::ptr_arg 2>/dev/null

# 6. Fix format string appends
echo "Step 6: Fixing format string appends..."
cargo clippy --fix --allow-dirty --allow-staged -p linkml-service -- -A clippy::all -W clippy::format_push_string 2>/dev/null

# Count remaining
echo ""
echo "Remaining warnings:"
cargo clippy -p linkml-service --no-deps 2>&1 | grep "warning:" | wc -l
