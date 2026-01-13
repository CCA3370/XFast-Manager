# GitHub Actions CI/CD Workflows

## Build Modes

This project supports two build modes optimized for different use cases:

### 🚀 Debug Build (Fast - for testing)

**Trigger**: Include `dbuild` in your commit message

**Example**:
```bash
git commit -m "fix: password detection dbuild"
git push
```

**Optimizations**:
- ✅ **Incremental compilation** enabled (reuses previous build artifacts)
- ✅ **Rust cache** with aggressive caching strategy
- ✅ **npm cache** for faster dependency installation
- ✅ **Debug profile** (opt-level=1, no LTO, 256 codegen units)
- ✅ **Dynamic linking** for faster compilation
- ✅ **Minimal debug symbols** (debug=1)
- ⚡ **~3-5x faster** than release builds

**Output**: `XFastInstall-debug.exe` (larger file, faster runtime for debugging)

**Retention**: 7 days

**Use case**: Quick iteration, testing bug fixes, CI validation

---

### 📦 Release Build (Optimized - for production)

**Trigger**: Any commit without `dbuild` in the message

**Example**:
```bash
git commit -m "feat: add new feature"
git push
```

**Optimizations**:
- ✅ **Full optimization** (opt-level=3, fat LTO, 1 codegen unit)
- ✅ **Symbol stripping** for smaller binary
- ✅ **Panic=abort** for smaller binary
- ✅ **Rust cache** with conservative strategy
- 📦 **Smallest binary size** and **best runtime performance**

**Output**: `XFastInstall.exe` (smaller file, optimized for end users)

**Retention**: 90 days (default)

**Use case**: Production releases, performance testing, distribution

---

## Build Time Comparison

| Mode | First Build | Incremental Build | Binary Size | Performance |
|------|-------------|-------------------|-------------|-------------|
| Debug | ~8-12 min | ~2-4 min | ~50-80 MB | Good |
| Release | ~15-25 min | ~10-15 min | ~8-15 MB | Excellent |

*Times are approximate and depend on code changes*

---

## Cache Strategy

### Debug Mode Cache
- **Prefix**: `v1-rust-debug`
- **Shared key**: `debug-build`
- **Cache on failure**: Yes (to speed up retry builds)
- **Save always**: Yes

### Release Mode Cache
- **Prefix**: `v1-rust-release`
- **Shared key**: `release-build`
- **Cache on failure**: No (avoid caching broken builds)

---

## Environment Variables

### Debug Build
```bash
CARGO_INCREMENTAL=1                    # Enable incremental compilation
TAURI_BUILD_PROFILE=debug              # Use debug profile
CARGO_PROFILE_DEV_OPT_LEVEL=1          # Minimal optimization
CARGO_PROFILE_DEV_CODEGEN_UNITS=256    # Max parallelism
CARGO_PROFILE_DEV_LTO=off              # No link-time optimization
CARGO_PROFILE_DEV_DEBUG=1              # Minimal debug info
RUSTFLAGS=-C prefer-dynamic            # Dynamic linking
```

### Release Build
```bash
CARGO_INCREMENTAL=0                         # Disable for reproducibility
CARGO_PROFILE_RELEASE_LTO=fat               # Full LTO
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1       # Single unit for max optimization
CARGO_PROFILE_RELEASE_OPT_LEVEL=3           # Maximum optimization
CARGO_PROFILE_RELEASE_STRIP=symbols         # Strip debug symbols
CARGO_PROFILE_RELEASE_PANIC=abort           # Smaller binary
```

---

## Tips for Faster CI Builds

1. **Use `dbuild` for rapid testing**:
   ```bash
   git commit -m "test: verify fix dbuild"
   ```

2. **Batch multiple changes** before pushing to reduce CI runs

3. **Use draft PRs** to prevent automatic CI triggers

4. **Cancel redundant builds** when pushing multiple commits quickly

5. **Local testing first**:
   ```bash
   # Test locally before pushing
   npm run tauri:dev
   cargo test
   ```

---

## Troubleshooting

### Cache Issues

If builds are slower than expected, clear the cache:

1. Go to **Actions** → **Caches**
2. Delete caches with prefix `v1-rust-debug` or `v1-rust-release`
3. Push a new commit to rebuild cache

### Debug Build Not Triggered

Check that your commit message contains `dbuild`:
```bash
# ✅ Correct
git commit -m "fix: issue dbuild"

# ❌ Wrong
git commit -m "fix: issue"
```

### Build Artifacts Not Found

- **Debug builds**: Check `target/debug/XFastInstall-debug.exe`
- **Release builds**: Check `target/release/XFastInstall.exe`

---

## Manual Workflow Dispatch

You can manually trigger builds from the GitHub Actions UI:

1. Go to **Actions** → **Build Tauri (Windows Portable)**
2. Click **Run workflow**
3. Select branch
4. Click **Run workflow**

The build mode will be determined by the latest commit message on that branch.
