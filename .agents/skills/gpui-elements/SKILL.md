---
name: gpui-elements
description: UI element creation and composition with GPUI. Use when building element trees, creating custom components, implementing layouts, or working with conditional rendering.
---

# GPUI Elements

This skill covers creating and composing UI elements in GPUI applications.

## Overview

Elements are the building blocks of GPUI UIs:
- Built with `div()` as the base element
- Styled with chainable methods
- Composed with `.child()` and `.children()`
- Support conditional rendering with `.when()`
- Use flexbox for layout

## Basic Element Creation

### div() Element

```rust
use gpui::*;

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child("Hello, GPUI!")
    }
}
```

### Adding Children

```rust
div()
    .child("First child")
    .child(
        div()
            .child("Nested child")
    )
    .child("Third child")
```

### Multiple Children with Vector

```rust
let items = vec!["One", "Two", "Three"];

div().children(
    items.into_iter().map(|text| {
        div().child(text)
    })
)
```

## Text Elements

### String Types

```rust
use gpui::*;

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // &str
            .child("Static text")
            // String
            .child(format!("Count: {}", self.count))
            // SharedString (most efficient)
            .child(self.title.clone())
    }
}
```

### SharedString

```rust
use gpui::*;

struct MyView {
    title: SharedString,
}

impl MyView {
    fn new() -> Self {
        Self {
            title: "Default Title".into(),
        }
    }
    
    fn set_title(&mut self, title: impl Into<SharedString>) {
        self.title = title.into();
    }
}

// SharedString implements IntoElement
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(self.title.clone())
    }
}
```

## Conditional Rendering

### when() Method

```rust
div()
    .when(condition, |this| {
        this.bg(rgb(0x3b82f6))
            .child("Condition is true")
    })
```

### when_some() for Options

```rust
div()
    .when_some(maybe_text, |this, text| {
        this.child(text)
    })
```

### Practical Example

```rust
struct MyView {
    is_active: bool,
    error: Option<String>,
}

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .when(self.is_active, |this| {
                this.bg(rgb(0x22c55e))
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .p_2()
                        .bg(rgb(0xef4444))
                        .text_color(rgb(0xffffff))
                        .child(error)
                )
            })
    }
}
```

## Dynamic Lists

### Simple List

```rust
struct ListView {
    items: Vec<String>,
}

impl Render for ListView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(
                self.items.iter().map(|item| {
                    div()
                        .p_2()
                        .bg(rgb(0x1a1a1a))
                        .rounded(px(4.0))
                        .child(item.clone())
                })
            )
    }
}
```

### List with Index

```rust
div()
    .children(
        self.items.iter().enumerate().map(|(i, item)| {
            div()
                .child(format!("{}. {}", i + 1, item))
        })
    )
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Custom Components
- Layout Patterns
- Nested Components
- Element Patterns

## Best Practices

1. **Use SharedString for text**: Avoids unnecessary string copies
2. **Use .when() for conditions**: More readable than if/else in render
3. **Extract reusable components**: Create RenderOnce components for common patterns
4. **Keep render pure**: Don't mutate state directly in render() method
5. **Use children() for lists**: More efficient than multiple .child() calls
6. **Chain methods**: Build elements with method chaining for clarity

## Common Mistakes

| Mistake | Problem | Solution |
|---------|---------|----------|
| Mutating state in render | Race conditions | Use event handlers to mutate |
| Not using SharedString | Extra allocations | Use `SharedString` for text |
| Deep nesting | Hard to read | Extract into components |
| Missing IntoElement | Compile error | Ensure types implement `IntoElement` |
| Forgetting .clone() | Borrow errors | Clone `SharedString` when using multiple times |

## Summary

- Use `div()` as the base element
- Add children with `.child()` or `.children()`
- Use `.when()` for conditional rendering
- Create components with `RenderOnce` or `Render`
- Use `SharedString` for efficient text handling
- Compose with flexbox layout methods

## References

- [GPUI Elements Documentation](https://gpui.rs)
- [gpui-component Library](https://github.com/longbridge/gpui-component)
