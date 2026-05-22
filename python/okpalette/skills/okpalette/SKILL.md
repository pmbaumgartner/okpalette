---
name: okpalette
description: Use when generating deterministic categorical color palettes with okpalette for plots, dashboards, labels, or reports.
---

# Okpalette

Okpalette creates deterministic OKLab categorical color palettes. Use it for simple palette
creation or extension when a user needs stable colors that can be consumed by plotting tools or
other code.

## Invocation Policy

- Prefer the CLI for simple create or extend requests when JSON colors are enough.
- Prefer the Python API for label-aware palettes, plotting previews, custom lightness/chroma/hue
  constraints, notebooks, or richer workflows.
- Treat CLI stdout as JSON only. Do not parse plain text colors from stdout.
- Describe `colorblind_mode` as colorblind-aware generation under selected CVD simulations, not as
  colorblind-safe or universally accessible output.

## CLI

Create a palette:

```bash
okpalette create 8
```

The success output is JSON:

```json
{"colors":["#080050","#e00800"],"format":"hex"}
```

Select RGB tuple formats when the caller asks for them:

```bash
okpalette create 5 --format rgb
okpalette create 5 --format rgb01
```

Extend existing colors:

```bash
okpalette extend 8 --color "#0057b8" --color "#ffd700"
```

Return only generated colors while using existing colors as anchors:

```bash
okpalette extend 6 --color "#0057b8" --generated-only
```

Use background filtering only when both options are present:

```bash
okpalette create 12 --background "#ffffff" --background-contrast wcag
```

Use colorblind-aware generation when requested:

```bash
okpalette create 12 --colorblind-mode red-green
okpalette create 12 --colorblind-mode all
```

Supported CLI modes are `protan`, `deutan`, `tritan`, `red-green`, and `all`. Omit
`--colorblind-mode` for ordinary generation.

## Python API

Use the Python API when the CLI surface is too small:

```python
from okpalette import create_label_palette, create_palette, extend_palette

colors = create_palette(8)
extended = extend_palette(["#0057b8", "#ffd700"], 12)
label_colors = create_label_palette(positions, labels)
```

Do not invent CLI flags for Python-only options. Use Python code instead.
