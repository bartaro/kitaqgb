"""Check that a warm compiler cache cannot hide a missing source argument.

Run on Windows with a built KITAQGB compiler. All sources, ROMs, logs and cache
entries are confined to a new directory beneath --output-parent. This checks
compiler CLI behavior only; it does not execute the emitted ROM on hardware.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='gb-cache-inputs-', dir=args.output_parent.resolve()))
    # A single complete source makes an extra missing file invisible to the old
    # key builder, which silently omitted nonexistent source arguments.
    (root / 'main.c').write_text('void main() { while (1) { } }\n', encoding='ascii')
    cases = []
    cold_hash = None
    specifications = [
        ('cold', [], True, False),
        ('warm', [], True, True),
        ('missing-relative', ['absent.c'], False, False),
        ('missing-absolute', [str(root / 'missing file.c')], False, False),
        ('missing-without-cache', ['--no-cache', 'absent.c'], False, False),
        ('valid-after-errors', [], True, True),
    ]
    for label, extra, should_succeed, should_hit in specifications:
        command = [str(compiler), 'main.c', '-o', 'program.gb', '--cache'] + extra
        # Capture byte streams to avoid locale-dependent decoding failures.
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=30,
                                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        (root / (label + '.stdout.log')).write_bytes(result.stdout)
        (root / (label + '.stderr.log')).write_bytes(result.stderr)
        hit = b'[cache] hit:' in result.stdout
        rom = root / 'program.gb'
        digest = hashlib.sha256(rom.read_bytes()).hexdigest() if rom.exists() else None
        if label == 'cold':
            cold_hash = digest
        passed = (result.returncode == 0) == should_succeed and hit == should_hit
        if should_succeed:
            passed = passed and digest is not None and digest == cold_hash
        # A failed compilation may leave the prior ROM in place. Its presence is
        # recorded for diagnostics but is not considered evidence of success.
        cases.append(dict(name=label, command=command, exit=result.returncode,
                          cache_hit=hit, existing_rom_sha256=digest, passed=passed))
    artifacts = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in root.rglob('*') if p.is_file()}
    report = dict(compiler=str(compiler), compiler_sha256=hashlib.sha256(compiler.read_bytes()).hexdigest(),
                  scope='Missing source validation on the real compiler CLI with a warm cache; no emulator/hardware claim.',
                  cases=cases, artifacts=artifacts)
    (root / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(dict(report=str(root / 'report.json'), checks=len(cases),
                          failures=[c['name'] for c in cases if not c['passed']])))
    return 0 if all(c['passed'] for c in cases) else 1


if __name__ == '__main__':
    raise SystemExit(main())
