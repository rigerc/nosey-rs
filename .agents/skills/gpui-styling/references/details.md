# GPUI Styling: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Flexbox Layout

### Flex Direction

```rust
div()
    .flex()                     // Enable flexbox
    .flex_row()                 // Horizontal (default)
    .flex_col()                 // Vertical
    
// Short forms (from gpui-component)
div()
    .v_flex()                   // flex + flex_col
    .h_flex()                   // flex + flex_row
```

### Alignment

```rust
div()
    .flex()
    .items_start()              // Align items to start
    .items_center()             // Center items
    .items_end()                // Align items to end
    .items_stretch()            // Stretch items
    
    .justify_start()            // Justify to start
    .justify_center()           // Center justify
    .justify_end()              // Justify to end
    .justify_between()          // Space between
    .justify_around()           // Space around
```

### Flex Properties

```rust
div()
    .flex_1()                   // flex: 1 (grow to fill)
    .flex_grow()                // Allow growing
    .flex_shrink()              // Allow shrinking
    .flex_none()                // Don't grow or shrink
    .flex_wrap()                // Wrap items
```

## Spacing

### Padding

```rust
div()
    .p(px(16.0))                // All sides
    .p_1()                      // 4px (0.25rem)
    .p_2()                      // 8px (0.5rem)
    .p_4()                      // 16px (1rem)
    .p_6()                      // 24px (1.5rem)
    .p_8()                      // 32px (2rem)
    
    .px(px(16.0))               // Horizontal padding
    .py(px(16.0))               // Vertical padding
    .px_4()                     // Horizontal 16px
    .py_2()                     // Vertical 8px
    
    .pt(px(8.0))                // Top padding
    .pr(px(8.0))                // Right padding
    .pb(px(8.0))                // Bottom padding
    .pl(px(8.0))                // Left padding
```

### Margin

```rust
div()
    .m(px(16.0))                // All sides
    .m_4()                      // 16px
    
    .mx(px(16.0))               // Horizontal margin
    .my(px(16.0))               // Vertical margin
    .mx_4()                     // Horizontal 16px
    
    .mt(px(8.0))                // Top margin
    .mr(px(8.0))                // Right margin
    .mb(px(8.0))                // Bottom margin
    .ml(px(8.0))                // Left margin
```

### Gap

```rust
div()
    .flex()
    .flex_col()
    .gap(px(16.0))              // Gap between children
    .gap_1()                    // 4px
    .gap_2()                    // 8px
    .gap_4()                    // 16px
    .gap_6()                    // 24px
```

## Common Patterns

### Card Component

```rust
div()
    .p_6()
    .bg(rgb(0x1a1a1a))
    .rounded_lg()
    .border_1()
    .border_color(rgb(0x2a2a2a))
    .flex()
    .flex_col()
    .gap_4()
```

### Button

```rust
div()
    .px_6()
    .py_3()
    .bg(rgb(0x3b82f6))
    .text_color(rgb(0xffffff))
    .rounded_md()
    .cursor_pointer()
    .font_semibold()
    .hover(|style| {
        style.bg(rgb(0x2563eb))
    })
```

### Input Field

```rust
div()
    .w_full()
    .px_3()
    .py_2()
    .bg(rgb(0x1a1a1a))
    .border_1()
    .border_color(rgb(0x4a4a4a))
    .rounded_md()
    .text_base()
```

### Navbar

```rust
div()
    .w_full()
    .h(px(64.0))
    .px_6()
    .bg(rgb(0x1a1a1a))
    .flex()
    .flex_row()
    .items_center()
    .justify_between()
    .border_b_1()
    .border_color(rgb(0x2a2a2a))
```

### Centered Container

```rust
div()
    .flex()
    .items_center()
    .justify_center()
    .size_full()
```

### Grid Layout

```rust
div()
    .flex()
    .flex_wrap()
    .gap_4()
    .children((0..12).map(|i| {
        div()
            .w(px(200.0))
            .h(px(150.0))
            .bg(rgb(0x3b82f6))
            .rounded_md()
    }))
```

## Color Utilities

### Common Colors

```rust
// Grays
rgb(0x1a1a1a)  // Dark gray
rgb(0x2a2a2a)  // Medium dark gray
rgb(0x4a4a4a)  // Medium gray
rgb(0x9ca3af)  // Light gray
rgb(0xf3f4f6)  // Very light gray

// Blues
rgb(0x3b82f6)  // Primary blue
rgb(0x2563eb)  // Darker blue
rgb(0x60a5fa)  // Lighter blue

// Greens
rgb(0x22c55e)  // Success green
rgb(0x16a34a)  // Darker green

// Reds
rgb(0xef4444)  // Error red
rgb(0xdc2626)  // Darker red

// Yellows
rgb(0xf59e0b)  // Warning yellow
```

### HSLA Colors

```rust
// More flexible color manipulation
hsla(0.0, 0.0, 0.1, 1.0)    // Dark gray
hsla(0.6, 0.8, 0.5, 1.0)    // Blue
hsla(0.33, 0.7, 0.5, 1.0)   // Green
hsla(0.0, 0.8, 0.6, 1.0)    // Red
hsla(0.15, 0.9, 0.5, 1.0)   // Yellow/Orange
```
