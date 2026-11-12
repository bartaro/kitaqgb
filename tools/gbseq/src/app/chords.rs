use super::*;
use std::{cell::RefCell, collections::BTreeMap};
#[derive(Default)]
pub(super) struct Labels {
    pub mix: bool,
    pub lanes: u32,
    cache: RefCell<BTreeMap<(Option<usize>, u32), Option<String>>>,
}
impl Labels {
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
    pub fn extra(&self) -> f32 {
        if self.mix {
            22.
        } else {
            0.
        }
    }
    fn at(&self, p: &SequenceProject, lane: Option<usize>, beat: u32) -> Option<String> {
        self.cache
            .borrow_mut()
            .entry((lane, beat))
            .or_insert_with(|| crate::harmony::chord(p, lane, beat))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_and_individual_overlays_are_independent() {
        let mut app = GbseqApp::new_with_audio(Size::new(1000., 700.), false);
        app.project.set_link16_mode(Link16Mode::Dmg07_16);
        app.apply_action(Action::ChordMix);
        app.apply_action(Action::ChordLane(5));
        assert_eq!(app.chords.lanes, 1 << 5);
        assert!(app.chords.mix);
        app.apply_action(Action::ChordBulk);
        assert_eq!(app.chords.lanes, 65535);
        app.apply_action(Action::ChordBulk);
        assert_eq!(app.chords.lanes, 0);
        assert!(app.chords.mix);
        assert_eq!(app.chord_buttons().len(), 18);
        app.draw_list();
    }
}
impl GbseqApp {
    pub(super) fn chord_buttons(&self) -> Vec<ButtonSpec> {
        let mut out = Vec::new();
        let w = (self.size.width - 24.).clamp(160., 570.);
        let x = 12.;
        let y = HEADER_H + 40.;
        let n = self.project.link16_mode.track_count();
        let all = (1u32 << n) - 1;
        for (j, (action, k, on)) in [
            (Action::ChordMix, "Combined chords", self.chords.mix),
            (
                Action::ChordBulk,
                "All lane labels",
                self.chords.lanes & all == all,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            out.push(ButtonSpec {
                action,
                label: format!("{}: {}", self.language.translate(k), on_off(on)),
                rect: Rect::new(x + w / 2. * j as f32, y, w / 2., 34.),
            });
        }
        for i in 0..n {
            out.push(ButtonSpec {
                action: Action::ChordLane(i),
                label: format!(
                    "P{} CH{}: {}",
                    i / 4 + 1,
                    i % 4 + 1,
                    on_off(self.chords.lanes & (1 << i) != 0)
                ),
                rect: Rect::new(
                    x + w / 4. * (i % 4) as f32,
                    y + 35. * (1 + i / 4) as f32,
                    w / 4.,
                    34.,
                ),
            });
        }
        out
    }
    pub(super) fn draw_chords(&self, list: &mut DrawList) {
        if !self.chords.mix && self.chords.lanes == 0 {
            return;
        }
        let view = self.timeline_viewport();
        let first = (self.scroll_x() / self.half_step_w / 4.).floor().max(0.) as u32;
        let last = (((self.scroll_x() + view.width) / self.half_step_w / 4.).ceil() as u32)
            .min(self.project.total_steps() / 4);
        let stride = (42. / (4. * self.half_step_w)).ceil().max(1.) as u32;
        let line = |list: &mut DrawList, rect: Rect, lane: Option<usize>| {
            list.push_clip(rect);
            let mut prev = None;
            let mut next_x = rect.x;
            for beat in (first..last).step_by(stride as usize) {
                let label = self.chords.at(&self.project, lane, beat);
                if label.is_some() && (prev != label || beat % 4 == 0) {
                    let name = label.as_deref().unwrap();
                    let x = self.step_x(beat * 4).max(rect.x);
                    if x >= next_x {
                        let w = 12. + name.len() as f32 * 7.;
                        let r = Rect::new(x, rect.y, w, rect.height.min(19.));
                        list.fill_rect(r, PANEL_RAISED);
                        self.text_in(list, r, name, 0.78, ACCENT);
                        next_x = x + w;
                    }
                }
                prev = label;
            }
            list.pop_clip();
        };
        if self.chords.mix {
            let r = Rect::new(view.x, view.y - 22., view.width, 22.);
            list.fill_rect(r, HEADER);
            self.text(list, 14., r.y + 4., "Chords", 0.9, ACCENT);
            line(list, r, None);
        }
        for i in self.visible_track_start()..self.visible_track_start() + self.visible_track_count()
        {
            if self.chords.lanes & (1 << i) == 0 || i % 4 == 3 {
                continue;
            }
            let r = if self.key_editor {
                view
            } else {
                self.lane_rect(i)
            };
            line(
                list,
                Rect::new(r.x, r.y, r.width, r.height.min(17.)),
                Some(i),
            );
        }
    }
}
