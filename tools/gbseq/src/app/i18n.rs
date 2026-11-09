use super::*;
use std::{borrow::Cow, sync::OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Language {
    English,
    Japanese,
}

fn preference_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    base.map(|p| p.join("gbseq/language.txt"))
}

impl Language {
    pub(super) fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        if value == "ja" || value.starts_with("ja-") || value.starts_with("ja_") {
            Some(Self::Japanese)
        } else if value == "en" || value.starts_with("en-") || value.starts_with("en_") {
            Some(Self::English)
        } else {
            None
        }
    }

    pub(super) fn detect() -> Self {
        if cfg!(test) {
            return Self::English;
        }
        if let Some(language) = std::env::var("GBSEQ_LANGUAGE")
            .ok()
            .as_deref()
            .and_then(Self::parse)
        {
            return language;
        }
        if let Some(language) = preference_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .as_deref()
            .and_then(Self::parse)
        {
            return language;
        }
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                if !value.is_empty() {
                    return Self::parse(&value).unwrap_or(Self::English);
                }
            }
        }
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                pub(super) fn GetUserDefaultUILanguage() -> u16;
            }
            if unsafe { GetUserDefaultUILanguage() } & 0x3ff == 0x11 {
                return Self::Japanese;
            }
        }
        #[cfg(target_os = "macos")]
        if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            let value = String::from_utf8_lossy(&output.stdout);
            if output.status.success()
                && value
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .find(|s| !s.is_empty())
                    == Some("ja")
            {
                return Self::Japanese;
            }
        }
        Self::English
    }

    pub(super) fn translate<'a>(self, text: &'a str) -> Cow<'a, str> {
        if self == Self::English {
            return Cow::Borrowed(text);
        }
        static CATALOG: OnceLock<Vec<(&str, &str)>> = OnceLock::new();
        let catalog = CATALOG.get_or_init(|| {
            include_str!("locale_ja.tsv")
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .collect()
        });
        if let Some((_, ja)) = catalog.iter().find(|(en, _)| *en == text) {
            return Cow::Borrowed(ja);
        }
        // Translate fixed UI messages, retaining project names, file paths and diagnostic details.
        for (en, ja) in catalog.iter().filter(|(en, _)| en.contains("{}")) {
            if let Some(value) = template(text, en, ja) {
                return Cow::Owned(value);
            }
        }
        if let Some((base, timing)) = text.split_once(" / render ") {
            return Cow::Owned(format!("{} / 生成 {}", self.translate(base), timing));
        }
        Cow::Borrowed(text)
    }
}

fn template(text: &str, pattern: &str, translated: &str) -> Option<String> {
    let mut parts = pattern.split("{}");
    let mut tail = text.strip_prefix(parts.next()?)?;
    let separators: Vec<_> = parts.collect();
    let mut values = Vec::new();
    for (i, separator) in separators.iter().enumerate() {
        if i == separators.len() - 1 {
            values.push(tail.strip_suffix(separator)?);
        } else {
            let at = tail.find(separator)?;
            values.push(&tail[..at]);
            tail = &tail[at + separator.len()..];
        }
    }
    let mut output = translated.split("{}");
    let mut value = output.next()?.to_string();
    for part in values {
        value.push_str(part);
        value.push_str(output.next()?);
    }
    if output.next().is_some() {
        return None;
    }
    Some(value)
}

impl GbseqApp {
    pub(super) fn text(
        &self,
        list: &mut DrawList,
        x: f32,
        y: f32,
        text: &str,
        scale: f32,
        color: Color,
    ) {
        self.text_in(
            list,
            Rect::new(x, y, (self.size.width - x - 8.0).max(1.0), 32.0),
            text,
            scale,
            color,
        );
    }

    pub(super) fn text_in(
        &self,
        list: &mut DrawList,
        rect: Rect,
        text: &str,
        scale: f32,
        color: Color,
    ) {
        // Icon-only buttons deliberately have no text area. plita skips empty
        // clips, so they must not emit an unmatched PopClip.
        if text.is_empty() || rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let translated = self.language.translate(text);
        let scale = if translated.is_ascii() {
            scale
        } else {
            scale.max(1.25)
        };
        let scale = if measure_text_width(&translated, scale) > rect.width {
            1.0
        } else {
            scale
        };
        let mut label = translated.into_owned();
        if measure_text_width(&label, scale) > rect.width {
            while !label.is_empty() && measure_text_width(&format!("{label}…"), scale) > rect.width
            {
                label.pop();
            }
            label.push('…');
        }
        list.push_clip(rect);
        draw_text(list, rect.x, rect.y, &label, scale, color);
        list.pop_clip();
    }

    pub(super) fn toggle_language(&mut self) {
        self.language = if self.language == Language::English {
            Language::Japanese
        } else {
            Language::English
        };
        self.status = "Display language changed".into();
        if !cfg!(test) {
            let result = preference_path()
                .ok_or_else(|| std::io::Error::other("No user configuration directory"))
                .and_then(|p| {
                    fs::create_dir_all(p.parent().unwrap())?;
                    fs::write(
                        p,
                        if self.language == Language::Japanese {
                            "ja\n"
                        } else {
                            "en\n"
                        },
                    )
                });
            if result.is_err() {
                self.status =
                    "Language changed for this session; preference could not be saved".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_and_dynamic_messages_keep_user_text_intact() {
        let mut keys = BTreeSet::new();
        for line in include_str!("locale_ja.tsv").lines() {
            let (en, ja) = line.split_once('\t').expect("TSV pair");
            assert!(keys.insert(en), "duplicate {en}");
            assert_eq!(en.matches("{}").count(), ja.matches("{}").count(), "{en}");
        }
        assert_eq!(Language::Japanese.translate("FILE"), "ファイル");
        assert_eq!(
            Language::Japanese.translate("POINT 31  VALUE 15"),
            "位置 31  値 15"
        );
        assert_eq!(
            Language::Japanese.translate("OPENED C:\\曲\\新曲.gbseq"),
            "読み込み完了 C:\\曲\\新曲.gbseq"
        );
        assert_eq!(Language::English.translate("FILE"), "FILE");
        assert_eq!(Language::parse("ja_JP.UTF-8"), Some(Language::Japanese));
    }
}
