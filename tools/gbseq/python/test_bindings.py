"""Native/CLI end-to-end smoke. Run after building the headless DLL and CLI."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import wave
from gbseq import Project, GbseqError

ROOT = Path(__file__).resolve().parents[1]
CLI = Path(os.environ.get("GBSEQ_CLI", ROOT / "target" / "release" / "gbseq-cli.exe"))


class BindingsTests(unittest.TestCase):
    def test_atomic_edit_and_roundtrip(self):
        p = Project()
        p.data["title"] = "Python roundtrip \u97f3\u697d"
        a = p.add_note(0, 0, 2, 24, 12)
        b = p.add_note(0, 4, 2, 28, 13)
        p.edit([a, b], "transpose", 12)
        self.assertEqual(p.data["tracks"][0]["notes"][0]["pitch"], 36)
        before = json.dumps(p.data, sort_keys=True)
        with self.assertRaises(GbseqError):
            p.edit([a], "move", 4)
        self.assertEqual(before, json.dumps(p.data, sort_keys=True))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "roundtrip_\u97f3\u697d.gbseq"
            p.save(path)
            self.assertEqual(Project.load(path).data, p.data)

    def test_cli_and_native_render_identical_wav(self):
        p = Project()
        p.data["measures"] = 1
        p.data["frames_per_step_override"] = 1
        p.data["loop_playback"] = False
        p.add_note(0, 0, 8, 24, 15)
        p.add_note(2, 0, 8, 12, 15)
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            project = tmp / "test.gbseq"
            native = tmp / "native.wav"
            cli_wav = tmp / "cli.wav"
            p.save(project)
            p.render_wav(native)
            subprocess.run([str(CLI), "render-wav", str(project), str(cli_wav)], check=True)
            self.assertEqual(native.read_bytes(), cli_wav.read_bytes())
            with wave.open(str(native)) as audio:
                self.assertEqual(audio.getnchannels(), 2)
                self.assertGreater(audio.getnframes(), 1000)
                self.assertTrue(any(audio.readframes(audio.getnframes())))
            failed = subprocess.run([str(CLI), "add-note", str(project), "--track", "1", "--start", "0", "--out", str(tmp / "invalid.gbseq")], capture_output=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse((tmp / "invalid.gbseq").exists())

    def test_errors_and_effect_data(self):
        p = Project()
        with self.assertRaises(GbseqError):
            p.add_note(0, 0, pitch=48)
        a = p.add_note(0, 0)
        p.data["tracks"][0]["notes"][0]["effects"]["duty"] = 3
        p.validate()
        copies = p.edit([a], "duplicate")
        self.assertEqual(len(copies), 1)
        self.assertEqual(p.data["tracks"][0]["notes"][1]["effects"]["duty"], 3)
        p.edit(copies, "delete")
        self.assertEqual(len(p.data["tracks"][0]["notes"]), 1)

    def test_native_exports_without_gui(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            project_path = tmp / "sixteen.gbseq"
            subprocess.run([str(CLI), "new", str(project_path), "--bars", "1", "--players", "4"], check=True)
            p = Project.load(project_path)
            p.add_note(15, 0, 2, 24, 12)
            report = p.export_kitaqgb(tmp / "kitaqgb", player=3)
            self.assertGreater(report["bytes"], 0)
            manifest = p.export_link16(tmp / "link16")
            self.assertEqual(len(manifest["files"]), 4)
            for entry in manifest["files"]:
                self.assertEqual(Path(entry["path"]).stat().st_size, entry["bytes"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
