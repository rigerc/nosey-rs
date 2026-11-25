---
name: gpui-fundamentals
description: Core GPUI concepts including contexts, windows, entities, elements, and rendering. Use when learning GPUI basics, understanding the framework architecture, or implementing core UI patterns.
---

# GPUI Fundamentals

This skill covers the core concepts of GPUI framework for building desktop UI applications in Rust.

## Overview

GPUI is a UI framework that provides:
- **Entities**: Handles to state with lifecycle management
- **Contexts**: Access to global state, windows, and system services
- **Elements**: Composable UI building blocks
- **Rendering**: `Render` trait for creating element trees
- **Concurrency**: Async primitives for background work

## Context Types

Context types allow interaction with global state, windows, entities, and system services. They are passed as the argument named `cx`.

### App

`App` is the root context type, providing access to global state and read/update of entities.

```rust
fn do_something(cx: &mut App) {
    // Access global state
    // Read/update entities
}
```

### Context<T>

Provided when updating an `Entity<T>`. This context dereferences into `App`, so functions which take `&App` can also take `&Context<T>`.

```rust
struct MyView {
    count: usize,
}

impl MyView {
    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify(); // Tell GPUI to re-render
    }
}
```

### AsyncApp and AsyncWindowContext

Provided by `cx.spawn()` for async operations. These can be held across await points.

```rust
fn start_async_work(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        // this: WeakEntity<Self>
        // cx: &mut AsyncApp
        
        // Do async work
        Ok(())
    }).detach();
}
```

## Window

`Window` provides access to the state of an application window. It is passed as an argument named `window` and comes **before** `cx` when present.

```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Hello")
    }
}
```

Used for:
- Managing focus
- Dispatching actions
- Directly drawing
- Getting user input state

## Element Composition

### Basic Composition

```rust
div()
    .child("Text")
    .child(div().child("Nested"))
    .child(another_element())
```

### Conditional Rendering

Use `.when()` for conditional attributes/children:

```rust
div()
    .when(is_active, |this| {
        this.bg(rgb(0x3b82f6))
    })
    .when_some(maybe_text, |this, text| {
        this.child(text)
    })
```

### Multiple Children

```rust
div()
    .children(vec![
        div().child("Item 1"),
        div().child("Item 2"),
        div().child("Item 3"),
    ])
```

## Application Lifecycle

### Basic Application

```rust
use gpui::*;

struct AppView;

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("My App")
    }
}

fn main() {
    let app = Application::new();
    
    app.run(move |cx| {
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                cx.new(|_| AppView)
            })?;
            
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

### Window Options

```rust
use gpui::*;

let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(Bounds {
        origin: Point { x: px(100.0), y: px(100.0) },
        size: Size { width: px(1024.0), height: px(768.0) },
    })),
    titlebar: Some(TitlebarOptions {
        title: Some("My Application".into()),
        appears_transparent: false,
        ..Default::default()
    }),
    focus: true,
    show: true,
    ..Default::default()
};

cx.open_window(options, |window, cx| {
    cx.new(|_| MyView::new())
})?;
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Entities
- Elements
- Common Patterns
