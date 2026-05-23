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
- Use `extend`, not `create --seed-color`, when the user's existing colors must appear in the
  returned palette. Seed colors are distance anchors.
- Describe `colorblind_mode` as colorblind-aware generation under selected CVD simulations, not as
  colorblind-safe or universally accessible output.

## Common Workflows

### User Wants Colors

For "give me N colors" or "make a categorical palette", use the CLI:

```bash
okpalette create 8
```

Return the `colors` array unless the user specifically wants the full JSON payload.

### User Has Existing Colors

If existing colors should stay in the output, extend them:

```bash
okpalette extend 8 --color "#0057b8" --color "#ffd700"
```

If existing colors should only guide generation, return generated colors only:

```bash
okpalette extend 6 --color "#0057b8" --generated-only
```

### User Has Labels And Positions

Use Python for position-aware label colors:

```python
from okpalette import create_label_palette

positions = [(0.0, 0.0), (0.2, 0.0), (5.0, 0.0), (5.2, 0.0)]
labels = ["control", "treated", "control", "outlier"]

colors = create_label_palette(positions, labels)
```

Use `fixed_colors` when specific labels must keep specific colors:

```python
colors = create_label_palette(
    positions,
    labels,
    fixed_colors={"control": "#0057b8"},
)
```

### User Wants Plotting Code

Use plain hex strings for most plotting libraries:

```python
from okpalette import create_palette

categories = ["control", "treated", "outlier"]
colors = create_palette(len(categories))
color_map = dict(zip(categories, colors))
```

Use `format="rgb01"` only when an API requires normalized RGB tuples.

## CLI Reference

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

Supported CLI modes are `protan`, `deutan`, `tritan`, `red-green`, and `all`. 

## Python API

Use the Python API when the CLI surface is too small:

```python
from okpalette import create_label_palette, create_palette, extend_palette

colors = create_palette(8)
extended = extend_palette(["#0057b8", "#ffd700"], 12)
label_colors = create_label_palette(positions, labels)
```

Do not invent CLI flags for Python-only options. Use Python code instead.
