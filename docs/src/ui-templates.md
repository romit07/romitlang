# UI templates and overrides

Romit UI resolves visual tokens at **compile time**. Nothing theme-related is interpreted in the browser.

## Cascade

From weakest to strongest:

1. **Built-in default** — readable contrast, spacing, typography, and a mobile-friendly layout with no config.
2. **Named template** — `use template default` or another name; optional file under `templates/<name>.rmt` next to your page.
3. **Page `theme` block** — overrides under the `page` (for example `color primary "#0a7"`).
4. **Element modifiers** — on a single line, for example `button "Sign up" color blue`.

Unknown theme tokens produce **U006** with the full list of valid token names.

## Built-in tokens

| Token | Role |
|-------|------|
| `color-primary` | Brand accent |
| `color-text` | Body text |
| `color-muted` | Secondary text |
| `color-background` | Page background |
| `color-accent` | Links and highlights |
| `color-button` / `color-button-text` | Primary buttons |
| `space-unit`, `space-section`, `space-gap` | Spacing scale |
| `font-family`, `font-size-base`, `font-size-title`, `font-size-hero` | Typography |
| `radius-button`, `radius-card` | Corner radius |
| `max-width` | Content column width |

Page-level syntax uses two words after the category: `color primary "#0a7"` maps to `color-primary`.

## Custom template files (optional)

If `templates/acme.rmt` exists beside your `.rmt` file:

```text
template acme
  color primary "#1e40af"
  color background "#f8fafc"
  space-section 64px
```

Use it with:

```text
use ui
use template acme
```

If no file is present, Romit keeps the built-in default for that template name (the MVP ships **default** only as a built-in; other names fall through to defaults until you add a file).

## Output

`romit build page.rmt` writes `index.html` and `style.css` under `dist/` (or `--out DIR`). CSS variables in `:root` reflect the resolved cascade.
