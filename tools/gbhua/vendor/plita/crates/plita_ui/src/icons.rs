//! Vendored Lucide icons, available without loading files at application startup.
use crate::{Color, ColoredVertex, DrawList, Point, Rect};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

type MeshKey = (String, [u32; 8]);
thread_local! {
    static MESH_CACHE: std::cell::RefCell<BTreeMap<MeshKey, Arc<[ColoredVertex]>>> = const { std::cell::RefCell::new(BTreeMap::new()) };
}

include!("../assets/lucide_paths.rs");

type Lines = Vec<Vec<[f32; 2]>>;
fn paths() -> &'static BTreeMap<&'static str, Lines> {
    static ICONS: OnceLock<BTreeMap<&'static str, Lines>> = OnceLock::new();
    ICONS.get_or_init(|| {
        LUCIDE_PATHS
            .iter()
            .map(|(name, paths)| {
                let mut lines = Vec::new();
                for source in *paths {
                    let path = kurbo::BezPath::from_svg(source).expect("Vendored Lucide path");
                    let mut line = Vec::new();
                    let mut first = [0.; 2];
                    kurbo::flatten(path.elements().iter().copied(), 0.12, |el| match el {
                        kurbo::PathEl::MoveTo(p) => {
                            if !line.is_empty() {
                                lines.push(std::mem::take(&mut line));
                            }
                            first = [p.x as f32, p.y as f32];
                            line.push(first);
                        }
                        kurbo::PathEl::LineTo(p) => line.push([p.x as f32, p.y as f32]),
                        kurbo::PathEl::ClosePath => line.push(first),
                        _ => {}
                    });
                    if !line.is_empty() {
                        lines.push(line);
                    }
                }
                (*name, lines)
            })
            .collect()
    })
}

/// Names in the bundled subset. Original SVGs and license live in `assets/lucide`.
pub fn lucide_icon_names() -> impl ExactSizeIterator<Item = &'static str> {
    LUCIDE_PATHS.iter().map(|(name, _)| *name)
}

/// Draw a centered, aspect-preserving Lucide icon, with round caps and joins.
/// Returns false (and emits nothing) for an unknown name or invalid rectangle.
pub fn draw_lucide_icon(list: &mut DrawList, rect: Rect, name: &str, color: Color) -> bool {
    if [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .any(|n| !n.is_finite())
        || rect.width <= 0.
        || rect.height <= 0.
    {
        return false;
    }
    let Some(lines) = paths().get(name) else {
        return false;
    };
    let key = (
        name.to_string(),
        [
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            color.r,
            color.g,
            color.b,
            color.a,
        ]
        .map(f32::to_bits),
    );
    if let Some(mesh) = MESH_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        list.fill_triangle_list(mesh);
        return true;
    }
    let scale = rect.width.min(rect.height) / 24.;
    let x = rect.x + (rect.width - scale * 24.) / 2.;
    let y = rect.y + (rect.height - scale * 24.) / 2.;
    let mut mesh = Vec::new();
    for line in lines {
        let points: Vec<Point> = line
            .iter()
            .map(|p| Point::new(x + p[0] * scale, y + p[1] * scale))
            .collect();
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len = dx.hypot(dy);
            if len > 0.00001 {
                let (nx, ny) = (-dy * scale / len, dx * scale / len);
                for p in [
                    Point::new(a.x + nx, a.y + ny),
                    Point::new(b.x + nx, b.y + ny),
                    Point::new(b.x - nx, b.y - ny),
                    Point::new(a.x + nx, a.y + ny),
                    Point::new(b.x - nx, b.y - ny),
                    Point::new(a.x - nx, a.y - ny),
                ] {
                    mesh.push(ColoredVertex::new(p, color));
                }
            }
        }
        for center in points {
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::TAU / 12.;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 12.;
                for p in [
                    center,
                    Point::new(center.x + a.cos() * scale, center.y + a.sin() * scale),
                    Point::new(center.x + b.cos() * scale, center.y + b.sin() * scale),
                ] {
                    mesh.push(ColoredVertex::new(p, color));
                }
            }
        }
    }
    let mesh: Arc<[ColoredVertex]> = mesh.into();
    MESH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 128 {
            cache.clear();
        }
        cache.insert(key, Arc::clone(&mesh));
    });
    list.fill_triangle_list(mesh);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_bundled_icons_render_within_their_viewbox() {
        assert!(lucide_icon_names().len() >= 24);
        for name in lucide_icon_names() {
            let mut list = DrawList::new(Color::rgb8(0, 0, 0));
            assert!(draw_lucide_icon(
                &mut list,
                Rect::new(10., 20., 48., 24.),
                name,
                Color::rgb8(255, 255, 255)
            ));
            let crate::DrawCommand::FillTriangles { vertices } = &list.commands()[0] else {
                panic!("Missing icon mesh")
            };
            assert!(!vertices.is_empty(), "{name}");
            for v in vertices.iter() {
                assert!(
                    v.position.x >= 22.
                        && v.position.x <= 46.
                        && v.position.y >= 20.
                        && v.position.y <= 44.,
                    "{name}: {:?}",
                    v.position
                );
            }
        }
    }
    #[test]
    fn repeated_icons_share_the_bounded_cached_mesh() {
        MESH_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let rect = Rect::new(0., 0., 24., 24.);
        for _ in 0..2 {
            assert!(draw_lucide_icon(
                &mut list,
                rect,
                "bug",
                Color::rgb8(255, 255, 255)
            ));
        }
        let crate::DrawCommand::FillTriangles { vertices: a } = &list.commands()[0] else {
            panic!("mesh");
        };
        let crate::DrawCommand::FillTriangles { vertices: b } = &list.commands()[1] else {
            panic!("mesh");
        };
        assert!(Arc::ptr_eq(a, b));
        for x in 0..140 {
            draw_lucide_icon(
                &mut list,
                Rect::new(x as f32, 0., 24., 24.),
                "bug",
                Color::rgb8(255, 255, 255),
            );
        }
        MESH_CACHE.with(|cache| assert!(cache.borrow().len() <= 128));
    }
    #[test]
    fn missing_icons_and_invalid_sizes_are_noops() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        assert!(!draw_lucide_icon(
            &mut list,
            Rect::new(0., 0., 24., 24.),
            "not-an-icon",
            Color::rgb8(255, 255, 255)
        ));
        assert!(!draw_lucide_icon(
            &mut list,
            Rect::new(0., 0., f32::NAN, 24.),
            "save",
            Color::rgb8(255, 255, 255)
        ));
        assert!(list.commands().is_empty());
    }
}
