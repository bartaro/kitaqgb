use std::fs;
use std::path::Path;
use std::path::PathBuf;
/// Match File.ReadAllText(path, Encoding.UTF8), including Unicode BOM detection
/// and replacement of malformed input.
pub fn read_utf8(path: impl AsRef<Path>) -> Result<String, String> {
    let bytes = fs::read(path.as_ref()).map_err(|e| format!("{}: {e}", path.as_ref().display()))?;
    Ok(decode_source(&bytes))
}
pub fn write_utf8(path: impl AsRef<Path>, text: &str) -> Result<(), String> {
    write_bytes(path, text.as_bytes())
}
pub fn write_bytes(path: impl AsRef<Path>, bytes: &[u8]) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut error = None;
    for attempt in 0..6 {
        match fs::write(path, bytes) {
            Ok(()) => return Ok(()),
            Err(e) => error = Some(e),
        }
        std::thread::sleep(std::time::Duration::from_millis(70 * (attempt + 1)));
    }
    Err(format!("{}: {}", path.display(), error.unwrap()))
}
/// Return the actual artifact destination after retrying a transient write failure.
pub fn write_bytes_robust(
    path: impl AsRef<Path>,
    bytes: &[u8],
    alternate: bool,
) -> Result<PathBuf, String> {
    let path = path.as_ref();
    match write_bytes(path, bytes) {
        Ok(()) => Ok(path.to_owned()),
        Err(error)
            if alternate
                && path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .is_none_or(|p| p.is_dir()) =>
        {
            let utc = crate::workflow::utc_now();
            let stamp = format!(
                "{}{}",
                utc[..19].replace(['-', ':'], "").replace('T', "_"),
                &utc[20..23]
            );
            let name = format!(
                "{}.lockretry_{}{}",
                path.file_stem().unwrap_or_default().to_string_lossy(),
                stamp,
                path.extension()
                    .map_or(String::new(), |s| format!(".{}", s.to_string_lossy()))
            );
            let actual = path.with_file_name(name);
            write_bytes(&actual, bytes)
                .map_err(|second| format!("{error}; alternate write failed: {second}"))?;
            Ok(actual)
        }
        Err(error) => Err(error),
    }
}
pub fn write_utf8_robust(
    path: impl AsRef<Path>,
    text: &str,
    alternate: bool,
) -> Result<PathBuf, String> {
    write_bytes_robust(path, text.as_bytes(), alternate)
}
/// Publish a CLI helper's text and expose an alternate destination to its caller.
pub(crate) fn publish_utf8(path: impl AsRef<Path>, text: &str) -> Result<PathBuf, String> {
    let path = path.as_ref();
    let actual = write_utf8_robust(path, text, true)?;
    if actual != path {
        eprintln!(
            "warning KQ0000: output is locked, wrote alternate file: {}",
            actual.display()
        );
    }
    Ok(actual)
}
fn decode_source(bytes: &[u8]) -> String {
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe, 0, 0]) {
        return decode_utf32(bytes, true);
    }
    if let Some(bytes) = bytes.strip_prefix(&[0, 0, 0xfe, 0xff]) {
        return decode_utf32(bytes, false);
    }
    for (bom, little) in [([0xff, 0xfe], true), ([0xfe, 0xff], false)] {
        if let Some(bytes) = bytes.strip_prefix(&bom) {
            let mut text = String::from_utf16_lossy(
                &bytes
                    .chunks_exact(2)
                    .map(|b| {
                        if little {
                            u16::from_le_bytes([b[0], b[1]])
                        } else {
                            u16::from_be_bytes([b[0], b[1]])
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            if bytes.len() % 2 != 0 {
                text.push('\u{fffd}');
            }
            return text;
        }
    }
    String::from_utf8_lossy(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)).into_owned()
}
fn decode_utf32(bytes: &[u8], little: bool) -> String {
    let mut text = String::new();
    for b in bytes.chunks_exact(4) {
        let b = [b[0], b[1], b[2], b[3]];
        let code = if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        };
        text.push(char::from_u32(code).unwrap_or('\u{fffd}'));
    }
    if bytes.len() % 4 != 0 {
        text.push('\u{fffd}');
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_source_boms_and_invalid_sequences() {
        let expected = "u8 日本😀;\n";
        let mut utf8 = vec![0xef, 0xbb, 0xbf];
        utf8.extend(expected.as_bytes());
        assert_eq!(decode_source(&utf8), expected);
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for u in expected.encode_utf16() {
                bytes.extend(if little {
                    u.to_le_bytes()
                } else {
                    u.to_be_bytes()
                });
            }
            assert_eq!(decode_source(&bytes), expected);
            bytes = if little {
                vec![0xff, 0xfe, 0, 0]
            } else {
                vec![0, 0, 0xfe, 0xff]
            };
            for c in expected.chars() {
                bytes.extend(if little {
                    (c as u32).to_le_bytes()
                } else {
                    (c as u32).to_be_bytes()
                });
            }
            assert_eq!(decode_source(&bytes), expected);
        }
        assert_eq!(decode_source(&[0xff, 0xfe, 0, 0xd8, 1]), "\u{fffd}\u{fffd}");
        assert_eq!(decode_source(&[0xff]), "\u{fffd}");
        assert_eq!(
            decode_source(&[0, 0, 0xfe, 0xff, 0, 0, 0xd8, 0]),
            "\u{fffd}"
        );
    }
}
