---
status: complete
priority: p2
issue_id: "005"
tags: [code-review, security, sdk]
dependencies: []
---

# Validate Cache Paths: Symlinks and Env Var

## Problem Statement

Two related security gaps in cache path handling:
1. No symlink check on cache file/directory — a symlink could redirect reads/writes
2. `SONOS_CACHE_DIR` accepts empty strings and relative paths, reintroducing the CWD fallback the plan explicitly removes

## Findings

- **Security-sentinel Finding 1 (Medium):** Symlink traversal on cache path. Use `fs::symlink_metadata()` instead of `fs::metadata()` and reject symlinks.
- **Security-sentinel Finding 2 (Medium):** `SONOS_CACHE_DIR=""` resolves to CWD. Require absolute, non-empty paths.

## Proposed Solutions

### Option 1: Add Validation Checks (Recommended)

**Approach:**
```rust
// In cache_dir()
.filter(|s| !s.is_empty())
.map(PathBuf::from)
.filter(|p| p.is_absolute())

// In load()
let meta = fs::symlink_metadata(&path).ok()?;
if meta.file_type().is_symlink() { return None; }
```

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] `SONOS_CACHE_DIR` rejects empty and relative paths
- [ ] `load()` uses `symlink_metadata` and rejects symlinks
- [ ] `save()` checks directory is not a symlink before writing

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
