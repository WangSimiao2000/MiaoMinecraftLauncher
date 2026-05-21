# Card & List Width Rules

## The Fundamental Rule

**All cards/list items in a vertical list MUST span 100% of the container width.**

Content length varies, but the card container does NOT. Text within truncates or wraps.

## Decision Framework

| Layout Context | Width Rule |
|---|---|
| Vertical list (scrollable) | 100% of container width |
| Card collection (single-column) | Constant width, variable height |
| Card grid (multi-column) | Fill the grid column |
| Content-width cards | NEVER in a vertical list |

## Rules

1. All cards in a vertical list span 100% of the scrollable container width (minus padding)
2. Height may vary between cards, width MUST NOT
3. Container padding: 12-16px on desktop
4. Gap between cards: 4-8px (compact list items), 12-16px (section cards)
5. Internal card padding: 12-16px all sides
6. Long text: truncate with ellipsis or wrap — never expand the card

## Implementation (egui)

```rust
frame.show(ui, |ui| {
    ui.set_min_width(ui.available_width());  // FIRST LINE — forces full width
    // ... content
});
```

## Anti-Pattern

```
❌ Cards sized by content (variable widths)
┌──────────────┐
│ Short text   │
└──────────────┘
┌──────────────────────────────┐
│ Much longer text content     │
└──────────────────────────────┘

✅ Cards fill container (uniform widths)
┌────────────────────────────────────┐
│ Short text                         │
└────────────────────────────────────┘
┌────────────────────────────────────┐
│ Much longer text content           │
└────────────────────────────────────┘
```
