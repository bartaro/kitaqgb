"""Build a portable GUI archive with native notices, fonts and all nine manuals."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

LANGUAGES = ("en", "ja", "ko", "zh-CN", "zh-TW", "es", "pt", "fr", "de")
TARGETS = {
    "x86_64-pc-windows-msvc": ("Windows", "AMD64", "windows-x86_64"),
    "aarch64-apple-darwin": ("Darwin", "arm64", "macos-arm64"),
    "x86_64-apple-darwin": ("Darwin", "x86_64", "macos-x86_64"),
}


def command(*args, cwd=None):
    return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def license_files(source):
    return [p for p in source.rglob("*") if p.is_file() and
            p.suffix.lower() in ("", ".md", ".txt", ".rst", ".html", ".apache", ".mit") and
            p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE", "COPYRIGHT"))]


def zip_bundle(bundle, archive):
    # Registry archives may use the Unix epoch, before ZIP's 1980 lower bound.
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, strict_timestamps=False) as stream:
        for path in sorted(bundle.rglob("*")):
            if path.is_file():
                stream.write(path, path.relative_to(bundle.parent).as_posix())


def collect_notices(root, bundle, target):
    metadata = json.loads(command("cargo", "metadata", "--locked", "--format-version", "1",
                                  "--features", "gui", "--filter-platform", target, cwd=root))
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    pending = [metadata["resolve"]["root"]]
    seen = set()
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        pending.extend(d["pkg"] for d in nodes[package_id]["deps"]
                       if any(k["kind"] != "dev" for k in d["dep_kinds"]))
    records = []
    missing = []
    for package_id in sorted(seen):
        package = packages[package_id]
        if package["source"] is None:
            continue
        source = Path(package["manifest_path"]).parent
        notices = license_files(source)
        if package.get("license_file"):
            notices.append(source / package["license_file"])
        if not notices:
            source = root / "licenses/overrides" / f"{package['name']}-{package['version']}"
            if source.is_dir():
                notices = [p for p in source.rglob("*") if p.is_file()]
        if not notices or not package.get("license"):
            missing.append(f"{package['name']}-{package['version']} ({package.get('license')})")
            continue
        copied = []
        for path in sorted(set(notices)):
            relative = Path("licenses/dependencies") / f"{package['name']}-{package['version']}" / path.relative_to(source)
            destination = bundle / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
            copied.append(relative.as_posix())
        records.append({"name": package["name"], "version": package["version"], "authors": package["authors"],
                        "license": package["license"], "notices": copied})
    if missing:
        raise RuntimeError("Manual license audit required: " + ", ".join(missing))
    (bundle / "GUI-DEPENDENCIES.json").write_text(json.dumps({"target": target,
        "scope": "Resolved GUI normal/build closure; original license texts retained. Not legal advice.",
        "packages": records}, indent=2) + "\n", encoding="utf-8")
    return len(records)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    system, arch, label = TARGETS[args.target]
    if (platform.system(), platform.machine()) != (system, arch):
        parser.error("Package on the native target OS/architecture")
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_commit):
        parser.error("A full source commit hash is required")
    root = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve(strict=True)
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    name = f"gbhua-gui-{label}"
    archive = out / (name + (".zip" if system == "Windows" else ".tar.gz"))
    receipt_path = out / f"{name}.json"
    if archive.exists() or receipt_path.exists():
        raise FileExistsError("Refusing to replace an existing distribution")
    with tempfile.TemporaryDirectory(prefix="gbhua-gui-") as temp:
        temp = Path(temp)
        bundle = temp / name
        bundle.mkdir()
        if system == "Darwin":
            if command("lipo", "-archs", str(binary)) != arch:
                raise RuntimeError("Unexpected Mach-O architecture")
            if os.environ.get("MACOSX_DEPLOYMENT_TARGET") != "11.0":
                raise RuntimeError("Expected macOS deployment target 11.0")
            contents = bundle / "GBHUA.app/Contents"
            executable = contents / "MacOS/GBHUA"
            assets = contents / "Resources/assets"
            contents.mkdir(parents=True)
            with (contents / "Info.plist").open("wb") as stream:
                plistlib.dump({"CFBundleName": "GBHUA", "CFBundleDisplayName": "GBHUA",
                    "CFBundleIdentifier": "com.github.bartaro.gbhua", "CFBundleExecutable": "GBHUA",
                    "CFBundlePackageType": "APPL", "CFBundleShortVersionString": "0.0.1",
                    "CFBundleVersion": "1", "LSMinimumSystemVersion": "11.0",
                    "NSHighResolutionCapable": True,
                    "NSHumanReadableCopyright": "Copyright (c) 2026 DAISUKE OBA"}, stream)
        else:
            executable = bundle / "GBHUA.exe"
            assets = bundle / "assets"
            data = binary.read_bytes()
            pe = int.from_bytes(data[60:64], "little")
            if int.from_bytes(data[pe + 92:pe + 94], "little") != 2:
                raise RuntimeError("Windows GUI must not open a console")
        executable.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        shutil.copytree(root / "vendor/plita/assets", assets)
        shutil.copytree(root / "docs", bundle / "docs")
        for language in LANGUAGES:
            if not (bundle / f"docs/manual.{language}.md").is_file():
                raise RuntimeError(f"Missing GUI manual: {language}")
        for filename in ("README.md", "GUI.md", "LICENSE", "THIRD_PARTY_NOTICES.md"):
            shutil.copy2(root / filename, bundle / filename)
        # Include CLI guides as referenced by GUI manuals, without bundling a second executable.
        for manual in root.glob("README.*.md"):
            shutil.copy2(manual, bundle / manual.name)
        shutil.copy2(root / "MACOS.md", bundle / "MACOS.md")
        shutil.copytree(root / "vendor/plita/licenses", bundle / "licenses/plita")
        shutil.copy2(root / "vendor/plita/LICENSE", bundle / "licenses/plita/LICENSE")
        shutil.copytree(root / "licenses/lucide", bundle / "licenses/lucide")
        shutil.copy2(root / "licenses/overrides/SOURCES.json", bundle / "licenses/override-sources.json")
        shutil.copy2(root / "licenses/overrides/README.md", bundle / "licenses/override-notes.md")
        count = collect_notices(root, bundle, args.target)
        rust_docs = Path(command("rustc", "--print", "sysroot")) / "share/doc/rust"
        shutil.copytree(rust_docs / "licenses", bundle / "licenses/rust/licenses")
        shutil.copy2(rust_docs / "COPYRIGHT-library.html", bundle / "licenses/rust/COPYRIGHT-library.html")
        if system == "Darwin":
            command("codesign", "--force", "--deep", "--sign", "-", str(bundle / "GBHUA.app"))
            command("codesign", "--verify", "--deep", "--strict", str(bundle / "GBHUA.app"))
        receipt = {"source_commit": args.source_commit, "target": args.target,
            "rustc": command("rustc", "--version"), "os": platform.platform(),
            "binary_sha256": sha256(executable), "manuals": list(LANGUAGES),
            "dependency_count": count, "developer_id_signed": False, "notarized": False,
            "gui_interactive_tested": False,
            "files": {p.relative_to(bundle).as_posix(): sha256(p)
                      for p in sorted(bundle.rglob("*")) if p.is_file()}}
        if system == "Windows":
            zip_bundle(bundle, archive)
        else:
            with tarfile.open(archive, "w:gz") as stream:
                stream.add(bundle, arcname=name)
        receipt["archive_sha256"] = sha256(archive)
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"archive": str(archive), "sha256": receipt["archive_sha256"],
                          "dependencies": count, "manuals": len(LANGUAGES)}))


if __name__ == "__main__":
    main()
