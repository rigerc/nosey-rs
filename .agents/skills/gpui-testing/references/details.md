# GPUI Testing: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Testing Async Operations

### Background Task Completion

```rust
#[gpui::test]
async fn test_background_task(cx: &mut TestAppContext) {
    let view = cx.new(|_| MyView::new());
    
    view.update(cx, |view, cx| {
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async {
                // Expensive computation
                42
            }).await;
            
            this.update(&mut *cx, |view, cx| {
                view.result = Some(result);
                cx.notify();
            })?;
            
            Ok(())
        }).detach();
    });
    
    // Wait for all async work
    cx.background_executor.run_until_parked();
    
    assert_eq!(view.read(cx).result, Some(42));
}
```

### Testing Task Cancellation

```rust
#[gpui::test]
async fn test_task_cancellation(cx: &mut TestAppContext) {
    let view = cx.new(|_| MyView::new());
    
    view.update(cx, |view, cx| {
        view.start_long_task(cx);
    });
    
    // Cancel task
    view.update(cx, |view, cx| {
        view.cancel_task();
    });
    
    cx.background_executor.run_until_parked();
    
    // Task should not have completed
    assert_eq!(view.read(cx).task_completed, false);
}
```

## Assertions and Expectations

### Entity State Assertions

```rust
#[gpui::test]
async fn test_state_changes(cx: &mut TestAppContext) {
    let view = cx.new(|_| MyView::new());
    
    // Initial state
    assert_eq!(view.read(cx).count, 0);
    
    // After update
    view.update(cx, |view, cx| {
        view.count = 10;
        cx.notify();
    });
    
    assert_eq!(view.read(cx).count, 10);
}
```

### Using assert Macros

```rust
#[gpui::test]
async fn test_with_assertions(cx: &mut TestAppContext) {
    let view = cx.new(|_| MyView::new());
    
    view.update(cx, |view, cx| {
        view.process_data(vec![1, 2, 3], cx);
    });
    
    cx.background_executor.run_until_parked();
    
    let view_state = view.read(cx);
    assert!(view_state.is_processed);
    assert_eq!(view_state.items.len(), 3);
    assert!(view_state.error.is_none());
}
```

## Testing Patterns

### Setup and Teardown

```rust
#[gpui::test]
async fn test_with_setup(cx: &mut TestAppContext) {
    // Setup
    let state = cx.new(|_| AppState::default());
    cx.set_global(state.clone());
    
    // Test
    let view = cx.new(|_| MyView::new());
    view.update(cx, |view, cx| {
        view.use_global_state(cx);
    });
    
    // Assertions
    assert!(view.read(cx).has_state);
    
    // Teardown is automatic when test ends
}
```

### Testing Error Cases

```rust
#[gpui::test]
async fn test_error_handling(cx: &mut TestAppContext) {
    let view = cx.new(|_| MyView::new());
    
    view.update(cx, |view, cx| {
        cx.spawn(async move |this, cx| {
            // Simulate error
            let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("Test error"));
            
            this.update(&mut *cx, |view, cx| {
                view.error = result.err().map(|e| e.to_string());
                cx.notify();
            })?;
            
            Ok(())
        }).detach();
    });
    
    cx.background_executor.run_until_parked();
    
    assert!(view.read(cx).error.is_some());
}
```

## Example Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    #[gpui::test]
    async fn test_counter_initialization(cx: &mut TestAppContext) {
        let counter = cx.new(|_| Counter { count: 0 });
        assert_eq!(counter.read(cx).count, 0);
    }

    #[gpui::test]
    async fn test_counter_increment(cx: &mut TestAppContext) {
        let counter = cx.new(|_| Counter { count: 0 });
        
        counter.update(cx, |counter, cx| {
            counter.increment(cx);
        });
        
        assert_eq!(counter.read(cx).count, 1);
    }

    #[gpui::test]
    async fn test_counter_async_increment(cx: &mut TestAppContext) {
        let counter = cx.new(|_| Counter { count: 0 });
        
        counter.update(cx, |counter, cx| {
            cx.spawn(async move |this, cx| {
                // Simulate async delay
                cx.background_executor.timer(Duration::from_millis(100)).await;
                
                this.update(&mut *cx, |counter, cx| {
                    counter.count += 1;
                    cx.notify();
                })?;
                
                Ok(())
            }).detach();
        });
        
        cx.background_executor.run_until_parked();
        assert_eq!(counter.read(cx).count, 1);
    }

    #[gpui::test]
    async fn test_counter_notify(cx: &mut TestAppContext) {
        let counter = cx.new(|_| Counter { count: 0 });
        let notified = Arc::new(AtomicBool::new(false));
        let notified_clone = notified.clone();
        
        cx.observe(&counter, move |_counter, _cx| {
            notified_clone.store(true, Ordering::SeqCst);
        });
        
        counter.update(cx, |counter, cx| {
            counter.count = 5;
            cx.notify();
        });
        
        assert!(notified.load(Ordering::SeqCst));
    }
}
```
