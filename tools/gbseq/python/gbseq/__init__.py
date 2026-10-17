"""GBSEQ native bindings. No GUI, subprocess, PyO3 or third-party Python packages.

Tracks and players are zero-based here. Time is in sixteenth-note steps.
The CLI intentionally uses one-based --track/--player for human-facing commands.
"""
import ctypes
import json
import os
from pathlib import Path
import sys

__all__ = ["Project", "GbseqError"]
_native = None


class GbseqError(RuntimeError):
    """An invalid project, edit, export, or native-render request."""


def _library():
    global _native
    if _native is None:
        root = Path(__file__).resolve().parents[2]
        name = "gbseq.dll" if sys.platform == "win32" else ("libgbseq.dylib" if sys.platform == "darwin" else "libgbseq.so")
        override = os.environ.get("GBSEQ_NATIVE_LIBRARY")
        candidates = [Path(override)] if override else [root / "dist" / name, root / "target" / "release" / name, root / "target" / "debug" / name]
        path = next((p for p in candidates if p.is_file()), None)
        if path is None:
            raise GbseqError("Native library not found. Build with cargo build --release --no-default-features --lib, or set GBSEQ_NATIVE_LIBRARY.")
        lib = ctypes.CDLL(str(path))
        lib.gbseq_call_json.argtypes = [ctypes.c_char_p]
        lib.gbseq_call_json.restype = ctypes.c_void_p
        lib.gbseq_free_json.argtypes = [ctypes.c_void_p]
        lib.gbseq_free_json.restype = None
        _native = lib
    return _native


def _call(op, **kwargs):
    lib = _library()
    pointer = lib.gbseq_call_json(json.dumps(dict(op=op, **kwargs), ensure_ascii=False).encode("utf-8"))
    if not pointer:
        raise GbseqError("Native library returned a null response")
    try:
        response = json.loads(ctypes.string_at(pointer).decode("utf-8"))
    finally:
        lib.gbseq_free_json(pointer)
    if not response["ok"]:
        raise GbseqError(response["error"])
    return response["result"]


class Project:
    """Editable project data; native operations validate and commit atomically.

    `data` exposes the complete format, including link modes, effects and CH3
    waveforms. Call validate() after changing fields directly.
    """
    def __init__(self, *, demo=False):
        self.data = _call("new", demo=demo)

    @classmethod
    def load(cls, path):
        project = cls.__new__(cls)
        project.data = _call("load", path=os.fspath(path))
        return project

    @classmethod
    def from_dict(cls, data):
        project = cls.__new__(cls)
        project.data = json.loads(json.dumps(data))
        project.validate()
        return project

    def validate(self):
        return _call("validate", project=self.data)["summary"]

    def save(self, path):
        return _call("save", project=self.data, path=os.fspath(path))

    def add_note(self, track, start, length=2, pitch=24, velocity=15):
        result = _call("add_note", project=self.data, track=track, start=start, length=length, pitch=pitch, velocity=velocity)
        self.data = result["project"]
        return result["id"]

    def edit(self, ids, action, value=0):
        result = _call("edit", project=self.data, ids=list(ids), action=action, value=value)
        self.data = result["project"]
        return result["ids"]

    def render_wav(self, path, *, player=0):
        """Render without an audio device. player=None mixes all active players."""
        return _call("render_wav", project=self.data, path=os.fspath(path), player=player)

    def export_kitaqgb(self, directory, *, player=0):
        return _call("export_kitaqgb", project=self.data, path=os.fspath(directory), player=player)

    def export_link16(self, directory):
        return _call("export_link16", project=self.data, path=os.fspath(directory))
