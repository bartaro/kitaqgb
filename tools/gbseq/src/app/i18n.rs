use super::*;
use std::{borrow::Cow, sync::OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Language {
    English,
    Japanese,
    Korean,
    ChineseSimplified,
    ChineseTraditional,
    Spanish,
    Portuguese,
    French,
    German,
}

pub(super) fn preference_path() -> Option<PathBuf> {
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
    pub(super) const ALL: [Self; 9] = [
        Self::English,
        Self::Japanese,
        Self::Korean,
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::Spanish,
        Self::Portuguese,
        Self::French,
        Self::German,
    ];
    pub(super) fn code(self) -> &'static str {
        [
            "en", "ja", "ko", "zh-CN", "zh-TW", "es", "pt-BR", "fr", "de",
        ][self as usize]
    }
    pub(super) fn name(self) -> &'static str {
        [
            "English",
            "日本語",
            "한국어",
            "简体中文",
            "繁體中文",
            "Español",
            "Português (Brasil)",
            "Français",
            "Deutsch",
        ][self as usize]
    }
    pub(super) fn parse(value: &str) -> Option<Self> {
        let value = value
            .trim()
            .split(['.', '@'])
            .next()?
            .to_ascii_lowercase()
            .replace('_', "-");
        if value == "zh" || value.starts_with("zh-") {
            return Some(
                if value
                    .split('-')
                    .any(|x| matches!(x, "tw" | "hk" | "mo" | "hant"))
                {
                    Self::ChineseTraditional
                } else {
                    Self::ChineseSimplified
                },
            );
        }
        Self::ALL
            .into_iter()
            .find(|l| l.code().split('-').next() == value.split('-').next())
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
                    if let Some(l) = Self::parse(&value) {
                        return l;
                    }
                }
            }
        }
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                pub(super) fn GetUserDefaultUILanguage() -> u16;
            }
            let n = unsafe { GetUserDefaultUILanguage() };
            if let Some(l) = match n & 0x3ff {
                4 => Some(if matches!(n >> 10, 1 | 3 | 5) {
                    Self::ChineseTraditional
                } else {
                    Self::ChineseSimplified
                }),
                7 => Some(Self::German),
                9 => Some(Self::English),
                10 => Some(Self::Spanish),
                12 => Some(Self::French),
                17 => Some(Self::Japanese),
                18 => Some(Self::Korean),
                22 => Some(Self::Portuguese),
                _ => None,
            } {
                return l;
            }
        }
        #[cfg(target_os = "macos")]
        if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            let value = String::from_utf8_lossy(&output.stdout);
            if output.status.success() {
                if let Some(l) = value
                    .split(|c: char| !(c.is_ascii_alphabetic() || c == '-'))
                    .filter(|s| !s.is_empty())
                    .find_map(Self::parse)
                {
                    return l;
                }
            }
        }
        Self::English
    }

    pub(super) fn translate<'a>(self, text: &'a str) -> Cow<'a, str> {
        if self == Self::English {
            return Cow::Borrowed(text);
        }
        type Catalog = (
            std::collections::HashMap<&'static str, &'static str>,
            Vec<(&'static str, &'static str)>,
        );
        static TABLES: OnceLock<Vec<Catalog>> = OnceLock::new();
        let (exact, patterns) = &TABLES.get_or_init(|| {
            (1..9)
                .map(|i| {
                    let map: std::collections::HashMap<_, _> = if i == 1 {
                        include_str!("locale_ja.tsv")
                            .lines()
                            .filter_map(|l| l.split_once('\t'))
                            .collect()
                    } else {
                        include_str!("locale_multi.tsv")
                            .lines()
                            .map(|l| {
                                let mut cols = l.split('\t');
                                (cols.next().unwrap(), cols.nth(i - 2).unwrap())
                            })
                            .collect()
                    };
                    let mut pat: Vec<_> = map
                        .iter()
                        .filter(|(k, _)| k.contains("{}"))
                        .map(|(&k, &v)| (k, v))
                        .collect();
                    pat.sort_by_key(|(k, _)| (std::cmp::Reverse(k.len()), *k));
                    (map, pat)
                })
                .collect()
        })[self as usize - 1];
        if let Some(&s) = exact.get(text) {
            return Cow::Borrowed(s);
        }
        // Translate fixed UI messages, retaining project names, file paths and diagnostic details.
        for (en, ja) in patterns {
            if let Some(value) = template(text, en, ja) {
                return Cow::Owned(value);
            }
        }
        if let Some((base, timing)) = text.split_once(" / render ") {
            return Cow::Owned(format!("{} / {}", self.translate(base), timing));
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

    pub(super) fn set_language(&mut self, l: Language) {
        self.language = l;
        self.status = "Display language changed".into();
        if !cfg!(test) {
            let result = preference_path()
                .ok_or_else(|| std::io::Error::other("No user configuration directory"))
                .and_then(|p| {
                    fs::create_dir_all(p.parent().unwrap())?;
                    fs::write(p, self.language.code())
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
        let mut multi_keys = BTreeSet::new();
        for row in include_str!("locale_multi.tsv").lines() {
            let cols = row.split('\t').collect::<Vec<_>>();
            assert_eq!(cols.len(), 8, "{row}");
            assert!(multi_keys.insert(cols[0]));
            assert!(
                cols[1..]
                    .iter()
                    .all(|s| !s.is_empty()
                        && s.matches("{}").count() == cols[0].matches("{}").count()),
                "{row}"
            );
        }
        assert_eq!(keys, multi_keys);
        for l in Language::ALL {
            assert_eq!(Language::parse(l.code()), Some(l));
            assert!(!l.name().is_empty());
        }
        assert_eq!(
            Language::parse("zh_Hant_HK.UTF-8"),
            Some(Language::ChineseTraditional)
        );
        assert_eq!(Language::parse("pt_BR.UTF-8"), Some(Language::Portuguese));
    }
}
