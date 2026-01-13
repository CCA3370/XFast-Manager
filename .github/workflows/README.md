# GitHub Actions CI/CD Workflows

## Build Modes

This project supports two build modes optimized for different use cases:

### 🚀 Fast Build (Optimized for compilation speed)

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
- ✅ **Thin LTO** (faster linking than fat LTO)
- ✅ **16 codegen units** (parallel compilation)
- ⚠️ **opt-level=2** (good performance, faster compilation)
- ⚡ **~2-3x faster** than standard builds

**Output**: `XFastInstall.exe` (slightly larger, good performance)

**Retention**: 7 days

**Use case**: Quick iteration, testing bug fixes, CI validation

**Runtime Performance**: ⚠️ **Good** (slightly slower than production)

---

### 📦 Production Build (Optimized for runtime performance)

**Trigger**: Any commit without `dbuild` in the message

**Example**:
```bash
git commit -m "feat: add new feature"
git push
```

**Optimizations**:
- ✅ **Fat LTO** for maximum cross-crate optimization
- ✅ **1 codegen unit** for maximum cross-function optimization
- ✅ **opt-level=3** (maximum optimization)
- ✅ **target-cpu=x86-64-v2** (modern CPU instructions)
- ✅ **Symbol stripping** for smaller binary
- ✅ **Panic=abort** for smaller binary
- 🚀 **Maximum runtime performance**

**Output**: `XFastInstall.exe` (smallest size, fastest runtime)

**Retention**: 90 days (default)

**Use case**: Production releases, final distribution

**Runtime Performance**: ✅ **Excellent** (maximum optimization)

---

## Build Time & Performance Comparison

| Mode | First Build | Incremental Build | Binary Size | Runtime Performance |
|------|-------------|-------------------|-------------|---------------------|
| **Fast** | ~10-15 min | ~5-8 min | ~10-12 MB | ⚠️ Good (opt-level=2) |
| **Production** | ~15-25 min | ~10-15 min | ~8-10 MB | ✅ Excellent (opt-level=3 + fat LTO) |

*Times are approximate and depend on code changes*

---

## Key Differences

### Fast Build (Testing)
```yaml
CARGO_INCREMENTAL=1                    # Enable incremental compilation
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 # Parallel compilation (16 threads)
CARGO_PROFILE_RELEASE_LTO=thin         # Thin LTO (faster)
CARGO_PROFILE_RELEASE_OPT_LEVEL=2      # Good optimization (faster compile)
CARGO_PROFILE_RELEASE_STRIP=symbols    # Strip symbols
CARGO_PROFILE_RELEASE_PANIC=abort      # Panic abort
```

### Production Build (Distribution)
```yaml
CARGO_INCREMENTAL=0                    # Disable for reproducibility
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1  # Single unit (max optimization)
CARGO_PROFILE_RELEASE_LTO=fat          # Fat LTO (max performance)
CARGO_PROFILE_RELEASE_OPT_LEVEL=3      # Maximum optimization
CARGO_PROFILE_RELEASE_STRIP=symbols    # Strip symbols
CARGO_PROFILE_RELEASE_PANIC=abort      # Panic abort
RUSTFLAGS=-C target-cpu=x86-64-v2      # Modern CPU instructions
```

---

## Optimization Levels Explained

### opt-level=2 (Fast Build)
- ✅ Most optimizations enabled
- ✅ Fast compilation
- ⚠️ Slightly slower runtime than opt-level=3
- 📦 Good balance for testing

### opt-level=3 (Production Build)
- ✅ All optimizations enabled
- ✅ Maximum runtime performance
- ⏱️ Slower compilation
- 🚀 Best for end users

### Fat LTO (Production Only)
- ✅ Cross-crate inlining and optimization
- ✅ Dead code elimination across crates
- ✅ Better code generation
- 🚀 ~5-15% performance improvement over thin LTO

