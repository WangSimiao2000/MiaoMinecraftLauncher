# Vertical Rhythm & Visual Consistency

## The Rhythm Formula

Use only these spacing values throughout the app:

| Token | Value | Use |
|---|---|---|
| XS | 4px | Tight gaps within a single component |
| SM | 8px | Between related items in a group |
| MD | 12px | Between groups within a section |
| LG | 16px | Between sections |
| XL | 24px | Major divisions |

## Typography Rhythm

Line heights must be multiples of 4px:

| Style | Size | Line Height |
|---|---|---|
| Title | 22px | 28px |
| Heading | 20px | 24px |
| Subheading | 14px | 20px |
| Body | 13px | 20px |
| Small | 11px | 16px |

## Section Pattern

Every section follows this structure:

```
[subheading]              ← section title
(8px gap)
┌─────────────────────┐  ← section_frame card
│ (12-16px padding)   │
│ content...          │
│                     │
└─────────────────────┘
(16px gap)               ← before next section
```

## Consistency Rules

1. Same type of content = same visual treatment everywhere
2. Same spacing value between all instances of same-level elements
3. Never mix spacing values at the same hierarchy level
4. If one list uses 4px gaps, ALL lists use 4px gaps

## Single-Direction Spacing

Always use spacing BELOW elements, never above:
- `ui.add_space(N)` goes AFTER an element, before the next one
- First element in a section: no top space (container padding handles it)
- Last element: no bottom space (container padding handles it)
