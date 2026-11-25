---
name: gpui-patterns
description: Common UI patterns and advanced techniques for GPUI applications. Use when implementing modals, lists, forms, state sharing, or complex component compositions.
---

# GPUI Patterns

This skill provides common UI patterns and advanced techniques for GPUI applications.

## Modal/Overlay Pattern

### Basic Modal

```rust
use gpui::*;

struct Modal {
    is_open: bool,
    content: SharedString,
}

impl Render for Modal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .when(self.is_open, |this| {
                this.absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0, 0, 0, 0.5))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.close(cx);
                    }))
                    .child(
                        div()
                            .p_6()
                            .bg(rgb(0x1a1a1a))
                            .rounded_lg()
                            .min_w(px(300.0))
                            .child(self.content.clone())
                    )
            })
    }
}

impl Modal {
    fn open(&mut self, content: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.is_open = true;
        self.content = content.into();
        cx.notify();
    }
    
    fn close(&mut self, cx: &mut Context<Self>) {
        self.is_open = false;
        cx.notify();
    }
}
```

## Global State Pattern

### Shared Application State

```rust
#[derive(Clone)]
struct AppState {
    user: Option<String>,
    theme: String,
}

// Set global state
fn init_app_state(cx: &mut App) {
    let state = AppState {
        user: None,
        theme: "dark".to_string(),
    };
    cx.set_global(state);
}

// Access global state
fn use_app_state(cx: &App) -> AppState {
    cx.global::<AppState>().clone()
}

// Update global state
fn set_user(cx: &mut App, user: String) {
    let mut state = cx.global::<AppState>().clone();
    state.user = Some(user);
    cx.set_global(state);
}
```

## Parent-Child Communication

### Child Notifying Parent

```rust
#[derive(Clone, Debug)]
enum ChildEvent {
    ValueChanged(i32),
}

impl EventEmitter<ChildEvent> for Child {}

struct Parent {
    child: Entity<Child>,
    _subscription: Subscription,
}

impl Parent {
    fn new(cx: &mut Context<Self>) -> Self {
        let child = cx.new(|_| Child::new());
        
        let subscription = cx.subscribe(&child, |this, _child, event, cx| {
            match event {
                ChildEvent::ValueChanged(value) => {
                    println!("Child value changed to: {}", value);
                    cx.notify();
                }
            }
        });
        
        Self {
            child,
            _subscription: subscription,
        }
    }
}

struct Child {
    value: i32,
}

impl Child {
    fn new() -> Self {
        Self { value: 0 }
    }
    
    fn set_value(&mut self, value: i32, cx: &mut Context<Self>) {
        self.value = value;
        cx.emit(ChildEvent::ValueChanged(value));
        cx.notify();
    }
}
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- List Pattern
- Form Pattern
- Tab Navigation
- Loading State Pattern
- Dropdown Pattern
- Production Component Patterns

## Summary

- Use modals for overlay content
- Implement dynamic lists with `.children()` and iterators
- Create forms with validation and error handling
- Use global state with `cx.set_global()` and `cx.global()`
- Communicate between components with events and subscriptions
- Manage tabs with state and conditional rendering
- Handle async loading with state machines
- Implement dropdowns with relative/absolute positioning
- **Use builder pattern with traits** for fluent APIs
- **Implement variant systems** with normal/hover/active/disabled states
- **Use SmallVec** for performance with dynamic children
- **Manage focus** with keyed state and track_focus
- **Support custom sizes** with Size enum

## References

- [GPUI Documentation](https://gpui.rs)
- [gpui-component Patterns](https://github.com/longbridge/gpui-component)
- [gpui-component Button](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/button/button.rs)
- [gpui-component TabBar](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/tab/tab_bar.rs)
