//! Deterministic CPU rendering of the same DrawList submitted to the native UI.
use super::*;

pub fn save(list: &DrawList, size: Size, path: &Path) -> std::result::Result<(), String> {
    let (w, h) = (size.width as u32, size.height as u32);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return Err("Invalid snapshot size".into());
    }
    let c = list.clear_color;
    let clear = [
        (c.r * 255.).round() as u8,
        (c.g * 255.).round() as u8,
        (c.b * 255.).round() as u8,
        (c.a * 255.).round() as u8,
    ];
    let mut data = clear.repeat(w as usize * h as usize);
    let mut clips = vec![Rect::new(0., 0., w as f32, h as f32)];
    for command in list.commands() {
        let clip = *clips.last().unwrap();
        match command {
            DrawCommand::PushClip { rect } => {
                let (x, y) = (rect.x.max(clip.x), rect.y.max(clip.y));
                clips.push(Rect::new(
                    x,
                    y,
                    (rect.right().min(clip.right()) - x).max(0.),
                    (rect.bottom().min(clip.bottom()) - y).max(0.),
                ));
            }
            DrawCommand::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            DrawCommand::FillRect { rect, color } => {
                raster(&mut data, w, h, *rect, clip, |_, _| *color)
            }
            DrawCommand::FillRectCorners {
                rect,
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => raster(&mut data, w, h, *rect, clip, |x, y| {
                let tx = (x - rect.x) / rect.width;
                let ty = (y - rect.y) / rect.height;
                mix(
                    mix(*top_left, *top_right, tx),
                    mix(*bottom_left, *bottom_right, tx),
                    ty,
                )
            }),
            DrawCommand::FillTriangles { vertices } => {
                for triangle in vertices.chunks_exact(3) {
                    let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
                    let cross = |a: Point, b: Point, c: Point| {
                        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
                    };
                    let area = cross(a.position, b.position, c.position);
                    if area.abs() < 0.00001 {
                        continue;
                    }
                    let x = a.position.x.min(b.position.x).min(c.position.x);
                    let y = a.position.y.min(b.position.y).min(c.position.y);
                    let rect = Rect::new(
                        x,
                        y,
                        a.position.x.max(b.position.x).max(c.position.x) - x,
                        a.position.y.max(b.position.y).max(c.position.y) - y,
                    );
                    raster(&mut data, w, h, rect, clip, |x, y| {
                        let p = Point::new(x, y);
                        let wa = cross(b.position, c.position, p) / area;
                        let wb = cross(c.position, a.position, p) / area;
                        let wc = 1. - wa - wb;
                        if wa < -0.0001 || wb < -0.0001 || wc < -0.0001 {
                            return Color::rgba8(0, 0, 0, 0);
                        }
                        Color {
                            r: a.color.r * wa + b.color.r * wb + c.color.r * wc,
                            g: a.color.g * wa + b.color.g * wb + c.color.g * wc,
                            b: a.color.b * wa + b.color.b * wb + c.color.b * wc,
                            a: a.color.a * wa + b.color.a * wb + c.color.a * wc,
                        }
                    });
                }
            }
            DrawCommand::ImageRgba {
                rect,
                width,
                height,
                pixels,
            } => blit(&mut data, w, h, *rect, clip, *width, *height, pixels),
            DrawCommand::ImageRgbaShared {
                rect,
                width,
                height,
                pixels,
            } => blit(
                &mut data,
                w,
                h,
                *rect,
                clip,
                *width,
                *height,
                &pixels.lock().map_err(|e| e.to_string())?,
            ),
        }
    }
    image::RgbaImage::from_raw(w, h, data)
        .ok_or("Invalid RGBA buffer")?
        .save(path)
        .map_err(|e| e.to_string())
}
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
fn raster(
    data: &mut [u8],
    w: u32,
    h: u32,
    rect: Rect,
    clip: Rect,
    mut sample: impl FnMut(f32, f32) -> Color,
) {
    let x0 = rect.x.max(clip.x).max(0.).floor() as u32;
    let y0 = rect.y.max(clip.y).max(0.).floor() as u32;
    let x1 = rect.right().min(clip.right()).min(w as f32).ceil().max(0.) as u32;
    let y1 = rect
        .bottom()
        .min(clip.bottom())
        .min(h as f32)
        .ceil()
        .max(0.) as u32;
    for y in y0..y1 {
        for x in x0..x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            if px < rect.x
                || px >= rect.right()
                || py < rect.y
                || py >= rect.bottom()
                || px < clip.x
                || px >= clip.right()
                || py < clip.y
                || py >= clip.bottom()
            {
                continue;
            }
            let c = sample(px, py);
            let i = ((y * w + x) * 4) as usize;
            for (n, v) in [c.r, c.g, c.b].into_iter().enumerate() {
                data[i + n] = (v * c.a * 255. + data[i + n] as f32 * (1. - c.a))
                    .round()
                    .clamp(0., 255.) as u8;
            }
            data[i + 3] = (c.a * 255. + data[i + 3] as f32 * (1. - c.a))
                .round()
                .clamp(0., 255.) as u8;
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn blit(data: &mut [u8], w: u32, h: u32, rect: Rect, clip: Rect, iw: u32, ih: u32, pixels: &[u8]) {
    if iw == 0 || ih == 0 {
        return;
    }
    raster(data, w, h, rect, clip, |x, y| {
        let sx = (((x - rect.x) / rect.width * iw as f32) as u32).min(iw - 1);
        let sy = (((y - rect.y) / rect.height * ih as f32) as u32).min(ih - 1);
        let i = ((sy * iw + sx) * 4) as usize;
        if i + 3 >= pixels.len() {
            Color::rgba8(0, 0, 0, 0)
        } else {
            Color::rgba8(pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3])
        }
    });
}
