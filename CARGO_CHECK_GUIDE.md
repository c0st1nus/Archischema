# Cargo Check Guide for ArchiSchema

## Quick Summary

**ArchiSchema** is a multi-feature Rust project with:
- **Library** (`src/lib.rs`) - Core code compiled to WASM (browser) AND SSR (server)
- **Binary** (`src/main.rs`) - Axum server (SSR only)

### Features
- `default = ["ssr"]` - Server-side rendering (Axum, Tokio, SQLx)
- `hydrate` - WebAssembly (browser: web-sys, wasm-bindgen)

---

## Why cargo leptos watch finds errors but go-task check doesn't

### The Root Cause: Different Compilation Modes

```
cargo leptos watch compiles:
│
├─ WASM build (no "ssr" feature)
│  └─ #[cfg(not(feature = "ssr"))] code is ACTIVE
│  └─ Requires: wasm-bindgen, web-sys, gloo-timers
│  └─ ❌ ERRORS in this section show up here!
│
└─ SSR build (with "ssr" feature)
   └─ #[cfg(feature = "ssr")] code is ACTIVE
   └─ Requires: axum, tokio, sqlx

go-task check compiles:
│
├─ cargo check --all-features
│  └─ Compiles with ssr=true AND hydrate=true
│
└─ cargo clippy --lib --all-features
   └─ Checks lib with all features enabled
   └─ ✓ Both cfg blocks are checked simultaneously
```

### The Problem We Faced

In `activity_tracker.rs`, we had:

```rust
// ❌ WRONG: Import at module level
use wasm_bindgen::JsCast;  // Only needed for WASM!

#[cfg(not(feature = "ssr"))]
{
    // This code uses JsCast
    document.add_event_listener_with_callback(
        "mousemove", 
        mousemove.as_ref().unchecked_ref()  // Error: JsCast not imported here!
    );
}
```

When `ssr=true` (in `go-task check`), this code isn't compiled, so the missing import isn't caught.
When `ssr=false` (in `cargo leptos watch`), the code IS compiled and the error appears.

### The Solution

```rust
// ✅ CORRECT: Import inside cfg block
#[cfg(not(feature = "ssr"))]
{
    use wasm_bindgen::JsCast;  // Only imported when needed
    
    // This code works fine
    document.add_event_listener_with_callback(
        "mousemove", 
        mousemove.as_ref().unchecked_ref()  // ✓ JsCast is imported
    );
}
```

---

## What is the --lib flag? Why do we need it?

### Quick Answer

`--lib` tells cargo to **only check the library** (`src/lib.rs`) and skip the binary (`src/main.rs`).

### Why is this useful?

| Without --lib | With --lib |
|---|---|
| Compiles lib + binary | Compiles lib only |
| Checks: web-sys, axum, tokio, sqlx | Checks: web-sys only |
| Time: ~29 seconds | Time: 0.3 seconds |
| Useful for: Full project validation | Useful for: Fast feedback during development |

### Real-world analogy

Imagine you're developing an engine for a car:
- **Without --lib**: You assemble the entire car (engine + chassis + wheels) and test it (slow)
- **With --lib**: You just test the engine on a test bench (fast)

In our project:
- **lib** = the engine (used everywhere: browser + server)
- **main.rs** = the full car (server only)

### Practical examples

```bash
# Fast check while developing lib
cargo check --lib
# Checks: src/lib.rs with default features (ssr)
# Time: 0.3 seconds

# Check that lib works with all features
cargo check --lib --all-features
# Checks: src/lib.rs with ssr AND hydrate
# Time: 17 seconds (more deps)

# Check everything (lib + server)
cargo check
# Checks: src/lib.rs + src/main.rs with default features (ssr)
# Time: 29 seconds

# Check everything with all features
cargo check --all-features
# Checks: src/lib.rs + src/main.rs with ssr AND hydrate
# Time: 0.2 seconds (already cached)
```

---

## Complete Cargo Check Reference

### Command Comparison Table

| Command | What it checks | Features | Time | Use case |
|---------|---|---|---|---|
| `cargo check` | lib + binary | ssr | 29 sec | Full validation with default config |
| `cargo check --lib` | lib only | ssr | 0.3 sec | **Fast feedback during development** |
| `cargo check --all-features` | lib + binary | ssr+hydrate | 0.2 sec | Verify all features work together |
| `cargo check --lib --all-features` | lib only | ssr+hydrate | 17 sec | Verify lib works in all configs |
| `go-task check` | lib + clippy + tests | ssr+hydrate | 30+ sec | **Pre-commit validation** |

### Feature-specific checks

