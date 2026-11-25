---
name: gpui-actions
description: Actions and event handling in GPUI applications. Use when implementing user interactions, keyboard shortcuts, custom actions, or event-driven behavior.
---

# GPUI Actions

This skill covers actions, events, and user interaction handling in GPUI.

## Overview

GPUI actions provide:
- **Type-safe event handling** with the Action trait
- **Keyboard shortcut binding** with action dispatch
- **Mouse/touch events** with `.on_click()`, `.on_mouse_down()`, etc.
- **Focus management** for keyboard navigation

## Defining Actions

### Simple Actions

Use the `actions!` macro for actions with no data:

```rust
use gpui::*;

actions!(my_app, [Save, Open, Close, Refresh]);

// These actions have no associated data
```

### Actions with Data

Use `#[derive(Clone, PartialEq)]` with the `Action` derive:

```rust
use gpui::*;

#[derive(Clone, PartialEq)]
struct SetCount {
    value: usize,
}

impl_actions!(my_app, [SetCount]);
```

### Action Documentation

Doc comments on actions are displayed to users:

```rust
actions!(editor, [
    /// Save the current file
    Save,
    /// Open a file
    Open,
]);
```

## Keyboard Events

### Key Press Handler

```rust
div()
    .on_key_down(|event, window, cx| {
        if event.keystroke.key == "Enter" {
            println!("Enter pressed");
        }
    })
    .on_key_up(|event, window, cx| {
        // Key released
    })
```

### Modifier Keys

```rust
.on_key_down(|event, window, cx| {
    if event.keystroke.modifiers.control && event.keystroke.key == "s" {
        println!("Ctrl+S pressed");
        // Handle save action
    }
})
```

## Action Handlers

### Registering Action Handlers

```rust
use gpui::*;

actions!(my_app, [Increment, Decrement]);

struct Counter {
    count: i32,
}

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(format!("Count: {}", self.count))
            .on_action(cx.listener(|this, action: &Increment, window, cx| {
                this.count += 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &Decrement, window, cx| {
                this.count -= 1;
                cx.notify();
            }))
    }
}
```

### Dispatching Actions

```rust
// Dispatch from window
window.dispatch_action(Save.boxed_clone(), cx);

// Dispatch from focus handle
focus_handle.dispatch_action(&Save, window, cx);
```

## Focus Management

### Focus Handles

```rust
use gpui::*;

struct InputField {
    focus_handle: FocusHandle,
    text: SharedString,
}

impl InputField {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            text: "".into(),
        }
    }
}

impl Render for InputField {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.focus_handle.focus(cx);
            }))
            .child(self.text.clone())
    }
}
```

### Focus Events

```rust
div()
    .track_focus(&self.focus_handle)
    .on_focus(cx.listener(|this, _event, _window, cx| {
        println!("Gained focus");
    }))
    .on_blur(cx.listener(|this, _event, _window, cx| {
        println!("Lost focus");
    }))
```

## Event Propagation

### Stopping Propagation

```rust
div()
    .on_click(|event, _window, cx| {
        println!("Child clicked");
        // Prevent parent from receiving event
        event.stop_propagation();
    })
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Mouse Events
- Practical Examples

## Best Practices

1. **Use cx.listener()**: For accessing entity state in event handlers
2. **Call cx.notify()**: After state changes that affect rendering
3. **Use actions for keyboard shortcuts**: More maintainable than key handlers
4. **Handle focus properly**: Use `track_focus()` and `FocusHandle`
5. **Document actions**: Add doc comments for better UX
6. **Use type-safe actions**: Prefer actions over string-based events

## Common Mistakes

| Mistake | Problem | Solution |
|---------|---------|----------|
| Forgetting cx.notify() | UI doesn't update | Call after state changes |
| Not using cx.listener() | Can't access entity | Use `cx.listener()` |
| Nested event handlers | Confusing behavior | Use event propagation control |
| Missing focus tracking | Keyboard events don't work | Use `.track_focus()` |
| Not boxing actions | Type errors | Use `.boxed_clone()` for dispatch |

## Summary

- Use `actions!()` macro for simple actions
- Use `.on_click(cx.listener(...))` for clickable elements
- Use `.on_action()` for action handlers
- Use `FocusHandle` for keyboard focus
- Call `cx.notify()` after state changes
- Dispatch actions with `window.dispatch_action()`

## References

- [GPUI Actions Documentation](https://gpui.rs)
- [Zed Keyboard Shortcuts](https://github.com/zed-industries/zed)
