"""Convert vendored Lucide SVG shapes to SVG paths for Kurbo rendering."""
import json
from pathlib import Path
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parent
icons = {}
for path in sorted((root / 'lucide').glob('*.svg')):
    paths = []
    for node in ET.parse(path).getroot():
        kind = node.tag.rsplit('}', 1)[-1]
        a = node.attrib
        if kind == 'path':
            paths.append(a['d'])
        elif kind == 'line':
            paths.append(f"M{a['x1']} {a['y1']}L{a['x2']} {a['y2']}")
        elif kind in ('polyline', 'polygon'):
            paths.append('M' + a['points'] + ('Z' if kind == 'polygon' else ''))
        elif kind == 'rect':
            x, y, w, h = (float(a[k]) for k in ('x', 'y', 'width', 'height'))
            r = float(a.get('rx', 0))
            paths.append(f'M{x+r} {y}H{x+w-r}Q{x+w} {y} {x+w} {y+r}V{y+h-r}Q{x+w} {y+h} {x+w-r} {y+h}H{x+r}Q{x} {y+h} {x} {y+h-r}V{y+r}Q{x} {y} {x+r} {y}Z')
        elif kind == 'circle':
            x, y, r = (float(a[k]) for k in ('cx', 'cy', 'r'))
            paths.append(f'M{x-r} {y}a{r} {r} 0 1 0 {2*r} 0a{r} {r} 0 1 0 {-2*r} 0')
        else:
            raise ValueError(f'Unsupported Lucide element: {kind}')
    icons[path.stem] = paths
lines = ['// Generated from vendored Lucide SVGs by build_icons.py.', 'const LUCIDE_PATHS: &[(&str, &[&str])] = &[']
for name, paths in icons.items():
    lines.append('    (' + json.dumps(name) + ', &[' + ', '.join(json.dumps(p) for p in paths) + ']),')
lines.append('];')
(root / 'lucide_paths.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8')
