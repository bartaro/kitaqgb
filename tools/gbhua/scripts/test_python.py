"""Test the freshly built wheel using an isolated temporary install."""
from pathlib import Path
import subprocess
import sys
import tempfile

SMOKE_TEST = """
from pathlib import Path
import json
import sys

directory = Path(sys.argv[1])
sys.path.insert(0, str(directory))
import gbhua

project = gbhua.Project(width=2, height=2, tile_count=4)
project.set_tile_pixel(0, 1, 1, 3)
project.save(str(directory / "test.gbh"))
project.save_preview(str(directory / "test.png"), "cgb", 2)
restored = gbhua.Project.from_image(str(directory / "test.png"), mode="cgb")
restored.save(str(directory / "restored.gbh"))
assert (directory / "restored.gbh").stat().st_size > 0
report = json.loads(restored.validate_json())
assert report["valid"] is True, report
print(json.dumps(report))
"""

wheels = list(Path(sys.argv[1]).glob("*.whl"))
if len(wheels) != 1:
    raise RuntimeError("Expected one freshly built Python wheel")
with tempfile.TemporaryDirectory(prefix="gbhua-python-") as directory:
    subprocess.run([sys.executable, "-m", "pip", "install", "--no-deps", "--target", directory,
                    str(wheels[0])], check=True)
    # Windows unloads the extension DLL when the child exits, before cleanup.
    subprocess.run([sys.executable, "-c", SMOKE_TEST, directory], check=True)
