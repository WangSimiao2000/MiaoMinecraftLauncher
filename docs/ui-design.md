# UI Design System

## Control Height — The Fundamental Rule

All interactive controls on the same horizontal row MUST share the same total height.

| Token | Height | Use Case |
|-------|--------|----------|
| `CONTROL_HEIGHT` | **28px** | All buttons, text inputs, combo boxes |
| `CONTROL_HEIGHT_SMALL` | 22px | Only for inline list actions (Del in table rows) |

## Grid System — 4px Base

Every dimension (height, width, padding, margin, gap) must be a multiple of 4px.

```
Scale: 4, 8, 12, 16, 20, 24, 28, 32, 40, 48
```

## Spacing Tokens

| Token | Value | Use |
|-------|-------|-----|
| `ITEM_SPACING` | 8×8 | Default gap between adjacent widgets |
| `BUTTON_PADDING` | 12×6 | Internal padding for buttons |
| `SECTION_GAP` | 16 | Between section cards |
| `SMALL_GAP` | 6 | Between related items |
| `PANEL_MARGIN` | 12 | Panel internal margin |
| `WINDOW_MARGIN` | 14 | Window internal margin |

### Spacing Hierarchy

| Level | Value | Use |
|---|---|---|
| XS | 4px | Tight gaps within a single component |
| SM | 8px | Between related items in a group |
| MD | 12px | Between groups within a section |
| LG | 16px | Between sections |
| XL | 24px | Major divisions |

## Typography

| Style | Size | Line Height | Use |
|-------|------|-------------|-----|
| Title | 22px Bold | 28px | Instance name, page title |
| Heading | 20px Bold | 24px | Section heading |
| Subheading | 14px Bold | 20px | Card headers |
| Body | 13px | 20px | Primary content |
| Small | 11px | 16px | Hints, secondary info |
| Button | 14px | — | Button labels |

Font stack: Inter Medium (English) + Noto Sans CJK SC (Chinese fallback).

## Color Layers (Dark Theme)

| Layer | RGB | Use |
|-------|-----|-----|
| BG_DARK | 22,24,30 | Top/bottom bars |
| BG_PANEL | 26,29,36 | Sidebar |
| BG_MAIN | 30,33,40 | Central panel |
| BG_ELEVATED | 38,42,52 | Cards |
| BG_WIDGET | 45,50,60 | Inputs, inactive buttons |
| BG_WIDGET_HOVER | 60,70,85 | Hover state |

### Semantic Colors

| Name | RGB | Use |
|------|-----|-----|
| ACCENT | 75,130,195 | Primary action, selection |
| ACCENT_LIGHT | 120,180,255 | Active tab, links |
| SUCCESS | 80,180,80 | Enabled state, launch button |
| DANGER | 200,80,80 | Delete, error |
| WARNING | 220,170,50 | Caution |

## Corner Radius

| Element | Radius |
|---------|--------|
| Widgets | 4px |
| Cards/Sections | 8px |
| List items | 6px |

## Shadow

| Element | Offset | Blur | Alpha |
|---------|--------|------|-------|
| Window | 0,6 | 20 | 80 |
| Popup | 0,8 | 24 | 100 |

## Horizontal Row Alignment

- **Rule A**: All widgets in `ui.horizontal()` resolve to the same height via `interact_size.y = 28px`.
- **Rule B**: TextEdit must match button height using `.min_size(interact_size)`.
- **Rule C**: No mixing `small_button` and `button` in same row.
- **Rule D**: Label-to-control gap = 8px (1 item_spacing unit).

## Card & List Width Rules

All cards/list items in a vertical list MUST span 100% of the container width.

```rust
frame.show(ui, |ui| {
    ui.set_min_width(ui.available_width());
    // ... content
});
```

| Layout Context | Width Rule |
|---|---|
| Vertical list (scrollable) | 100% of container width |
| Card collection (single-column) | Constant width, variable height |
| Content-width cards | NEVER in a vertical list |

## Section Pattern

```
[subheading]              ← section title
(8px gap)
┌─────────────────────┐  ← section_frame card
│ (12-16px padding)   │
│ content...          │
└─────────────────────┘
(16px gap)               ← before next section
```

## Layout Positioning

### Edge Alignment

1. All content within a panel shares the same left edge
2. Maximum 2-3 vertical alignment edges per panel
3. Right-align only: numeric data, commit buttons
4. Sub-items indent by exactly 1 spacing unit (12-16px)

### Panel Sizing

| Panel | Min Width | Default |
|---|---|---|
| Side navigation | 140px | 150-240px |
| Main content | 400px | Fill remaining |
| Content max width | — | 600-800px |

### Container Hierarchy

```
CONTAINER (Panel)
├── padding: 12-16px
├── SECTION
│   ├── margin-bottom: 16-24px
│   ├── ITEM (margin-bottom: 4-8px)
│   └── ITEM
└── SECTION
```

## Vertical Rhythm

Single-direction spacing: always use spacing BELOW elements, never above.

- `ui.add_space(N)` goes AFTER an element, before the next one
- First element in a section: no top space (container padding handles it)
- Last element: no bottom space (container padding handles it)

## Interaction Patterns

- Pointer cursor (`PointingHand`) on all interactive elements
- Hover highlight on list items and cards (subtle `from_white_alpha(8)` overlay)
- Selected state uses `BG_WIDGET_HOVER`
- Tab indicator: accent-colored underline (2.5px height)
- Search toggle: accent-filled button when active
- Settings: left sidebar tabs with `SidePanel::show_inside`

## Layout Pitfalls

### `with_layout(right_to_left)` Height Explosion

**Problem**: Using `ui.with_layout(Layout::right_to_left(Align::Center), ...)` directly inside a vertical parent (e.g. `section_frame`) causes the child UI to consume **all remaining vertical space**. egui needs the full height to center-align vertically, so the enclosing card stretches far beyond its content.

**Fix**: Always wrap in `ui.horizontal(...)` first. The horizontal constrains the height to one row, then right-to-left alignment works within that single row.

```rust
// ✗ WRONG — card height explodes
theme::section_frame().show(ui, |ui| {
    ui.label("some content");
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.button("Save");
    });
});

// ✓ CORRECT — button stays in a single row
theme::section_frame().show(ui, |ui| {
    ui.label("some content");
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.button("Save");
        });
    });
});
```

### Settings Page Card Layout Conventions

| Element | Pattern |
|---------|---------|
| Multi-option selection | Grid cards (equal-width columns), like theme/language pickers |
| Input fields | Full-width, vertical stack, each with a label above |
| Action buttons | Right-aligned via `horizontal` + `with_layout(right_to_left)` |
| Section separation | Separate `section_frame` cards, NOT `separator()` within one card |
| Related settings group | Single `section_frame` with sub-labels (`theme::small`) dividing groups |
