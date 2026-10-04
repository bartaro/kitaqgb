//! Content-addressed native build cache. Corrupt or incomplete entries are ignored.
use crate::{hash, io, json::Value, tokenizer};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
const EXTENSIONS: [&str; 8] = [
    "gb",
    "map",
    "dbg",
    "dbc",
    "banks.txt",
    "funcsizes.txt",
    "source_map.txt",
    "dbg2.json",
];
pub struct Entry {
    directory: PathBuf,
}
impl Entry {
    pub fn for_invocation(
        args: &[String],
        inputs: &[PathBuf],
        includes: &[PathBuf],
        header: &crate::rom_header::Options,
    ) -> Option<Self> {
        let scan = tokenizer::tokenize_files(inputs, includes);
        if scan.has_errors() {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let mut data = format!(
            "kitaqgb-rust-cache-v1\n{}\n{:?}\n{header:?}\n",
            hash::file(exe),
            scan.header
        );
        let mut i = 0;
        while i < args.len() {
            if args[i] == "-o" {
                i += 2;
                continue;
            }
            if matches!(args[i].as_str(), "--cache" | "--no-cache" | "--no-banner") {
                i += 1;
                continue;
            }
            data.push_str(&args[i]);
            data.push('\n');
            i += 1;
        }
        for file in scan.dependencies {
            data.push_str(&file.display().to_string());
            data.push('|');
            data.push_str(&hash::file(file));
            data.push('\n');
        }
        Some(Self {
            directory: Path::new(".kitaqgb_cache/rust-v1").join(hash::sha256(data.as_bytes())),
        })
    }
    pub fn key(&self) -> &str {
        self.directory.file_name().unwrap().to_str().unwrap()
    }
    pub fn restore(&self, output: &Path) -> Result<bool, String> {
        let Ok(text) = io::read_utf8(self.directory.join("manifest.json")) else {
            return Ok(false);
        };
        let Ok(Value::Object(manifest)) = Value::parse(&text) else {
            return Ok(false);
        };
        let mut files = BTreeMap::new();
        for suffix in EXTENSIONS.into_iter().chain(["build_report.json"]) {
            let path = self.directory.join(format!("out.{suffix}"));
            let Some(Value::String(expected)) = manifest.get(suffix) else {
                return Ok(false);
            };
            let Ok(bytes) = std::fs::read(path) else {
                return Ok(false);
            };
            if hash::sha256(&bytes) != *expected {
                return Ok(false);
            }
            files.insert(suffix, bytes);
        }
        let old = if let Some(Value::String(path)) = manifest.get("output_path") {
            path.as_str()
        } else {
            return Ok(false);
        };
        for (suffix, bytes) in files {
            let destination = output.with_extension(suffix);
            if matches!(suffix, "dbg2.json" | "build_report.json") {
                let mut value =
                    Value::parse(&String::from_utf8(bytes).map_err(|e| e.to_string())?)?;
                rewrite_path(&mut value, old, &output.display().to_string());
                io::write_utf8(destination, &value.stringify())?;
            } else if suffix == "dbg" {
                // The debugger file contains the sibling .dbc basename.
                let old_dbc = Path::new(old)
                    .with_extension("dbc")
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let new_dbc = output
                    .with_extension("dbc")
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                io::write_utf8(
                    destination,
                    &String::from_utf8(bytes)
                        .map_err(|e| e.to_string())?
                        .replace(&old_dbc, &new_dbc),
                )?;
            } else {
                io::write_bytes(destination, &bytes)?;
            }
        }
        Ok(true)
    }
    pub fn save(&self, output: &Path) -> Result<(), String> {
        let mut manifest = BTreeMap::new();
        for suffix in EXTENSIONS.into_iter().chain(["build_report.json"]) {
            let bytes = std::fs::read(output.with_extension(suffix)).map_err(|e| e.to_string())?;
            io::write_bytes(self.directory.join(format!("out.{suffix}")), &bytes)?;
            manifest.insert(suffix.into(), Value::String(hash::sha256(&bytes)));
        }
        manifest.insert(
            "output_path".into(),
            Value::String(output.display().to_string()),
        );
        io::write_utf8(
            self.directory.join("manifest.json"),
            &Value::Object(manifest).stringify(),
        )
    }
}
fn rewrite_path(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(s) if s == old => *s = new.into(),
        Value::Array(values) => {
            for value in values {
                rewrite_path(value, old, new)
            }
        }
        Value::Object(fields) => {
            for value in fields.values_mut() {
                rewrite_path(value, old, new)
            }
        }
        _ => {}
    }
}
