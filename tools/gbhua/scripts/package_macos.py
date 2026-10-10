"""Package a native, tested macOS CLI with its manuals and license notices."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
import zlib


TARGETS = {
    "aarch64-apple-darwin": ("arm64", "macos-arm64"),
    "x86_64-apple-darwin": ("x86_64", "macos-x86_64"),
}


def command(*args, cwd=None):
    result = subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True)
    return result.stdout.strip()


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def smoke(binary, directory):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    # An 8x8 four-shade fixture; no image library is needed by this build helper.
    pixels = b"".join(b"\0" + bytes([255, 170, 85, 0] * 2) for _ in range(8))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 8, 8, 8, 0, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b"")
    (directory / "input.png").write_bytes(png)
    checks = []
    if "import-image" not in command(str(binary), "--help"):
        raise RuntimeError("CLI help is missing")
    for mode in ("dmg", "cgb"):
        project = f"{mode}.gbh"
        commands = [
            ["import-image", "input.png", "--out", project, "--mode", mode],
            ["inspect", project],
            ["validate", project],
            ["preview", project, "--out", f"{mode}.png", "--mode", mode],
        ]
        for extension in ("c", "2bpp", "gbtb", "gbmb"):
            commands.append(["export", project, "--out", f"{mode}.{extension}"])
        commands.extend([
            ["validate", f"{mode}.gbtb"],
            ["validate", f"{mode}.gbmb"],
        ])
        for args in commands:
            report = json.loads(command(str(binary), *args, cwd=directory))
            if report.get("ok") is False:
                raise RuntimeError(f"CLI validation failed: {args}")
            checks.append(" ".join(args))
        if (directory / f"{mode}.2bpp").stat().st_size != 16:
            raise RuntimeError("Expected exactly one 16-byte tile")
        if not (directory / f"{mode}.gbr").is_file():
            raise RuntimeError("GBMB companion tiles are missing")
    return ["--help", *checks]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    arch, package = TARGETS[args.target]
    if platform.system() != "Darwin" or platform.machine() != arch:
        parser.error("Packaging must run on the target's native macOS architecture")
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_commit):
        parser.error("A full source commit hash is required")
    if os.environ.get("MACOSX_DEPLOYMENT_TARGET") != "11.0":
        parser.error("Set MACOSX_DEPLOYMENT_TARGET=11.0 for this distribution")
    binary = args.binary.resolve(strict=True)
    if command("lipo", "-archs", str(binary)) != arch:
        raise RuntimeError("Binary architecture differs from the native runner")
    root = Path(__file__).resolve().parents[1]
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    name = f"gbhua-{package}"
    archive = out / f"{name}.tar.gz"
    receipt_path = out / f"{name}.json"
    if archive.exists() or receipt_path.exists():
        raise FileExistsError("Refusing to overwrite an existing distribution")
    rust_docs = Path(command("rustc", "--print", "sysroot")) / "share/doc/rust"
    with tempfile.TemporaryDirectory(prefix="gbhua-package-") as temp:
        temp = Path(temp)
        bundle = temp / name
        bundle.mkdir()
        executable = bundle / "gbhua"
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        for path in sorted(root.glob("README*.md")):
            shutil.copy2(path, bundle / path.name)
        for filename in ("MACOS.md", "LICENSE", "THIRD_PARTY_NOTICES.md", "DEPENDENCIES.json"):
            shutil.copy2(root / filename, bundle / filename)
        shutil.copytree(root / "licenses", bundle / "licenses")
        rust_notices = bundle / "licenses/rust"
        shutil.copytree(rust_docs / "licenses", rust_notices / "licenses")
        shutil.copy2(rust_docs / "COPYRIGHT-library.html", rust_notices / "COPYRIGHT-library.html")
        receipt = {
            "application": "GBHUA CLI", "target": args.target,
            "source_commit": args.source_commit,
            "rustc": command("rustc", "--version"),
            "built_on": {"macos": platform.mac_ver()[0], "architecture": platform.machine()},
            "deployment_target": "11.0", "developer_id_signed": False, "notarized": False,
            "binary_sha256": sha256(executable),
            "binary_description": command("file", "-b", str(executable)),
            "load_commands": command("vtool", "-show-build", str(executable)),
        }
        (bundle / "BUILD.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        with tarfile.open(archive, "w:gz") as tar:
            for path in sorted(bundle.rglob("*")):
                if path.is_symlink():
                    raise RuntimeError(f"Unexpected symbolic link: {path}")
                if path.is_file():
                    info = tar.gettarinfo(str(path), arcname=str(path.relative_to(temp)))
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    info.mode = 0o755 if path == executable else 0o644
                    with path.open("rb") as stream:
                        tar.addfile(info, stream)
        unpacked = temp / "unpacked"
        unpacked.mkdir()
        command("tar", "-xzf", str(archive), "-C", str(unpacked))
        distributed = unpacked / name / "gbhua"
        if sha256(distributed) != receipt["binary_sha256"] or not os.access(distributed, os.X_OK):
            raise RuntimeError("Archive changed the executable or its permissions")
        fixture = temp / "smoke"
        fixture.mkdir()
        receipt["native_smoke_commands"] = smoke(distributed, fixture)
        receipt["archive"] = archive.name
        receipt["archive_sha256"] = sha256(archive)
        receipt["archive_bytes"] = archive.stat().st_size
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
