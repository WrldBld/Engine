# Next Steps - Batch 4 Cleanup

## What Was Done

All code changes have been completed:

1. **Test Mock Consolidation** ✓
   - Consolidated 6 duplicate `MockLlm` structs into one shared mock
   - File: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_service.rs`

2. **Unsafe Code Fix** ✓
   - Removed unsafe `unwrap()` on session_id
   - File: `/home/otto/repos/WrldBldr/Engine/src/application/services/llm_queue_service.rs`

3. **Module Declaration Updates** ✓
   - Removed references to dead code modules
   - Files: `src/domain/mod.rs`, `src/domain/aggregates/mod.rs`

## What You Need to Do

Run the cleanup script to delete the dead code files and verify everything works:

```bash
cd /home/otto/repos/WrldBldr/Engine
chmod +x cleanup_batch4.sh
./cleanup_batch4.sh
```

This script will:
1. Delete the 3 dead code files
2. Remove empty directories
3. Run `cargo check` to verify compilation
4. Run `cargo test --lib` to verify tests pass

## Manual Alternative

If you prefer to run commands manually:

```bash
cd /home/otto/repos/WrldBldr/Engine

# Delete dead code files
rm -f src/domain/events/domain_events.rs
rm -f src/domain/events/mod.rs
rm -f src/domain/aggregates/world_aggregate.rs

# Remove empty directories
rmdir src/domain/events
rmdir src/domain/aggregates

# Verify compilation
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo check"

# Run tests
nix-shell -p rustc cargo gcc pkg-config openssl.dev --run "cargo test --lib"
```

## Expected Results

- ✓ No compilation errors
- ✓ All tests pass
- ✓ ~470 lines of dead code removed
- ✓ ~120 lines of duplicate test mocks consolidated
- ✓ No more potential panics from unwrap

## Files to Review

See `BATCH4_CHANGES.md` for a detailed summary of all changes made.

## Cleanup

After verifying everything works, you can delete:
- `cleanup_batch4.sh`
- `BATCH4_CHANGES.md`
- `NEXT_STEPS.md` (this file)

Or keep them for reference.
