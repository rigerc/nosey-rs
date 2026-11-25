---
name: gpui-entities
description: Entity and context patterns for state management in GPUI. Use when managing component state, implementing entity lifecycle, handling subscriptions, or avoiding common entity access pitfalls.
---

# GPUI Entities

This skill provides comprehensive patterns for working with entities and contexts in GPUI applications.

## Overview

Entities are GPUI's core state management primitive:
- **Type-safe handles** to state with automatic lifecycle management  
- **Read/write access** with borrow checking at runtime
- **Weak references** to avoid memory leaks
- **Subscriptions** for observing state changes

## Creating Entities

### In Window Context

```rust
use gpui::*;

struct MyView {
    count: usize,
}

fn main() {
    let app = Application::new();
    
    app.run(move |cx| {
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                // Create entity with cx.new()
                cx.new(|_cx| MyView { count: 0 })
            })?;
            
            Ok::<_, anyhow::Error>(())
        }).detach();
    });
}
```

### In Entity Context

```rust
struct ParentView {
    child: Entity<ChildView>,
}

impl ParentView {
    fn new(cx: &mut Context<Self>) -> Self {
        let child = cx.new(|_cx| ChildView::new());
        Self { child }
    }
}
```

## Reading Entities

### Direct Read

```rust
fn display_count(entity: &Entity<MyView>, cx: &App) {
    let view = entity.read(cx);
    println!("Count: {}", view.count);
}
```

### Read with Closure

```rust
fn get_count(entity: &Entity<MyView>, cx: &App) -> usize {
    entity.read_with(cx, |view, _cx| view.count)
}
```

**Important**: The closure return value is returned directly from `.read_with()`.

## Entity Lifecycle

### Entity ID

```rust
let id = entity.entity_id();
// id: EntityId - unique identifier that persists
```

### Checking Entity Validity

```rust
// Strong entity is always valid
let entity: Entity<MyView> = cx.new(|_| MyView { count: 0 });
// entity is guaranteed to be valid

// Weak entity might be invalid
let weak: WeakEntity<MyView> = entity.downgrade();

if let Some(entity) = weak.upgrade() {
    // Entity still exists
    entity.update(cx, |view, cx| {
        // ...
    });
} else {
    // Entity was dropped
    println!("Entity no longer exists");
}
```

## Observing Changes

Use `cx.observe()` to be notified when an entity calls `cx.notify()`:

```rust
struct Dashboard {
    counter: Entity<Counter>,
    _observer: Subscription,
}

impl Dashboard {
    fn new(cx: &mut Context<Self>) -> Self {
        let counter = cx.new(|_| Counter { value: 0 });
        
        let observer = cx.observe(&counter, |this, observed, cx| {
            // Called whenever counter.cx.notify() is called
            // observed: Entity<Counter>
            println!("Counter changed!");
            cx.notify(); // Re-render dashboard
        });
        
        Self {
            counter,
            _observer: observer,
        }
    }
}
```

## Context Methods

### Getting Entity Reference

From within an entity's method:

```rust
impl MyView {
    fn get_self_reference(&self, cx: &Context<Self>) -> WeakEntity<Self> {
        cx.entity().downgrade()
    }
}
```

### Notify

Trigger re-render for this entity:

```rust
impl MyView {
    fn update_state(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify(); // Mark this entity as needing re-render
    }
}
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Updating Entities
- WeakEntity
- Subscriptions
- Common Patterns

## Common Mistakes

| Mistake | Problem | Solution |
|---------|---------|----------|
| Using outer `cx` in update | Borrow checker error | Use inner `cx` from closure |
| Nested updates | Runtime panic | Restructure to avoid updating while updating |
| Forgetting `cx.notify()` | UI doesn't update | Call `cx.notify()` after state changes |
| Strong circular references | Memory leak | Use `WeakEntity` for back-references |
| Dropping `Subscription` | Events stop working | Store `Subscription` in struct field |
| Not handling `None` from weak upgrade | Panic or undefined behavior | Always check `upgrade()` result |

## Best Practices

1. **Use WeakEntity for back-references**: Prevents memory leaks
2. **Store subscriptions**: Keep `Subscription` values in struct fields
3. **Always call cx.notify()**: After state changes that affect rendering
4. **Handle weak reference failures**: Always check if `upgrade()` returns `Some`
5. **Use read_with for simple access**: More ergonomic than `.read()`
6. **Avoid long-lived strong references**: Consider `WeakEntity` if entity won't be used frequently

## Summary

- Create with `cx.new()`
- Read with `.read()` or `.read_with()`
- Update with `.update()` or `.update_in()`
- Use `WeakEntity` to avoid cycles
- Subscribe with `cx.subscribe()` for events
- Observe with `cx.observe()` for notify signals
- Always call `cx.notify()` after state changes

## References

- [GPUI Entities Documentation](https://gpui.rs)
- [Zed GEMINI.md - Entities](https://github.com/zed-industries/zed/blob/main/GEMINI.md)
