//! Stable ordering for ASCII identifiers at culture-sorted C# emission sites.
//! Underscores precede letters/digits; case is a secondary weight.
pub(super) fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    fn primary(s: &str) -> Vec<u32> {
        s.chars()
            .flat_map(char::to_lowercase)
            .map(|c| if c == '_' { 0 } else { c as u32 })
            .collect()
    }
    fn case(s: &str) -> Vec<bool> {
        s.chars().map(char::is_uppercase).collect()
    }
    primary(a)
        .cmp(&primary(b))
        .then_with(|| case(a).cmp(&case(b)))
        .then_with(|| a.encode_utf16().cmp(b.encode_utf16()))
}
