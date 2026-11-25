---
name: gpui-code-quality
description: Best practices and code quality guidelines for GPUI development. Use when refactoring, reviewing code, or ensuring adherence to GPUI idioms.
---

# GPUI Code Quality

This skill covers best practices and code quality guidelines for GPUI development.

## Variable Naming

### Use Full Words

**Rule**: Avoid abbreviations, use descriptive names

```rust
// ❌ WRONG
let q = VecDeque::new();
let cnt = 0;
let btn = Button::new();

// ✅ CORRECT
let queue = VecDeque::new();
let count = 0;
let button = Button::new();
```

### Meaningful Names

```rust
// ❌ WRONG
let x = calculate();
let temp = process(data);
let thing = Entity::new();

// ✅ CORRECT
let result = calculate();
let processed_data = process(data);
let user_profile = Entity::new();
```

## Async Context Variable Shadowing

Use variable shadowing to scope clones in async contexts:

```rust
// ✅ CORRECT - clear scoping
fn start_work(&mut self, cx: &mut Context<Self>) {
    let data = self.data.clone();
    let config = self.config.clone();
    
    cx.spawn(async move |this, cx| {
        let result = process(data, config).await;
        
        this.update(&mut *cx, |view, cx| {
            view.result = result;
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## File Organization

### Avoid mod.rs Files

**Rule**: Use `src/module.rs` instead of `src/module/mod.rs`

```
// ❌ WRONG
src/
  components/
    mod.rs      # Don't do this
    button.rs

// ✅ CORRECT
src/
  components.rs  # Module file
  components/
    button.rs
```

### Library Root Paths

For crates, specify library root in `Cargo.toml`:

```toml
[lib]
path = "src/my_lib.rs"  # Instead of default lib.rs
```

## Comments

### Only Explain "Why"

**Rule**: Don't write comments that summarize code

```rust
// ❌ WRONG - obvious from code
// Increment the counter
self.counter += 1;

// Set the title to "Hello"
self.title = "Hello".into();

// ✅ CORRECT - explains non-obvious reasoning
// We must update the cache before notifying observers
// to ensure they see the most recent data
self.update_cache();
cx.notify();

// ✅ ALSO CORRECT - explains tricky logic
// Use saturating_sub to avoid underflow when count is 0
self.count = self.count.saturating_sub(1);
```

## Testing

### Use GPUI Timers in Tests

```rust
// ❌ WRONG
#[gpui::test]
async fn test_delay(cx: &mut TestAppContext) {
    smol::Timer::after(Duration::from_secs(1)).await;
}

// ✅ CORRECT
#[gpui::test]
async fn test_delay(cx: &mut TestAppContext) {
    cx.background_executor.timer(Duration::from_secs(1)).await;
    cx.background_executor.run_until_parked();
}
```

## Build Guidelines

### Use Project Scripts

```bash
# ❌ WRONG
cargo clippy

# ✅ CORRECT (for Zed-based projects)
./script/clippy

# Always use -q flag for cargo
cargo build -q
cargo test -q
```

## Code Organization

### Prefer Existing Files

**Rule**: Add functionality to existing files unless it's a new logical component

```rust
// Instead of creating many small files:
// src/button_click.rs
// src/button_hover.rs
// src/button_focus.rs

// Keep related functionality together:
// src/button.rs (with all button behavior)
```

### Module Organization

```rust
// Good module structure
src/
  ui/
    components.rs   // Component definitions
    theme.rs        // Theme and colors
    styles.rs       // Shared styles
  data/
    models.rs       // Data models
    state.rs        // Application state
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Error Handling
- State Management
- Rendering
- Async Patterns

## Summary of Best Practices

| Rule | Rationale |
|------|-----------|
| Never use `unwrap()` | Prevents panics, forces explicit error handling |
| Never `let _ =` on fallible ops | Errors should be handled or logged |
| Use full variable names | Improves readability and maintainability |
| Avoid mod.rs files | Cleaner project structure |
| Call `cx.notify()` after mutations | Ensures UI updates |
| Use `WeakEntity` for back-refs | Prevents memory leaks |
| Keep `render()` pure | Avoids race conditions |
| Use `SharedString` for text | Reduces allocations |
| Detach or store tasks | Prevents cancelled work |
| Use GPUI timers in tests | Prevents test failures |
| Only comment "why", not "what" | Reduces noise, focuses on reasoning |
| Propagate errors to UI | Provides user feedback |
| Use `background_spawn()` for CPU work | Keeps UI responsive |

## Checklist for Code Review

- [ ] No `unwrap()` calls
- [ ] No silently discarded errors (`let _ =` on fallible ops)
- [ ] `cx.notify()` called after state changes
- [ ] Tasks are detached or stored
- [ ] Weak references used for back-pointers
- [ ] `SharedString` used for text
- [ ] No mutations in `render()`
- [ ] Descriptive variable names
- [ ] Comments explain "why", not "what"
- [ ] GPUI timers used in tests
- [ ] Errors propagated to UI layer

## References

- [GPUI Documentation](https://gpui.rs)
- [Zed GEMINI.md](https://github.com/zed-industries/zed/blob/main/GEMINI.md)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
