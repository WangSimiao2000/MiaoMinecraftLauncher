# Layout & Positioning Rules

## Spacing Scale (4px base)

All spacing must be a multiple of 4px:
```
4, 8, 12, 16, 24, 32, 48
```

## Spacing Hierarchy

Spacing between elements increases with hierarchy level (1.5-2x multiplier):

| Level | Value | Use |
|---|---|---|
| Within component | 4-8px | Internal padding, label-to-value |
| Between siblings | 8-12px | Items in same group |
| Between groups | 16-24px | Groups within a section |
| Between sections | 24-32px | Top-level sections |

## Edge Alignment

1. All content within a panel shares the same left edge
2. Maximum 2-3 vertical alignment edges per panel
3. Right-align only: numeric data, commit buttons
4. Sub-items indent by exactly 1 spacing unit (12-16px)

## Width Variants

Limit to 2-3 distinct content widths per view:

| Count | Acceptable |
|---|---|
| 1 width | Forms, settings, lists |
| 2 widths | Main + sidebar |
| 3 widths | Dashboard with sidebar + main + aside |
| 4+ widths | AVOID |

## Container Hierarchy

```
CONTAINER (Panel)
├── padding: 12-16px
├── SECTION
│   ├── margin-bottom: 16-24px
│   ├── ITEM (margin-bottom: 4-8px)
│   ├── ITEM
│   └── ITEM
└── SECTION
```

## Panel Sizing

| Panel | Min Width | Default |
|---|---|---|
| Side navigation | 140px | 150-240px |
| Main content | 400px | Fill remaining |
| Content max width | — | 600-800px |

## Visual Anchoring Checklist

- [ ] Every element aligns to at least one grid line
- [ ] Only 3-4 spacing values used in entire view
- [ ] Related elements: tight spacing (≤12px)
- [ ] Unrelated elements: wide spacing (≥24px)
- [ ] Containment (cards/borders) groups related items
- [ ] Consistent repetition of patterns

## Form Layout

- Labels above inputs (default for desktop)
- Labels left-aligned
- Input height: 28px (our CONTROL_HEIGHT)
- Between inputs: 8-12px
- Between form sections: 16-24px
- Max 7±2 fields per section
