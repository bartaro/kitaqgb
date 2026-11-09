use super::*;

// The curve is a display aid only. Stored/exported CH3 data stays 32 x 4 bits.
fn curve(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let slope = |x: f32, y: f32| {
        if x * y <= 0.0 {
            0.0
        } else {
            2.0 * x * y / (x + y)
        }
    };
    let m0 = slope(b - a, c - b);
    let m1 = slope(c - b, d - c);
    (2.0 * t * t * t - 3.0 * t * t + 1.0) * b
        + (t * t * t - 2.0 * t * t + t) * m0
        + (-2.0 * t * t * t + 3.0 * t * t) * c
        + (t * t * t - t * t) * m1
}

fn line(list: &mut DrawList, a: Point, b: Point, color: Color) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx.hypot(dy).max(0.001);
    let nx = -dy / len;
    let ny = dx / len;
    let p = [
        Point::new(a.x + nx, a.y + ny),
        Point::new(b.x + nx, b.y + ny),
        Point::new(b.x - nx, b.y - ny),
        Point::new(a.x - nx, a.y - ny),
    ];
    list.fill_triangle_list(
        [0, 1, 2, 0, 2, 3]
            .map(|i| ColoredVertex {
                position: p[i],
                color,
            })
            .to_vec(),
    );
}

fn sample_at(r: Rect, p: Point) -> (usize, u8) {
    let index = ((p.x - r.x) / r.width * WAVE_SAMPLE_COUNT as f32)
        .round()
        .clamp(0.0, (WAVE_SAMPLE_COUNT - 1) as f32) as usize;
    let value = ((r.bottom() - 3.0 - p.y) / (r.height - 6.0) * 15.0)
        .round()
        .clamp(0.0, 15.0) as u8;
    (index, value)
}

impl GbseqApp {
    pub(super) fn edit_wave_at(&mut self, point: Point) -> bool {
        let (Some(rect), Some(wave_index)) =
            (self.wave_editor_rect(), self.active_user_wave_index())
        else {
            return false;
        };
        let (index, value) = sample_at(rect, point);
        let (last_index, last_value) = self.wave_drag_last.unwrap_or((index, value));
        let samples = &mut self.project.user_waves[wave_index].samples;
        let mut changed = false;
        for i in index.min(last_index)..=index.max(last_index) {
            let t = if index == last_index {
                1.0
            } else {
                (i as f32 - last_index as f32) / (index as f32 - last_index as f32)
            };
            let next = (last_value as f32 + (value as f32 - last_value as f32) * t)
                .round()
                .clamp(0.0, 15.0) as u8;
            changed |= samples[i] != next;
            samples[i] = next;
        }
        self.wave_drag_last = Some((index, value));
        changed
    }

    pub(super) fn draw_wave_graph(&self, list: &mut DrawList, r: Rect, samples: &[u8]) {
        list.fill_rect(r, Color::rgb8(14, 18, 22));
        list.push_clip(r);
        for i in 0..=4 {
            list.fill_rect(
                Rect::new(r.x, r.y + i as f32 * r.height / 4.0, r.width, 1.0),
                BORDER.with_alpha(if i == 2 { 0.8 } else { 0.4 }),
            );
        }
        for i in 0..=8 {
            list.fill_rect(
                Rect::new(r.x + i as f32 * r.width / 8.0, r.y, 1.0, r.height),
                BORDER.with_alpha(0.4),
            );
        }
        let point = |i: f32, v: f32| {
            Point::new(
                r.x + i * r.width / samples.len() as f32,
                r.bottom() - 3.0 - v / 15.0 * (r.height - 6.0),
            )
        };
        let get = |i: usize| samples[i % samples.len()] as f32;
        let mut prev = point(0.0, get(0));
        for i in 0..samples.len() {
            let steps = if self.wave_curve { 8 } else { 1 };
            for j in 1..=steps {
                let t = j as f32 / steps as f32;
                let value = if self.wave_curve {
                    curve(
                        get(i + samples.len() - 1),
                        get(i),
                        get(i + 1),
                        get(i + 2),
                        t,
                    )
                } else {
                    get(i) * (1.0 - t) + get(i + 1) * t
                };
                let next = point(i as f32 + t, value);
                line(list, prev, next, track_color(2));
                prev = next;
            }
        }
        for (i, value) in samples.iter().enumerate() {
            let p = point(i as f32, *value as f32);
            list.fill_rounded_rect(Rect::new(p.x - 2.0, p.y - 2.0, 4.0, 4.0), 2.0, TEXT);
        }
        if r.contains(self.hover) {
            let (index, _) = sample_at(r, self.hover);
            let p = point(index as f32, get(index));
            list.fill_rect(Rect::new(p.x, r.y, 1.0, r.height), TEXT.with_alpha(0.4));
            list.fill_rounded_rect(Rect::new(p.x - 3.0, p.y - 3.0, 6.0, 6.0), 3.0, ACCENT);
        }
        list.pop_clip();
        list.stroke_rect(r, 1.0, BORDER);
        self.text(list, r.x + 5.0, r.y + 4.0, "15", 0.75, MUTED);
        self.text(list, r.x + 5.0, r.bottom() - 15.0, "0", 0.75, MUTED);
        let label = if r.contains(self.hover) {
            let (index, _) = sample_at(r, self.hover);
            format!("POINT {}  VALUE {}", index, samples[index])
        } else {
            "DRAG TO DRAW / 32 SAMPLES / 0-15".into()
        };
        self.text(list, r.x, r.bottom() + 8.0, &label, 0.78, MUTED);
    }
}
