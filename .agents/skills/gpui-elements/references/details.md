# GPUI Elements: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Custom Components

### RenderOnce Components

```rust
use gpui::*;

#[derive(IntoElement)]
struct Button {
    label: SharedString,
    on_click: Option<Box<dyn Fn(&mut App) + 'static>>,
}

impl Button {
    fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
        }
    }
    
    fn on_click(mut self, handler: impl Fn(&mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut element = div()
            .px_4()
            .py_2()
            .bg(rgb(0x3b82f6))
            .text_color(rgb(0xffffff))
            .rounded(px(6.0))
            .cursor_pointer()
            .child(self.label);
        
        if let Some(handler) = self.on_click {
            element = element.on_click(move |_event, _window, cx| {
                handler(cx);
            });
        }
        
        element
    }
}

// Usage
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Button::new("Click Me")
                .on_click(|cx| {
                    println!("Button clicked!");
                })
        )
    }
}
```

### Render Components

```rust
struct Card {
    title: SharedString,
    content: SharedString,
}

impl Render for Card {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .bg(rgb(0x1a1a1a))
            .rounded(px(8.0))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_lg()
                    .font_bold()
                    .child(self.title.clone())
            )
            .child(
                div()
                    .text_sm()
                    .child(self.content.clone())
            )
    }
}
```

## Layout Patterns

### Vertical Stack

```rust
div()
    .flex()
    .flex_col()
    .gap_4()
    .child("Item 1")
    .child("Item 2")
    .child("Item 3")
```

### Horizontal Row

```rust
div()
    .flex()
    .flex_row()
    .gap_4()
    .items_center()
    .child("Left")
    .child("Center")
    .child("Right")
```

### Grid-like Layout

```rust
div()
    .flex()
    .flex_wrap()
    .gap_4()
    .children((0..12).map(|i| {
        div()
            .w(px(100.0))
            .h(px(100.0))
            .bg(rgb(0x3b82f6))
            .child(format!("Item {}", i))
    }))
```

### Centering

```rust
div()
    .flex()
    .items_center()
    .justify_center()
    .size_full()
    .child("Centered content")
```

## Nested Components

### Component Composition

```rust
struct Header;
impl Render for Header {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(64.0))
            .bg(rgb(0x1a1a1a))
            .child("Header")
    }
}

struct Sidebar;
impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(250.0))
            .bg(rgb(0x2a2a2a))
            .child("Sidebar")
    }
}

struct AppLayout {
    header: Entity<Header>,
    sidebar: Entity<Sidebar>,
}

impl Render for AppLayout {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.header.clone())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex_1()
                            .child("Main content")
                    )
            )
    }
}
```

## Element Patterns

### Clickable Card

```rust
div()
    .p_4()
    .bg(rgb(0x1a1a1a))
    .rounded(px(8.0))
    .cursor_pointer()
    .hover(|style| style.bg(rgb(0x2a2a2a)))
    .on_click(|_event, _window, cx| {
        println!("Card clicked");
    })
    .child("Click me")
```

### Input Field

```rust
struct InputField {
    value: SharedString,
}

impl Render for InputField {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .bg(rgb(0x1a1a1a))
            .border_1()
            .border_color(rgb(0x4a4a4a))
            .rounded(px(4.0))
            .child(self.value.clone())
            .on_click(cx.listener(|this, _event, _window, cx| {
                // Handle focus
            }))
    }
}
```

### Toggle Switch

```rust
struct Toggle {
    enabled: bool,
}

impl Render for Toggle {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(44.0))
            .h(px(24.0))
            .rounded(px(12.0))
            .bg(if self.enabled { 
                rgb(0x22c55e) 
            } else { 
                rgb(0x4a4a4a) 
            })
            .cursor_pointer()
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.enabled = !this.enabled;
                cx.notify();
            }))
    }
}
```
