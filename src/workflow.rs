//! Portable workflow paths, timestamps and command history.
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    for path in cwd.ancestors() {
        if path.join("kitaqgb.sln").is_file() {
            return path.to_owned();
        }
        if let Ok(text) = std::fs::read_to_string(path.join("Cargo.toml")) {
            if text.contains("name = \"kitaqgb\"") {
                return path.to_owned();
            }
        }
    }
    cwd
}
pub fn utc_now() -> String {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    timestamp(time.as_secs(), time.subsec_nanos() / 100)
}
pub fn timestamp(seconds: u64, fraction: u32) -> String {
    timestamp_signed(seconds as i64, fraction)
}
pub fn timestamp_signed(seconds: i64, fraction: u32) -> String {
    let z = seconds.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let rem = seconds.rem_euclid(86400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{fraction:07}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}
pub fn quote_arg(arg: &str) -> String {
    let mut text = String::from("\"");
    let mut slashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        text.push_str(&"\\".repeat(if c == '"' { slashes * 2 + 1 } else { slashes }));
        text.push(c);
        slashes = 0;
    }
    text.push_str(&"\\".repeat(slashes * 2));
    text.push('"');
    text
}
pub fn history_path() -> PathBuf {
    repo_root().join("integration_test/reports/COMMAND_HISTORY.log")
}
pub fn append_history(args: &[String]) {
    fn append(args: &[String]) -> std::io::Result<()> {
        let path = history_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let clean = |s: String| s.replace(['\r', '\n', '\t'], " ");
        let cwd = std::env::current_dir()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(
            file,
            "{}\t{}\t{}",
            utc_now(),
            clean(cwd.display().to_string()),
            clean(
                args.iter()
                    .map(|s| quote_arg(s))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        )
    }
    let _ = append(args);
}
pub(crate) fn full(path: impl AsRef<Path>) -> PathBuf {
    let p = path.as_ref();
    if p.is_absolute() {
        p.to_owned()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn calendar() {
        assert_eq!(super::timestamp(0, 0), "1970-01-01T00:00:00.0000000Z");
        assert_eq!(
            super::timestamp(951782400, 1234567),
            "2000-02-29T00:00:00.1234567Z"
        );
    }
}
