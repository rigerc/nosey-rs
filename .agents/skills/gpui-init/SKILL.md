---
name: gpui-init
description: Initialize new GPUI application projects with proper structure and boilerplate. Use when creating a new GPUI app, setting up project structure, or scaffolding a cross-platform Rust desktop UI application.
---

# GPUI Project Initialization

This skill enables you to scaffold new GPUI application projects with proper structure, dependencies, and boilerplate code.

## Overview

When creating a new GPUI project, generate the appropriate files based on project complexity:
- **Simple app**: Minimal structure for learning/prototyping (single file)
- **Production app**: Full structure with modules and components

## Simple App Template

### Directory Structure

```
my_app/
├── Cargo.toml
├── src/
│   └── main.rs
└── .gitignore
```

### Cargo.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"
description = "A GPUI application"
license = "MIT"

[dependencies]
gpui = "0.2.2"
```

### src/main.rs

```rust
use gpui::*;

struct HelloWorld {
    counter: usize,
}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .items_center()
            .justify_center()
            .child(format!("Counter: {}", self.counter))
            .child(
                div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x3b82f6))
                    .text_color(rgb(0xffffff))
                    .rounded(px(4.0))
                    .child("Click Me")
                    .on_click(|_event, _window, cx| {
                        println!("Clicked!");
                    })
            )
    }
}

fn main() {
    let app = Application::new();
    
    app.run(move |cx| {
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                cx.new(|_cx| HelloWorld { counter: 0 })
            })?;
            
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

### .gitignore

```
/target
.DS_Store
*.swp
*.swo
.idea/
.vscode/
Cargo.lock
```

## Optional: Using Pre-built Components

> [!NOTE]
> For pre-built UI components (similar to shadcn/ui), see the **gpui-component-usage** skill.

## Project Checklist

When initializing a new project, ensure:

- [ ] `Cargo.toml` has correct package metadata
- [ ] `gpui` dependency is version `0.2.2` or later
- [ ] `main.rs` initializes `Application::new()`
- [ ] Window is created with `cx.open_window()`
- [ ] `.gitignore` excludes `/target` and IDE files
- [ ] `rust-toolchain.toml` specifies stable channel

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Production App Template

## References

- [GPUI Documentation](https://gpui.rs)
- [Zed Repository](https://github.com/zed-industries/zed)
