# GBSEQ GUI, CLI and Python

## Build

GUI (plita) and CLI:

```powershell
cargo build --release --bins
```

Headless CLI and native Python library, without plita, GPU or audio-device dependencies:

```powershell
cargo build --release --no-default-features --lib --bin gbseq-cli
```

Windows outputs: `target/release/gbseq.exe`, `gbseq-cli.exe`, `gbseq.dll`.
The DLL is a C ABI library, not a CPython extension: no Python-version-specific build is required.
Use a 64-bit Python interpreter with the 64-bit Windows build.

## GUI

- File/Edit/View/Export menus use plita's embedded Lucide vector icons.
- Ctrl/Shift-click toggles note selection. Ctrl+A selects all notes in the current lane.
- Arrow keys move or transpose a selection; Shift+Up/Down transposes an octave.
- Ctrl+D duplicates, Delete removes, Ctrl+Z/Ctrl+Y undo/redo.
- G cycles the 1/16, 1/8, 1/4 grid; Q aligns selected note starts to that grid.
- F fits the complete song; Home returns to the beginning; +/- zoom.
- Ctrl+S saves, Ctrl+Shift+S saves as. New/Open/drop warns before replacing unsaved edits.
- Double-click still opens per-note effects; drag the right edge to resize.
- WAV export runs on an immutable snapshot in the background, so editing can continue.
- The bar-count box supports 100-bar and longer songs within the existing model limit.
- `gbseq.exe --open path/to/song.gbseq` opens a project without the file dialog.

Group edits are atomic: an overlap, out-of-range pitch or song boundary rejects the whole operation.
The four physical channels remain CH1/CH2/CH3/CH4. LINK16 uses separate four-channel players.

## CLI

CLI tracks and players are **one-based**. Starts are **zero-based sixteenth-note steps**;
length 1 = 0.25 beat, length 2 = 0.5 beat, length 16 = one 4/4 bar.
For frame-authored SFX, each step instead lasts `frames_per_step_override` Game Boy frames.
Commands that edit a project require `--out`; the source is not implicitly overwritten.

```powershell
.\dist\gbseq-cli.exe new out\song.gbseq --bars 100 --bpm 150
.\dist\gbseq-cli.exe add-note out\song.gbseq --track 1 --start 0 --length 4 --pitch 24 --out out\song.gbseq
.\dist\gbseq-cli.exe notes out\song.gbseq --track 1
.\dist\gbseq-cli.exe transform out\song.gbseq --track 1 --transpose 12 --out out\higher.gbseq
.\dist\gbseq-cli.exe duplicate-notes out\song.gbseq --ids 1 --out out\double.gbseq
.\dist\gbseq-cli.exe validate out\song.gbseq
.\dist\gbseq-cli.exe info out\song.gbseq --json
.\dist\gbseq-cli.exe render-wav out\song.gbseq out\song.wav
.\dist\gbseq-cli.exe render-wav song.gbseq out\all16.wav --all-players
.\dist\gbseq-cli.exe export-kitaqgb out\song.gbseq out\kitaqgb --player 1
.\dist\gbseq-cli.exe export-link16-sav out\song.gbseq out\saves
.\dist\gbseq-cli.exe bench-render song.gbseq --all-players --frames 600 --runs 3
```

`transform` supports `--shift`, `--transpose`, `--quantize` and `--velocity`.
Effects and waveform data remain in the documented project JSON and are fully exposed by Python.
Invalid operations return a nonzero exit code. `--help` lists all arguments.
The GUI executable also accepts these CLI commands for compatibility.

## Python Bindings

From the GBSEQ directory:

```powershell
$env:PYTHONPATH = "$PWD\python"
$env:GBSEQ_NATIVE_LIBRARY = "$PWD\dist\gbseq.dll"
python python\test_bindings.py
```

An editable install with `python -m pip install -e ./python` is also supported.
For a non-editable install, set `GBSEQ_NATIVE_LIBRARY` to the absolute DLL/library path.
If unset, the source package searches `dist`, `target/release`, then `target/debug`.
Set `GBSEQ_CLI` when running the tests against a CLI outside `target/release`.

```python
from gbseq import Project, GbseqError

song = Project()
song.data["title"] = "Python sequence"
song.data["measures"] = 100
note = song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.edit([note], "transpose", 12)
song.data["tracks"][0]["notes"][0]["effects"]["duty"] = 3
song.validate()
song.save("out/python.gbseq")
song.render_wav("out/python.wav")
song.export_kitaqgb("out/python_export")

loaded = Project.load("out/python.gbseq")
loaded.render_wav("out/mix.wav", player=None)  # All active players
```

Python tracks and players are **zero-based**. `player=None` mixes all active players.
`Project.data` exposes the complete format (note effects, CH3 waves, exact frame timing, link mode).
Call `validate()` after modifying it directly; call `Project.from_dict(data)` to validate imported data.
`edit(ids, action, value)` supports move, transpose, velocity, quantize, delete and duplicate.
Failed native edits raise `GbseqError` and leave `Project.data` unchanged.
No subprocess, GUI or sound device is involved in these bindings. ctypes releases the GIL during the native call.
Use separate Project objects if multiple threads are editing simultaneously.

## Playback Performance

- The existing KOKURA tick path is retained. The trace-free path was tested for PCM equality, but timing did not establish a reliable speed benefit in this workload.
- Independent players render concurrently (maximum four). Mixing order and integer normalization stay deterministic.
- One completed playback is cached by the entire project snapshot and player selection, capped at 128 MiB.
- Replay shares the cached PCM buffer; edits, tempo, wave data, mute/solo and player changes invalidate the key.
- Stop cancels the worker between 16-frame chunks. Audition remains a short, uncached render.
- First playback still renders the complete song. This is not a streaming playback engine.

`bench-render --serial` provides the reference mixing path. Compare PCM checksums as well as elapsed times.
Performance depends on machine load and song length; timings are measurements, not guarantees.
