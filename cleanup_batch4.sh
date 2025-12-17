#!/usr/bin/env bash
set -e

echo "=== Engine Test Mock Cleanup and Dead Code Removal (Batch 4) ==="
echo ""

# Navigate to Engine directory
cd /home/otto/repos/WrldBldr/Engine

echo "Step 1: Delete dead code files..."
rm -f src/domain/events/domain_events.rs
rm -f src/domain/events/mod.rs
rmdir src/domain/events 2>/dev/null || true
rm -f src/domain/aggregates/world_aggregate.rs
rmdir src/domain/aggregates 2>/dev/null || true
echo "  ✓ Dead code files removed"
echo ""

echo "Step 2: Running cargo check..."
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo check"
echo "  ✓ Compilation successful"
echo ""

echo "Step 3: Running cargo test..."
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo test --lib"
echo "  ✓ Tests passed"
echo ""

echo "=== All tasks completed successfully! ==="
