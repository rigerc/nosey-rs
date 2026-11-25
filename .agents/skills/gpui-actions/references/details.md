# GPUI Actions: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Mouse Events

### on_click

```rust
use gpui::*;

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child("Click me")
            .on_click(|event, window, cx| {
                println!("Clicked at: {:?}", event.position);
            })
    }
}
```

### on_click with cx.listener

Use `cx.listener()` to access the entity:

```rust
impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(format!("Count: {}", self.count))
            .child(
                div()
                    .child("Increment")
                    .on_click(cx.listener(|this, event, window, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
            )
    }
}
```

### Other Mouse Events

```rust
div()
    .on_mouse_down(|event, window, cx| {
        // Mouse button pressed
    })
    .on_mouse_up(|event, window, cx| {
        // Mouse button released
    })
    .on_mouse_move(|event, window, cx| {
        // Mouse moved over element
    })
    .on_hover(|is_hovered, window, cx| {
        // Hover state changed
    })
```

## Practical Examples

### Button Component

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
        let mut div = div()
            .px_4()
            .py_2()
            .bg(rgb(0x3b82f6))
            .text_color(rgb(0xffffff))
            .rounded(px(6.0))
            .cursor_pointer()
            .child(self.label);
        
        if let Some(handler) = self.on_click {
            div = div.on_click(move |_event, _window, cx| {
                handler(cx);
            });
        }
        
        div
    }
}
```

### Form with Multiple Actions

```rust
actions!(form, [Submit, Cancel, Reset]);

struct LoginForm {
    username: SharedString,
    password: SharedString,
}

impl Render for LoginForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_4()
            .on_action(cx.listener(|this, _: &Submit, window, cx| {
                println!("Submitting: {}", this.username);
                // Handle submit
            }))
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                println!("Cancelled");
                // Handle cancel
            }))
            .on_action(cx.listener(|this, _: &Reset, window, cx| {
                this.username = "".into();
                this.password = "".into();
                cx.notify();
            }))
            .child("Login Form")
    }
}
```

### Drag and Drop

```rust
struct DraggableItem {
    position: Point<Pixels>,
    is_dragging: bool,
    drag_offset: Point<Pixels>,
}

impl Render for DraggableItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left(self.position.x)
            .top(self.position.y)
            .w(px(100.0))
            .h(px(100.0))
            .bg(rgb(0x3b82f6))
            .cursor_pointer()
            .on_mouse_down(cx.listener(|this, event, _window, cx| {
                this.is_dragging = true;
                this.drag_offset = event.position - this.position;
                cx.notify();
            }))
            .on_mouse_up(cx.listener(|this, _event, _window, cx| {
                this.is_dragging = false;
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|this, event, _window, cx| {
                if this.is_dragging {
                    this.position = event.position - this.drag_offset;
                    cx.notify();
                }
            }))
            .child("Drag me")
    }
}
```
