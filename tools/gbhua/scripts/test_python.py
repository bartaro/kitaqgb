"""Test the freshly built wheel using an isolated temporary install."""
from pathlib import Path
import subprocess
import sys
import tempfile

wheels = list(Path(sys.argv[1]).glob("*.whl"))
if len(wheels) != 1:
    raise RuntimeError("Expected one freshly built Python wheel")
with tempfile.TemporaryDirectory(prefix="gbhua-python-") as directory:
    subprocess.run([sys.executable, "-m", "pip", "install", "--no-deps", "--target", directory,
                    str(wheels[0])], check=True)
    sys.path.insert(0, directory)
    import gbhua
    project = gbhua.Project(width=2, height=2, tile_count=4)
    project.set_tile_pixel(0, 1, 1, 3)
    project.save(str(Path(directory) / "test.gbh"))
    project.save_preview(str(Path(directory) / "test.png"), "cgb", 2)
    restored = gbhua.Project.from_image(str(Path(directory) / "test.png"), mode="cgb")
    restored.save(str(Path(directory) / "restored.gbh"))
    assert (Path(directory) / "restored.gbh").stat().st_size > 0
    print(restored.validate_json())
