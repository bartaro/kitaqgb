# Shared Lucide Icons

PLITA owns the SVG sources in `lucide/`, their upstream LICENSE, the generated
`lucide_paths.rs`, and the renderer in `src/icons.rs`. Wire3D CAD consumes this
shared renderer; no CAD-specific icon files are required at runtime.

Source: https://github.com/lucide-icons/lucide/tree/main/icons (retrieved 2026-09-07).
Licenses: `lucide/LICENSE` contains the Lucide ISC and inherited Feather MIT terms.

```rust
use plita::prelude::*;

let mut list = DrawList::new(Color::rgb8(24, 24, 27));
draw_lucide_icon(&mut list, Rect::new(12.0, 12.0, 24.0, 24.0),
                 "save", Color::rgb8(230, 230, 235));
```

`draw_lucide_icon` preserves aspect ratio, centers the icon, scales its stroke,
and returns false for an unknown name or invalid rectangle. The enclosing
application owns the button hit area, command, disabled state, and tooltip.
`lucide_icon_names()` enumerates the 31 bundled names. Curves are flattened once,
cached, and emitted through the normal PLITA DrawList renderer with round joins.
The seven desktop additions (step-forward, camera, bug, volume-2, settings,
maximize and ellipsis) were retrieved from the same upstream on 2026-09-09.
Repeated icon meshes are cached per thread by name, rectangle and color, with
a bounded 128-entry cache; animated game frames do not retessellate unchanged
toolbar icons.

To regenerate after adding an upstream SVG:

```powershell
python crates/plita_ui/assets/build_icons.py
```

Keep upstream SVGs and the license when redistributing source. The generated
paths are embedded in applications; no network access or asset installation is
needed at runtime.
