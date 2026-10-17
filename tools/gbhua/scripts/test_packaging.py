import tempfile
import hashlib
import json
import os
import zipfile
import unittest
from pathlib import Path

from package_gui import LANGUAGES, license_files, zip_bundle


class PackagingTests(unittest.TestCase):
    def test_zip_retains_epoch_dated_registry_notices(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = root / "gbhua"
            bundle.mkdir()
            notice = bundle / "LICENSE"
            notice.write_text("upstream notice", encoding="utf-8")
            os.utime(notice, (1, 1))
            zip_bundle(bundle, root / "gui.zip")
            with zipfile.ZipFile(root / "gui.zip") as archive:
                self.assertEqual(archive.read("gbhua/LICENSE"), b"upstream notice")
                self.assertEqual(archive.getinfo("gbhua/LICENSE").date_time[0], 1980)

    def test_license_supplements_match_their_source_hashes(self):
        root = Path(__file__).resolve().parents[1] / "licenses/overrides"
        records = json.loads((root / "SOURCES.json").read_text(encoding="utf-8-sig"))["files"]
        self.assertGreater(len(records), 0)
        recorded = set()
        for record in records:
            path = root / record["package"] / record["file"]
            self.assertTrue(path.resolve().is_relative_to(root.resolve()))
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), record["sha256"])
            recorded.add(path)
        self.assertEqual(recorded, {p for d in root.iterdir() if d.is_dir() for p in d.rglob("*") if p.is_file()})

    def test_source_files_are_not_mistaken_for_license_notices(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("LICENSE-MIT", "LICENSE.APACHE", "LICENSE.md", "NOTICE", "COPYING.rs", "copyright.c"):
                (root / name).touch()
            self.assertEqual({p.name for p in license_files(root)},
                             {"LICENSE-MIT", "LICENSE.APACHE", "LICENSE.md", "NOTICE"})

    def test_all_nine_gui_manuals_and_local_links_exist(self):
        root = Path(__file__).resolve().parents[1]
        import re
        self.assertEqual(len(LANGUAGES), 9)
        for language in LANGUAGES:
            path = root / f"docs/manual.{language}.md"
            text = path.read_text(encoding="utf-8")
            self.assertIn("--features gui --bin gbhua_gui", text)
            self.assertNotIn("C:\\kitaqgb_project", text)
            for link in re.findall(r"\]\(([^)]+)\)", text):
                if not link.startswith(("http:", "https:", "#")):
                    self.assertTrue((path.parent / link.split("#")[0]).is_file(), link)


if __name__ == "__main__":
    unittest.main()
