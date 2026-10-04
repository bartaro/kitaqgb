//! Failure snapshots; all copied destinations are relative to the package.
use crate::{drivers, hash, io, json::Value, observation::Session, workflow};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, PathBuf},
};
pub fn on_failure(args: &[String], session: &mut Session, exit: i32) -> Result<(), String> {
    let mut path = None;
    for a in args {
        if a == "--repro-pack" {
            path = Some(String::new());
        } else if let Some(p) = a.strip_prefix("--repro-pack=") {
            path = Some(p.into());
        }
    }
    let Some(mut path) = path else {
        return Ok(());
    };
    if path.trim().is_empty() {
        let time = workflow::utc_now().replace(['-', ':', '.'], "");
        path = format!(
            "kitaqgb_fail_repro_{}",
            time.replace('T', "_").trim_end_matches('Z')
        );
    }
    let root = workflow::full(path);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    io::write_utf8(
        root.join("diagnostics.json"),
        &session.diagnostic_json(exit).stringify(),
    )?;
    io::write_utf8(root.join("args.txt"), &(args.join("\n") + "\n"))?;
    io::write_utf8(
        root.join("command.txt"),
        &format!(
            "kitaqgb {}",
            args.iter()
                .map(|a| workflow::quote_arg(a))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    )?;
    let mut files = drivers::sources(args)
        .into_iter()
        .map(workflow::full)
        .collect::<BTreeSet<_>>();
    files.extend(session.dependencies.iter().map(workflow::full));
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let input = root.join("inputs");
    std::fs::create_dir_all(&input).map_err(|e| e.to_string())?;
    let mut manifest = BTreeMap::new();
    let mut used = BTreeSet::new();
    for file in files.into_iter().filter(|p| p.is_file()) {
        let mut rel = PathBuf::new();
        for part in file.strip_prefix(&cwd).unwrap_or(&file).components() {
            if let Component::Normal(part) = part {
                rel.push(part);
            }
        }
        if rel.as_os_str().is_empty() {
            continue;
        }
        let key = rel.display().to_string().to_lowercase();
        if !used.insert(key) {
            let digest = hash::sha256(file.display().to_string().as_bytes());
            rel = PathBuf::from(&digest[..12]).join(rel);
        }
        let dest = input.join(&rel);
        // Components above exclude prefixes, root and parent traversal.
        if !dest.starts_with(&input) {
            return Err("repro input destination escaped package".into());
        }
        match std::fs::read(&file)
            .map_err(|e| e.to_string())
            .and_then(|b| io::write_bytes(&dest, &b))
        {
            Ok(()) => {
                manifest.insert(
                    file.display().to_string(),
                    Value::String(rel.display().to_string()),
                );
            }
            Err(error) => session.warning(&format!("repro input copy skipped: {error}")),
        }
    }
    io::write_utf8(
        root.join("inputs.json"),
        &Value::Object(manifest.clone()).stringify(),
    )?;
    let utc = workflow::utc_now();
    io::write_utf8(
        root.join("README.txt"),
        &format!(
            "# KITAQGB build failure repro package\nutc={}Z\nexit_code={exit}\ncopied_input_files={}\n\nFiles:\n- diagnostics.json : machine-readable diagnostics\n- args.txt / command.txt : invocation\n- inputs/ : source and dependency snapshot\n- inputs.json : original path to snapshot mapping\n\nRecorded command paths are unchanged; use inputs.json when relocating the package.\n",
            &utc[..19],
            manifest.len()
        ),
    )?;
    session.remember("repro_package", &root);
    if !session.hosted {
        eprintln!("[repro-pack] {}", root.display());
    }
    Ok(())
}
