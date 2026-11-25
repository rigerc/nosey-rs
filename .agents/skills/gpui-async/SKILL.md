---
name: gpui-async
description: Async patterns and concurrency in GPUI applications. Use when implementing background tasks, async/await operations, task management, or concurrent operations.
---

# GPUI Async

This skill covers async patterns and concurrency management in GPUI.

## Overview

GPUI provides async primitives for:
- **Foreground tasks**: `cx.spawn()` for UI thread work
- **Background tasks**: `cx.background_spawn()` for CPU-intensive work
- **Task management**: `Task<R>` for cancellation and lifecycle
- **Async contexts**: `AsyncApp` and `AsyncWindowContext` for async operations

## Foreground Tasks

All UI rendering happens on a single foreground thread.

### Basic Spawn

```rust
use gpui::*;

impl MyView {
    fn start_work(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            // this: WeakEntity<Self>
            // cx: &mut AsyncApp
            
            // Do async work on UI thread
            this.update(&mut *cx, |view, cx| {
                view.status = "Working...".into();
                cx.notify();
            })?;
            
            Ok(())
        }).detach();
    }
}
```

### Accessing Entity in Async

```rust
fn fetch_data(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        // Simulate async work
       tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Update entity from async context
        this.update(&mut *cx, |view, cx| {
            view.data = "Loaded!".into();
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## Async with Window

Use `.update_in()` when you need window access in async contexts:

```rust
fn async_with_window(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        this.update_in(&mut *cx, |view, window, cx| {
            view.count += 1;
            window.dispatch_action(CountChanged.boxed_clone(), cx);
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## Error Handling

### Propagating Errors

```rust
cx.spawn(async move |this, cx| {
    // Use ? to propagate errors
    let data = fetch_data().await?;
    
    this.update(&mut *cx, |view, cx| {
        view.data = data;
        cx.notify();
    })?;
    
    Ok::<_, anyhow::Error>(())
}).detach_and_log_err(cx);
```

### Handling Errors Explicitly

```rust
cx.spawn(async move |this, cx| {
    match fetch_data().await {
        Ok(data) => {
            this.update(&mut *cx, |view, cx| {
                view.data = data;
                view.error = None;
                cx.notify();
            })?;
        }
        Err(e) => {
            this.update(&mut *cx, |view, cx| {
                view.error = Some(e.to_string());
                cx.notify();
            })?;
        }
    }
    
    Ok(())
}).detach();
```

## Variable Shadowing in Async

Use variable shadowing to scope clones clearly:

```rust
fn async_with_data(&mut self, cx: &mut Context<Self>) {
    let data = self.shared_data.clone();
    
    cx.spawn(async move |this, cx| {
        // Use cloned data
        let result = process(data).await;
        
        this.update(&mut *cx, |view, cx| {
            view.result = result;
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Background Tasks
- Task Management
- Common Patterns

## Best Practices

1. **Use `.detach()` for fire-and-forget**: When you don't need the result
2. **Store tasks for cancellation**: Keep `Task` in struct field to cancel on drop
3. **Use `background_spawn()` for CPU work**: Don't block UI thread
4. **Always handle errors**: Use `?` or explicit error handling
5. **Clone before move**: Use variable shadowing for clarity
6. **Call `cx.notify()`**: After updating state from async

## Common Mistakes

| Mistake | Problem | Solution |
|---------|---------|----------|
| Not using `&mut *cx` | Borrow error | Dereference with `&mut *cx` |
| Forgetting `cx.notify()` | UI doesn't update | Call after state changes |
| Long-running foreground tasks | UI freezes | Use `background_spawn()` |
| Not handling weak entity None | Panic | Always use `?` or check result |
| Blocking in spawn | UI freezes | Use `background_spawn()` for blocking work |

## Summary

- Use `cx.spawn()` for foreground async work
- Use `cx.background_spawn()` for CPU-intensive tasks
- Call `.detach()` for fire-and-forget tasks
- Store `Task` in fields for cancellation
- Use `?` to propagate errors
- Call `cx.notify()` after async updates
- Use variable shadowing for clones

## References

- [GPUI Concurrency Documentation](https://gpui.rs)
- [Zed GEMINI.md - Concurrency](https://github.com/zed-industries/zed/blob/main/GEMINI.md)
