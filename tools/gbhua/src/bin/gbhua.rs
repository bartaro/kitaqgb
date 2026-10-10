use gbhua_lib::{
    image_io::{self, ImageMode, ImportOptions},
    Project,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

const HELP: &str = "\
gbhua - GBHUA headless CLI
  import-image INPUT.png --out PROJECT.gbh [--mode dmg|cgb] [--size 160x144]
  inspect PROJECT
  validate PROJECT
  preview PROJECT --out PREVIEW.png [--mode dmg|cgb] [--scale 1..8]
  export PROJECT --out OUTPUT [--format c|gbh|gbr|gbtb|gbm|gbmb|json|2bpp] [--prefix kq_asset]
Writes require --force to overwrite existing files.
Successful commands emit one compact JSON object. Errors emit JSON on stderr.
Exit codes: 0 success, 1 invalid project, 2 command/IO/conversion error.
";

type CliResult<T> = Result<T, String>;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args == ["--help"] || args == ["-h"] || args == ["help"] {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok((value, valid)) => {
            println!("{value}");
            ExitCode::from(if valid { 0 } else { 1 })
        }
        Err(error) => {
            eprintln!("{}", json!({"ok": false, "error": error}));
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> CliResult<(Value, bool)> {
    if args.len() < 2 {
        return Err("command requires an input path; use --help".into());
    }
    let command = args[0].as_str();
    let allowed: &[&str] = match command {
        "import-image" => &["out", "mode", "size", "force"],
        "inspect" | "validate" => &[],
        "preview" => &["out", "mode", "scale", "force"],
        "export" => &["out", "format", "prefix", "force"],
        _ => return Err(format!("unknown command {command}; use --help")),
    };
    let flags = parse_flags(&args[2..], allowed)?;
    let input = Path::new(&args[1]);
    let mode = || {
        flags
            .get("mode")
            .map(String::as_str)
            .unwrap_or("cgb")
            .parse::<ImageMode>()
            .map_err(|e| e.to_string())
    };
    let output = || {
        flags
            .get("out")
            .map(PathBuf::from)
            .ok_or_else(|| "--out is required".to_string())
    };
    let force = flags.contains_key("force");
    if command == "import-image" {
        let size = flags.get("size").map(|s| parse_size(s)).transpose()?;
        let out = output()?;
        if !matches!(
            out.extension().and_then(|s| s.to_str()),
            Some("gbh" | "json")
        ) {
            return Err("import output must end in .gbh or .json".into());
        }
        let (project, report) = image_io::import_png(
            input,
            &ImportOptions {
                mode: mode()?,
                resize: size,
            },
        )
        .map_err(|e| e.to_string())?;
        write_outputs(
            input,
            &[(
                out.clone(),
                project
                    .to_json_pretty()
                    .map_err(|e| e.to_string())?
                    .into_bytes(),
            )],
            force,
        )?;
        return Ok((json!({"ok":true,"output":out,"report":report}), true));
    }
    let project = image_io::load_checked(input).map_err(|e| e.to_string())?;
    let validation = image_io::validate(&project);
    if command == "validate" {
        return Ok((
            json!({"ok":validation.valid,"validation":validation}),
            validation.valid,
        ));
    }
    if command == "inspect" {
        return Ok((
            json!({"ok":validation.valid,"name":project.name,
            "map_size":[project.map_width,project.map_height],"budget":project.budget_report(),
            "validation":validation}),
            validation.valid,
        ));
    }
    image_io::require_valid(&project).map_err(|e| e.to_string())?;
    let out = output()?;
    let outputs = if command == "preview" {
        let scale = flags
            .get("scale")
            .map(|s| s.parse::<u32>().map_err(|_| "scale must be 1..8"))
            .transpose()?
            .unwrap_or(1);
        vec![(
            out.clone(),
            image_io::preview_png_bytes(&project, mode()?, scale).map_err(|e| e.to_string())?,
        )]
    } else {
        export_outputs(
            &project,
            &out,
            flags.get("format").map(String::as_str),
            flags
                .get("prefix")
                .map(String::as_str)
                .unwrap_or("kq_asset"),
        )?
    };
    write_outputs(input, &outputs, force)?;
    Ok((
        json!({"ok":true,"outputs":outputs.iter().map(|entry| &entry.0).collect::<Vec<_>>(),
        "warnings":validation.warnings}),
        true,
    ))
}

fn parse_flags(args: &[String], allowed: &[&str]) -> CliResult<BTreeMap<String, String>> {
    let mut flags = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let key = args[index].strip_prefix("--").ok_or("expected --option")?;
        if !allowed.contains(&key) {
            return Err(format!("unknown option --{key}"));
        }
        if flags.contains_key(key) {
            return Err(format!("duplicate option --{key}"));
        }
        let value = if key == "force" {
            String::new()
        } else {
            index += 1;
            args.get(index)
                .filter(|value| !value.starts_with("--"))
                .ok_or_else(|| format!("missing value for --{key}"))?
                .clone()
        };
        flags.insert(key.to_string(), value);
        index += 1;
    }
    Ok(flags)
}

fn parse_size(value: &str) -> CliResult<(u32, u32)> {
    let (width, height) = value.split_once('x').ok_or("size must be WIDTHxHEIGHT")?;
    Ok((
        width.parse().map_err(|_| "invalid width")?,
        height.parse().map_err(|_| "invalid height")?,
    ))
}

fn export_outputs(
    project: &Project,
    out: &Path,
    format: Option<&str>,
    prefix: &str,
) -> CliResult<Vec<(PathBuf, Vec<u8>)>> {
    let format = format.unwrap_or_else(|| out.extension().and_then(|s| s.to_str()).unwrap_or(""));
    let bytes = match format {
        "c" => project.export_kitaqgb_c(prefix).map(|s| s.into_bytes()),
        "gbr" | "gbtd" | "gbtb" => project.to_gbtd_bytes(),
        "gbm" | "gbmb" => {
            let tiles = out.with_extension("gbr");
            let name = tiles
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("invalid tile filename")?;
            return Ok(vec![
                (
                    tiles.clone(),
                    project.to_gbtd_bytes().map_err(|e| e.to_string())?,
                ),
                (
                    out.to_path_buf(),
                    project.to_gbmb_bytes(name).map_err(|e| e.to_string())?,
                ),
            ]);
        }
        "gbh" | "json" => project.to_json_pretty().map(|s| s.into_bytes()),
        "2bpp" => Ok(project.tile_bytes_2bpp()),
        _ => return Err("format must be c, gbh, gbr, gbtd, gbtb, gbm, gbmb, json or 2bpp".into()),
    }
    .map_err(|e| e.to_string())?;
    Ok(vec![(out.to_path_buf(), bytes)])
}

fn absolute_target(path: &Path) -> CliResult<PathBuf> {
    if path.exists() {
        return path.canonicalize().map_err(|e| e.to_string());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent
        .canonicalize()
        .map_err(|e| e.to_string())?
        .join(path.file_name().ok_or("output filename is required")?))
}

fn write_outputs(input: &Path, outputs: &[(PathBuf, Vec<u8>)], force: bool) -> CliResult<()> {
    let source = input.canonicalize().map_err(|e| e.to_string())?;
    let mut destinations = Vec::new();
    for (path, _) in outputs {
        let target = absolute_target(path)?;
        if target == source || destinations.contains(&target) {
            return Err("input and output paths, and output paths, must be distinct".into());
        }
        if path.exists() && !force {
            return Err(format!(
                "{} exists; use --force to overwrite",
                path.display()
            ));
        }
        destinations.push(target);
    }
    for (path, bytes) in outputs {
        let mut options = fs::OpenOptions::new();
        options.write(true);
        if force {
            options.create(true).truncate(true);
        } else {
            options.create_new(true);
        }
        let mut file = options
            .open(path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        file.write_all(bytes)
            .and_then(|_| file.flush())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_duplicate_and_missing_options() {
        for args in [vec!["--bad"], vec!["--out"], vec!["--force", "--force"]] {
            assert!(parse_flags(
                &args.into_iter().map(String::from).collect::<Vec<_>>(),
                &["out", "force"]
            )
            .is_err());
        }
        assert_eq!(parse_size("160x144").unwrap(), (160, 144));
        assert!(parse_size("160X144").is_err());
    }
}
