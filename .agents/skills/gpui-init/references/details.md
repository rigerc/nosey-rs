# GPUI Project Initialization: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Production App Template

### Directory Structure

```
my_app/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── ui/
│   │   ├── mod.rs
│   │   └── home.rs
│   └── shared/
│       ├── mod.rs
│       └── theme.rs
├── .gitignore
└── rust-toolchain.toml
```

### Cargo.toml (Production)

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"
description = "A cross-platform GPUI application"
license = "MIT"
authors = ["Your Name"]

[dependencies]
gpui = "0.2.2"
anyhow = "1.0"

[features]
default = []

[profile.release]
opt-level = 3
lto = "thin"
```

### src/main.rs (Production)

```rust
mod app;
mod ui;
mod shared;

fn main() {
    app::run();
}
```

### src/app.rs

```rust
use gpui::*;
use crate::ui::home::HomeView;

pub fn run() {
    let app = Application::new();
    
    app.run(move |cx| {
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point { x: px(100.0), y: px(100.0) },
                        size: Size { width: px(1024.0), height: px(768.0) },
                    })),
                    titlebar: Some(TitlebarOptions {
                        title: Some("My App".into()),
                        appears_transparent: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|_cx| HomeView::new())
                }
            )?;
            
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

### src/ui/mod.rs

```rust
pub mod home;
```

### src/ui/home.rs

```rust
use gpui::*;
use crate::shared::theme;

pub struct HomeView {
    counter: usize,
}

impl HomeView {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
    
    fn increment(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.counter += 1;
        cx.notify();
    }
}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .size_full()
            .p_8()
            .bg(theme::BG_COLOR)
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme::TEXT_COLOR)
                    .child("Welcome to GPUI")
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_6()
                    .bg(theme::CARD_BG)
                    .rounded(px(8.0))
                    .child(
                        div()
                            .text_lg()
                            .text_color(theme::TEXT_COLOR)
                            .child(format!("Counter: {}", self.counter))
                    )
                    .child(
                        div()
                            .px_6()
                            .py_3()
                            .bg(theme::PRIMARY_COLOR)
                            .text_color(rgb(0xffffff))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .child("Increment")
                            .on_click(cx.listener(Self::increment))
                    )
            )
    }
}
```

### src/shared/mod.rs

```rust
pub mod theme;
```

### src/shared/theme.rs

```rust
use gpui::*;

pub const BG_COLOR: Hsla = hsla(0.0, 0.0, 0.05, 1.0);
pub const CARD_BG: Hsla = hsla(0.0, 0.0, 0.1, 1.0);
pub const TEXT_COLOR: Hsla = hsla(0.0, 0.0, 0.9, 1.0);
pub const PRIMARY_COLOR: Hsla = hsla(0.6, 0.8, 0.5, 1.0);
```

### rust-toolchain.toml

```toml
[toolchain]
channel = "stable"
```
