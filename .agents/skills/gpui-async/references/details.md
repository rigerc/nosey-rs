# GPUI Async: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Background Tasks

Use `background_spawn()` for CPU-intensive work that shouldn't block UI.

### Background Computation

```rust
fn compute_heavy(&mut self, cx: &mut Context<Self>) {
    let input = self.input.clone();
    
    cx.spawn(async move |this, cx| {
        // Spawn background task
        let result = cx.background_spawn(async move {
            // Heavy computation on background thread
            expensive_calculation(input)
        }).await;
        
        // Update UI with result
        this.update(&mut *cx, |view, cx| {
            view.result = result;
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

### Parallel Background Work

```rust
fn parallel_work(&mut self, cx: &mut Context<Self>) {
    let items = self.items.clone();
    
    cx.spawn(async move |this, cx| {
        // Process multiple items in parallel
        let tasks: Vec<_> = items.iter().map(|item| {
            let item = item.clone();
            cx.background_spawn(async move {
                process_item(item)
            })
        }).collect();
        
        // Wait for all tasks
        let results = futures::future::join_all(tasks).await;
        
        // Update UI
        this.update(&mut *cx, |view, cx| {
            view.results = results;
            cx.notify();
        })?;
        
        Ok(())
    }).detach();
}
```

## Task Management

### Task<R> Type

`Task<R>` is a future that can be:
- **Awaited**: Get the result
- **Detached**: Run independently (`.detach()`)
- **Stored**: Cancel when dropped

### Detaching Tasks

```rust
// Task runs independently, won't be cancelled
cx.spawn(async move |this, cx| {
    // Work...
    Ok(())
}).detach();
```

### Storing Tasks

```rust
struct MyView {
    current_task: Option<Task<anyhow::Result<()>>>,
}

impl MyView {
    fn start_task(&mut self, cx: &mut Context<Self>) {
        // Cancel previous task by dropping it
        self.current_task = Some(cx.spawn(async move |this, cx| {
            // Task automatically cancelled if dropped
            tokio::time::sleep(Duration::from_secs(5)).await;
            
            this.update(&mut *cx, |view, cx| {
                view.status = "Done!".into();
                cx.notify();
            })?;
            
            Ok(())
        }));
    }
    
    fn cancel_task(&mut self) {
        // Dropping the task cancels it
        self.current_task = None;
    }
}
```

### Task::ready()

Create a task that immediately provides a value:

```rust
fn immediate_value(cx: &mut Context<Self>) -> Task<String> {
    Task::ready("Immediate result".to_string())
}
```

### Detach with Error Logging

```rust
cx.spawn(async move |this, cx| {
    // Work that might fail
    might_fail().await?;
    Ok(())
}).detach_and_log_err(cx);
```

## Common Patterns

### Loading State

```rust
enum LoadingState<T> {
    Idle,
    Loading,
    Loaded(T),
    Error(String),
}

struct DataView {
    state: LoadingState<String>,
}

impl DataView {
    fn load_data(&mut self, cx: &mut Context<Self>) {
        self.state = LoadingState::Loading;
        cx.notify();
        
        cx.spawn(async move |this, cx| {
            match fetch_data().await {
                Ok(data) => {
                    this.update(&mut *cx, |view, cx| {
                        view.state = LoadingState::Loaded(data);
                        cx.notify();
                    })?;
                }
                Err(e) => {
                    this.update(&mut *cx, |view, cx| {
                        view.state = LoadingState::Error(e.to_string());
                        cx.notify();
                    })?;
                }
            }
            
            Ok(())
        }).detach();
    }
}

impl Render for DataView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.state {
            LoadingState::Idle => div().child("Click to load"),
            LoadingState::Loading => div().child("Loading..."),
            LoadingState::Loaded(data) => div().child(data.clone()),
            LoadingState::Error(err) => div().child(format!("Error: {}", err)),
        }
    }
}
```

### Cancellable Task

```rust
struct Search {
    current_search: Option<Task<anyhow::Result<()>>>,
    results: Vec<String>,
}

impl Search {
    fn search(&mut self, query: String, cx: &mut Context<Self>) {
        // Cancel previous search
        self.current_search = None;
        self.results.clear();
        cx.notify();
        
        self.current_search = Some(cx.spawn(async move |this, cx| {
            let results = cx.background_spawn(async move {
                // Expensive search
                search_items(query)
            }).await;
            
            this.update(&mut *cx, |view, cx| {
                view.results = results;
                cx.notify();
            })?;
            
            Ok(())
        }));
    }
}
```

### Periodic Task

```rust
struct AutoRefresh {
    refresh_task: Option<Task<()>>,
}

impl AutoRefresh {
    fn start_auto_refresh(&mut self, cx: &mut Context<Self>) {
        self.refresh_task = Some(cx.spawn(async move |this, cx| {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                if this.update(&mut *cx, |view, cx| {
                    view.refresh_data(cx);
                }).is_err() {
                    break; // Entity was dropped
                }
            }
        }));
    }
    
    fn stop_auto_refresh(&mut self) {
        self.refresh_task = None;
    }
}
```
