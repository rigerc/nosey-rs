# GPUI Code Quality: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Error Handling

### Never Use unwrap()

**Rule**: Avoid `unwrap()` and panic-inducing methods

```rust
// ❌ WRONG
let value = option.unwrap();
let result = operation().unwrap();

// ✅ CORRECT - propagate with ?
let value = option.ok_or_else(|| anyhow::anyhow!("Missing value"))?;
let result = operation()?;

// ✅ ALSO CORRECT - explicit handling
match option {
    Some(value) => {
        // Use value
    }
    None => {
        // Handle error case
    }
}
```

### Never Silently Discard Errors

**Rule**: Don't use `let _ =` on fallible operations

```rust
// ❌ WRONG - silently discards error
let _ = client.request(url).await?;

// ✅ CORRECT - propagate error
client.request(url).await?;

// ✅ ALSO CORRECT - log but ignore
operation().log_err();

// ✅ ALSO CORRECT - explicit handling
if let Err(e) = operation() {
    eprintln!("Operation failed: {}", e);
}
```

### Propagate Errors to UI

```rust
fn save_file(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        match write_file(path, data).await {
            Ok(()) => {
                this.update(&mut *cx, |view, cx| {
                    view.status = "Saved successfully".into();
                    cx.notify();
                })?;
            }
            Err(e) => {
                this.update(&mut *cx, |view, cx| {
                    view.error = Some(format!("Save failed: {}", e));
                    cx.notify();
                })?;
            }
        }
        
        Ok(())
    }).detach_and_log_err(cx);
}
```

## State Management

### Call cx.notify() After Changes

```rust
// ❌ WRONG
fn update_data(&mut self, data: String, cx: &mut Context<Self>) {
    self.data = data;
    // Missing cx.notify() - UI won't update!
}

// ✅ CORRECT
fn update_data(&mut self, data: String, cx: &mut Context<Self>) {
    self.data = data;
    cx.notify(); // Trigger re-render
}
```

### Use WeakEntity for Back-References

```rust
// ❌ WRONG - memory leak
struct Child {
    parent: Entity<Parent>, // Strong reference creates cycle!
}

// ✅ CORRECT
struct Child {
    parent: WeakEntity<Parent>, // Weak reference prevents leak
}
```

## Rendering

### Keep Render Pure

**Rule**: Don't mutate state in `render()` method

```rust
// ❌ WRONG
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.render_count += 1; // Don't mutate in render!
        div().child(format!("Renders: {}", self.render_count))
    }
}

// ✅ CORRECT - mutate in event handlers
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(format!("Count: {}", self.count))
            .child(
                div()
                    .child("Increment")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
            )
    }
}
```

### Use SharedString for Text

```rust
// ❌ LESS EFFICIENT
struct MyView {
    title: String,
}

// ✅ MORE EFFICIENT
struct MyView {
    title: SharedString,
}
```

## Async Patterns

### Detach or Store Tasks

```rust
// ❌ WRONG - task dropped immediately
cx.spawn(async move |this, cx| {
    // Work...
    Ok(())
});

// ✅ CORRECT - detach
cx.spawn(async move |this, cx| {
    // Work...
    Ok(())
}).detach();

// ✅ ALSO CORRECT - store for cancellation
self.current_task = Some(cx.spawn(async move |this, cx| {
    // Work...
    Ok(())
}));
```

### Use background_spawn for CPU Work

```rust
// ❌ WRONG - blocks UI thread
cx.spawn(async move |this, cx| {
    let result = expensive_computation(); // Blocks UI!
    Ok(())
});

// ✅ CORRECT - run on background thread
cx.spawn(async move |this, cx| {
    let result = cx.background_spawn(async {
        expensive_computation()
    }).await;
    
    this.update(&mut *cx, |view, cx| {
        view.result = result;
        cx.notify();
    })?;
    
    Ok(())
});
```
