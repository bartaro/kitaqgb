use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("gbhua-cli-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_gbhua"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn renamed_json_project_works_without_a_legacy_extension_loader() {
    let fixture = Fixture::new();
    let project = gbhua_lib::Project::default();
    let old_path = fixture.0.join("old.kgbasset");
    project.save_json_file(&old_path).unwrap();
    assert_eq!(
        fixture.run(&["inspect", "old.kgbasset"]).status.code(),
        Some(2)
    );
    let bytes = fs::read(&old_path).unwrap();
    fs::rename(&old_path, fixture.0.join("renamed.gbh")).unwrap();
    assert_eq!(fixture.ok(&["validate", "renamed.gbh"])["ok"], true);
    assert_eq!(fs::read(fixture.0.join("renamed.gbh")).unwrap(), bytes);
    fixture.ok(&["export", "renamed.gbh", "--out", "exchange.gbtb"]);
    fixture.ok(&["export", "renamed.gbh", "--out", "exchange.gbmb"]);
    assert_eq!(fixture.ok(&["validate", "exchange.gbtb"])["ok"], true);
    assert_eq!(fixture.ok(&["validate", "exchange.gbmb"])["ok"], true);
}

#[test]
fn image_project_preview_and_legacy_formats_round_trip() {
    let fixture = Fixture::new();
    image::RgbaImage::from_fn(16, 16, |x, y| {
        let value = [255, 170, 85, 0][((x + y) % 4) as usize];
        image::Rgba([value, value, value, 255])
    })
    .save(fixture.0.join("input.png"))
    .unwrap();
    let report = fixture.ok(&[
        "import-image",
        "input.png",
        "--out",
        "scene.gbh",
        "--mode",
        "dmg",
    ]);
    assert_eq!(report["report"]["tile_count"], 1);
    assert_eq!(
        fixture.ok(&["validate", "scene.gbh"])["validation"]["valid"],
        true
    );
    assert_eq!(
        fixture.ok(&["inspect", "scene.gbh"])["map_size"],
        serde_json::json!([2, 2])
    );
    fixture.ok(&[
        "preview",
        "scene.gbh",
        "--out",
        "preview.png",
        "--scale",
        "2",
    ]);
    assert_eq!(
        image::open(fixture.0.join("preview.png")).unwrap().width(),
        32
    );
    fixture.ok(&[
        "export",
        "scene.gbh",
        "--out",
        "scene.c",
        "--prefix",
        "scene",
    ]);
    assert!(fs::read_to_string(fixture.0.join("scene.c"))
        .unwrap()
        .contains("scene_load"));
    fixture.ok(&["export", "scene.gbh", "--out", "scene.2bpp"]);
    assert_eq!(
        fs::metadata(fixture.0.join("scene.2bpp")).unwrap().len(),
        16
    );
    fixture.ok(&["export", "scene.gbh", "--out", "legacy.gbm"]);
    assert!(fixture.0.join("legacy.gbr").exists());
    assert_eq!(fixture.ok(&["validate", "legacy.gbm"])["ok"], true);
    fixture.ok(&["export", "legacy.gbm", "--out", "roundtrip.json"]);
    let original: gbhua_lib::Project =
        serde_json::from_str(&fs::read_to_string(fixture.0.join("scene.gbh")).unwrap()).unwrap();
    let restored = gbhua_lib::Project::load_project_file(fixture.0.join("roundtrip.json")).unwrap();
    assert_eq!(original.tile_bytes_2bpp(), restored.tile_bytes_2bpp());
    assert_eq!(
        original.map_tile_bytes().unwrap(),
        restored.map_tile_bytes().unwrap()
    );
    let refusal = fixture.run(&["import-image", "input.png", "--out", "scene.gbh"]);
    assert_eq!(refusal.status.code(), Some(2));
    assert!(
        serde_json::from_slice::<Value>(&refusal.stderr).unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("--force")
    );
    fixture.ok(&[
        "import-image",
        "input.png",
        "--out",
        "scene.gbh",
        "--force",
        "--mode",
        "cgb",
    ]);
}

#[test]
fn malformed_projects_and_aliasing_outputs_are_rejected() {
    let fixture = Fixture::new();
    let mut project = gbhua_lib::Project::default();
    project.map[0].tile = 500;
    fs::write(
        fixture.0.join("invalid.json"),
        serde_json::to_vec(&project).unwrap(),
    )
    .unwrap();
    let invalid = fixture.run(&["validate", "invalid.json"]);
    assert_eq!(invalid.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&invalid.stdout).unwrap()["ok"],
        false
    );
    let preview = fixture.run(&["preview", "invalid.json", "--out", "bad.png"]);
    assert_eq!(preview.status.code(), Some(2));
    assert!(!fixture.0.join("bad.png").exists());
    project.map[0].tile = 0;
    project
        .save_json_file(fixture.0.join("valid.json"))
        .unwrap();
    let before = fs::read(fixture.0.join("valid.json")).unwrap();
    let alias = fixture.run(&["export", "valid.json", "--out", "valid.json", "--force"]);
    assert_eq!(alias.status.code(), Some(2));
    assert_eq!(fs::read(fixture.0.join("valid.json")).unwrap(), before);
    let repeated = fixture.run(&["inspect", "valid.json", "--mode", "cgb"]);
    assert_eq!(repeated.status.code(), Some(2));
}