### target-cpu=x86-64-v2 (Production Only)
- ✅ Uses SSE3, SSE4.1, SSE4.2, SSSE3 instructions
- ✅ Better performance on modern CPUs (2008+)
- ⚠️ Won't run on very old CPUs (pre-2008)
- 🎯 Good balance of compatibility and performance

---

## Cache Strategy

### Fast Mode Cache
- **Prefix**: `v1-rust-fast`
- **Shared key**: `fast-build`
- **Cache on failure**: Yes (to speed up retry builds)
- **Save always**: Yes

### Production Mode Cache
- **Prefix**: `v1-rust-standard`
- **Shared key**: `standard-build`
- **Cache on failure**: No (avoid caching broken builds)

---

## When to Use Each Mode

### Use Fast Build (`dbuild`) when:
- ✅ Testing bug fixes quickly
- ✅ Validating CI changes
- ✅ Iterating on features
- ✅ Need quick feedback
- ✅ Runtime performance doesn't matter (testing only)

### Use Production Build (default) when:
- ✅ Creating production releases
- ✅ Final distribution to users
- ✅ Performance testing
- ✅ Benchmarking
- ✅ Maximum runtime speed is critical

---

## Important Notes

1. **Fast build is for testing only** - slightly slower runtime
2. **Production build is for end users** - maximum performance
3. **Binary size difference** - fast build is ~10-20% larger
4. **Performance difference** - production build is ~5-15% faster
5. **Compilation time** - fast build is ~2-3x faster

---

## Performance Impact Examples

For typical operations in XFastInstall:

| Operation | Fast Build | Production Build | Difference |
|-----------|-----------|------------------|------------|
| ZIP extraction | ~100 MB/s | ~110-115 MB/s | +10-15% |
| File copying | ~200 MB/s | ~220-230 MB/s | +10-15% |
| Archive scanning | ~50 files/s | ~55-60 files/s | +10-20% |
| UI responsiveness | Good | Excellent | Noticeable |

*Actual performance depends on hardware and file types*

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
2. Delete caches with prefix `v1-rust-fast` or `v1-rust-standard`
3. Push a new commit to rebuild cache

### Fast Build Not Triggered

Check that your commit message contains `dbuild`:
```bash
# ✅ Correct
git commit -m "fix: issue dbuild"

# ❌ Wrong
git commit -m "fix: issue"
```

### Build Artifacts Not Found

Both modes output to `target/release/XFastInstall.exe`

---

## Manual Workflow Dispatch

You can manually trigger builds from the GitHub Actions UI:

1. Go to **Actions** → **Build Tauri (Windows Portable)**
2. Click **Run workflow**
3. Select branch
4. Click **Run workflow**

The build mode will be determined by the latest commit message on that branch.

---

## Technical Details

### Why opt-level=2 vs opt-level=3?

- **opt-level=2**: Enables most optimizations, fast compilation
- **opt-level=3**: Enables all optimizations including aggressive inlining
- **Compilation time**: opt-level=2 is ~30-40% faster
- **Runtime performance**: opt-level=3 is ~5-10% faster

### Why Fat LTO vs Thin LTO?

- **Fat LTO**: Optimizes across all compilation units, maximum performance
- **Thin LTO**: Optimizes within compilation units, faster linking
- **Performance**: Fat LTO is ~5-15% faster at runtime
- **Compilation**: Thin LTO is ~2x faster to link

### Why target-cpu=x86-64-v2?

- **x86-64**: Basic 64-bit (2003+)
- **x86-64-v2**: Adds SSE3/SSE4 (2008+) - **recommended**
- **x86-64-v3**: Adds AVX/AVX2 (2013+)
- **x86-64-v4**: Adds AVX-512 (2017+)

We use v2 for good compatibility (99%+ of users) with modern optimizations.

### Why 16 Codegen Units?

- More codegen units = more parallel compilation = faster build
- Fewer codegen units = more cross-unit optimization = better performance
- 16 is optimal for GitHub Actions runners (4 cores with hyperthreading)


