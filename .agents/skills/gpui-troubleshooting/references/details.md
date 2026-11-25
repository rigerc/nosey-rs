# GPUI Troubleshooting: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Entity Errors

### Update While Updating Panic

**Error**: `already mutably borrowed: BorrowError`

**Cause**: Trying to update an entity while it's already being updated

**Solution**: Avoid nested entity updates

```rust
// ❌ WRONG - will panic
entity.update(cx, |view, cx| {
    entity.update(cx, |view2, cx2| {
        // Nested update - PANIC!
    });
});

// ✅ CORRECT - restructure logic
entity.update(cx, |view, cx| {
    view.prepare_update();
});
// Update happens after first update completes
entity.update(cx, |view, cx| {
    view.apply_update();
});
```

### WeakEntity Upgrade Failure

**Error**: Panic when calling methods on `None`

**Solution**: Always check `upgrade()` result

```rust
// ❌ WRONG
let entity = weak.upgrade().unwrap(); // May panic!

// ✅ CORRECT
if let Some(entity) = weak.upgrade() {
    entity.update(cx, |view, cx| {
        // ...
    });
} else {
    // Entity was dropped
}

// ✅ ALSO CORRECT (in async context)
this.update(&mut *cx, |view, cx| {
    // ...
})?; // Propagate error if entity gone
```

## Async Errors

### "Nothing Left to Run" in Tests

**Error**: `run_until_parked()` completes but test fails

**Cause**: Using `smol::Timer` instead of GPUI timers

**Solution**: Use GPUI executor timers

```rust
// ❌ WRONG
smol::Timer::after(Duration::from_secs(1)).await;

// ✅ CORRECT
cx.background_executor.timer(Duration::from_secs(1)).await;
```

### Task Dropped Before Completion

**Error**: Async work doesn't complete

**Cause**: Task dropped without being awaited or detached

**Solution**: Either detach or store the task

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

// ✅ ALSO CORRECT - store
self.current_task = Some(cx.spawn(async move |this, cx| {
    // Work...
    Ok(())
}));
```

### Async Context Borrow Error

**Error**: Cannot dereference `cx` in async context

**Solution**: Use `&mut *cx` to dereference

```rust
// ❌ WRONG
cx.spawn(async move |this, cx| {
    this.update(cx, |view, cx| { // ERROR!
        // ...
    });
});

// ✅ CORRECT
cx.spawn(async move |this, cx| {
    this.update(&mut *cx, |view, cx| {
        // ...
    })?;
    Ok(())
});
```

## UI Not Updating

### Forgetting cx.notify()

**Symptom**: State changes but UI doesn't update

**Solution**: Call `cx.notify()` after state changes

```rust
// ❌ WRONG
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    // Missing cx.notify() - UI won't update!
}

// ✅ CORRECT
fn increment(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    cx.notify(); // Trigger re-render
}
```

### Subscription Dropped

**Symptom**: Events stop being received

**Solution**: Store `Subscription` in a field

```rust
// ❌ WRONG
impl Parent {
    fn new(cx: &mut Context<Self>) -> Self {
        let child = cx.new(|_| Child::new());
        
        cx.subscribe(&child, |this, _child, event, cx| {
            // Handle event
        }); // Subscription dropped here!
        
        Self { child }
    }
}

// ✅ CORRECT
struct Parent {
    child: Entity<Child>,
    _subscription: Subscription, // Store to keep alive
}

impl Parent {
    fn new(cx: &mut Context<Self>) -> Self {
        let child = cx.new(|_| Child::new());
        
        let subscription = cx.subscribe(&child, |this, _child, event, cx| {
            // Handle event
        });
        
        Self {
            child,
            _subscription: subscription,
        }
    }
}
```