```bash
# Check that code works on server (SSR)
cargo check --lib --no-default-features --features ssr
# Activates: #[cfg(feature = "ssr")]
# Deactivates: #[cfg(not(feature = "ssr"))]

# Check that code works in browser (WASM/Hydrate)
cargo check --lib --no-default-features --features hydrate
# Activates: #[cfg(not(feature = "ssr"))]
# Deactivates: #[cfg(feature = "ssr")]

# Check that code works EVERYWHERE (both)
cargo check --lib --all-features
# Activates: both #[cfg] blocks
```

---

## Our go-task check explained

Located in `Taskfile.yml`:

```yaml
check:
  desc: Run pre-commit checks (check, fmt, clippy, test)
  cmds:
    - cargo check --all-features              # Check everything compiles
    - task: fmt                                # Format code
    - task: clippy                             # Run linter
    - task: test                               # Run tests

clippy:
  desc: Run clippy lints
  cmds:
    - cargo clippy --lib --all-features       # Only check lib (faster)

test:
  desc: Run tests
  cmds:
    - cargo test --lib --all-features         # Only run lib tests (faster)
```

Why `--lib` in clippy and test?
- **Faster feedback** - Don't need to compile server dependencies
- **Focused** - Library is what gets published, not main.rs
- **Comprehensive** - All features tested simultaneously

---

## Best practices for multi-feature projects

### ❌ DON'T do this

```rust
// Bad: Import at module level that's only used in cfg block
use wasm_bindgen::JsCast;
use leptos::task::spawn_local;
use gloo_timers::callback::Interval;

#[cfg(not(feature = "ssr"))]
{
    // Code that uses these imports
}

// When ssr=true: These imports are unused warnings
// When ssr=false: These imports are unavailable (optional dependencies)
```

### ✅ DO this instead

```rust
// Good: Imports inside cfg blocks

#[cfg(not(feature = "ssr"))]
{
    use wasm_bindgen::JsCast;
    use gloo_timers::callback::Interval;
    
    // Code that uses these imports
}

#[cfg(feature = "ssr")]
{
    use tokio::time::interval;
    use axum::extract::FromRef;
    
    // Different code for server
}
```

### For conditional features in Cargo.toml

```toml
[dependencies]
# Always required
serde = { version = "1.0", features = ["derive"] }

# Optional, only when feature is enabled
gloo-timers = { version = "0.3", optional = true }
web-sys = { version = "0.3", optional = true }
tokio = { version = "1", optional = true }

[features]
default = ["ssr"]
hydrate = ["dep:gloo-timers", "dep:web-sys"]
ssr = ["dep:tokio"]
```

---

## Recommended workflow

### During development (fastest feedback)

```bash
# For quick syntax checks
cargo check --lib

# For linting while coding
cargo clippy --lib

# Run tests for your changes
cargo test --lib
```

### Before committing

```bash
# Full pre-commit checks
go-task check
```

### Before pushing to CI

```bash
# Ensure all features work together
cargo check --all-features

# Full testing
cargo test --all-features
```

### Debugging feature issues

```bash
# If code breaks with a feature:
cargo check --lib --no-default-features --features the_broken_feature

# Check the specific cfg conditionals:
grep -r "#\[cfg" src/ui/activity_tracker.rs
```

---

## Troubleshooting

### "Unresolved import X in cfg block"

**Problem**: Import is at module level but used in cfg-specific code

**Solution**: Move import inside the cfg block

```rust
// Before (wrong)
use some_optional_crate::SomeType;

#[cfg(not(feature = "ssr"))]
{
    // Uses SomeType
}

// After (correct)
#[cfg(not(feature = "ssr"))]
{
    use some_optional_crate::SomeType;
    // Uses SomeType
}
```

### "Unused import warning in ssr mode"

**Problem**: Import is needed for WASM but shows as unused in SSR

**Solution**: Move import inside `#[cfg(not(feature = "ssr"))]` block

```rust
// Before (wrong)
#[cfg(not(feature = "ssr"))]
use gloo_timers::callback::Interval;

// After (correct)
#[cfg(not(feature = "ssr"))]
{
    use gloo_timers::callback::Interval;
}
```

### "Works with go-task check but fails with cargo leptos watch"

**Problem**: Different feature configurations

**Solution**: Test with both configs:

```bash
# Test WASM build (what leptos watch does)
cargo check --lib --no-default-features --features hydrate

# Test SSR build (what leptos watch does)
cargo check --lib --no-default-features --features ssr

# Test all at once
cargo check --lib --all-features
```

---

## Summary

| Question | Answer |
|---|---|
| **Why errors in leptos watch but not go-task?** | Different feature configs. leptos compiles without ssr, go-task compiles with all features. |
| **What does --lib do?** | Skips binary compilation, checks only library. Much faster. |
| **When should I use --lib?** | During development for fast feedback. |
| **When should I not use --lib?** | Before committing (use `go-task check`), when testing binary specifically. |
| **Which command should I use normally?** | `cargo check --lib` for fast feedback, `go-task check` before commits. |