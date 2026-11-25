# GPUI Entities: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Updating Entities

### Basic Update

```rust
fn increment(entity: &Entity<MyView>, cx: &mut App) {
    entity.update(cx, |view, cx| {
        view.count += 1;
        cx.notify(); // Trigger re-render
    });
}
```

### Update with Window

Use `.update_in()` when you need window access:

```rust
fn increment_and_dispatch(
    entity: &Entity<MyView>,
    cx: &mut AsyncWindowContext,
) -> anyhow::Result<()> {
    entity.update_in(cx, |view, window, cx| {
        view.count += 1;
        window.dispatch_action(CountChanged.boxed_clone(), cx);
        cx.notify();
    })
}
```

### Return Values

```rust
// update returns the closure's return value
let new_count = entity.update(cx, |view, cx| {
    view.count += 1;
    cx.notify();
    view.count // Return new count
});

println!("New count: {}", new_count);
```

## WeakEntity

Use `WeakEntity<T>` to avoid memory leaks when entities reference each other.

### Creating Weak References

```rust
struct Parent {
    child: Entity<Child>,
}

struct Child {
    parent: WeakEntity<Parent>, // Weak to avoid cycle
}

impl Parent {
    fn new(cx: &mut Context<Self>) -> Self {
        let parent_weak = cx.entity().downgrade();
        
        let child = cx.new(|_cx| Child {
            parent: parent_weak,
        });
        
        Self { child }
    }
}
```

### Using Weak References

```rust
impl Child {
    fn notify_parent(&self, cx: &mut Context<Self>) {
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |parent, cx| {
                parent.handle_child_event(cx);
            });
        }
    }
}
```

### Async Context with WeakEntity

When using async contexts, methods return `anyhow::Result`:

```rust
fn do_async_work(&self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        // this: WeakEntity<Self>
        
        // Async work
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Update returns Result in async context
        this.update(&mut *cx, |view, cx| {
            view.count += 1;
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## Subscriptions

Subscribe to entity events to observe state changes.

### Defining Events

```rust
use gpui::*;

#[derive(Clone, Debug)]
enum MyEvent {
    CountChanged { new_value: usize },
    Reset,
}

// Declare that this entity can emit MyEvent
impl EventEmitter<MyEvent> for MyView {}
```

### Emitting Events

```rust
impl MyView {
    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        cx.emit(MyEvent::CountChanged { new_value: self.count });
        cx.notify();
    }
    
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.count = 0;
        cx.emit(MyEvent::Reset);
        cx.notify();
    }
}
```

### Subscribing to Events

```rust
struct ParentView {
    child: Entity<ChildView>,
    _subscription: Subscription,
}

impl ParentView {
    fn new(cx: &mut Context<Self>) -> Self {
        let child = cx.new(|_| ChildView::new());
        
        let subscription = cx.subscribe(&child, |this, child, event, cx| {
            // this: &mut ParentView
            // child: Entity<ChildView>
            // event: &MyEvent
            // cx: &mut Context<ParentView>
            
            match event {
                MyEvent::CountChanged { new_value } => {
                    println!("Child count changed: {}", new_value);
                }
                MyEvent::Reset => {
                    println!("Child was reset");
                }
            }
        });
        
        Self {
            child,
            _subscription: subscription, // Keeps subscription alive
        }
    }
}
```

**Important**: Store `Subscription` in a field to keep it active. When dropped, the subscription is automatically cancelled.

## Common Patterns

### Parent-Child Communication

```rust
struct Parent {
    child: Entity<Child>,
}

impl Parent {
    fn tell_child_something(&self, cx: &mut Context<Self>) {
        self.child.update(cx, |child, cx| {
            child.handle_message_from_parent(cx);
        });
    }
}

struct Child {
    parent: WeakEntity<Parent>,
}

impl Child {
    fn tell_parent_something(&self, cx: &mut Context<Self>) {
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |parent, cx| {
                parent.handle_message_from_child(cx);
            });
        }
    }
}
```

### Shared State Entity

```rust
#[derive(Clone)]
struct AppState {
    theme: String,
    user: Option<String>,
}

struct MyApp {
    state: Entity<AppState>,
}

impl MyApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState {
            theme: "dark".into(),
            user: None,
        });
        
        Self { state }
    }
    
    fn change_theme(&self, theme: String, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.theme = theme;
            cx.notify();
        });
    }
}
```

### Entity as Global State

```rust
fn set_global_state(cx: &mut App, state: Entity<AppState>) {
    cx.set_global(state);
}

fn get_global_state(cx: &App) -> Entity<AppState> {
    cx.global::<Entity<AppState>>().clone()
}

// Usage
impl MyView {
    fn access_global(&self, cx: &Context<Self>) {
        let state = get_global_state(cx);
        state.read_with(cx, |state, _| {
            println!("Theme: {}", state.theme);
        });
    }
}
```
