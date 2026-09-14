"""Check scalar CGB tile-attribute coordinates through compiled ROM execution.

Pass --compiler and --emulator paths to KITAQGB and KOKURA CLI respectively.
All fixtures and reports are written to a fresh directory under --output-parent.
Only generated test data is used. This checks emulator behavior, not hardware.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def sha256(path):
    """Identify the exact executable, source or output used by a case."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(command, directory, log):
    """Bound each child process and keep its diagnostics even when it fails."""
    try:
        result = subprocess.run(command, cwd=directory, capture_output=True, timeout=30)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b'') + (error.stderr or b''))
        raise
    log.write_bytes(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError('Command failed; see ' + str(log))


def make_source(x, y, unsafe, lcd_on, dynamic):
    """Observe the requested cell and row 1 separately, with distinct X/Y values."""
    intrinsic = '__settileattr' + ('_unsafe' if unsafe else '')
    call = 'input_x, input_y, input_attr' if dynamic else f'{x}, {y}, 5'
    return f'''// Generated scalar attribute-coordinate regression; no external assets.
__location(0xFF40) u8 lcdc;
__location(0xFF4F) u8 vbk;
__location({0x9800 + y * 32 + x}) u8 requested_cell;
__location({0x9820 + x}) u8 row_one_cell;
__location(0xC700) u8 result_bank;
__location(0xC701) u8 result_requested;
__location(0xC702) u8 result_row_one;
__location(0xC703) u8 done;
u8 input_x;
u8 input_y;
u8 input_attr;
void {intrinsic}(u8 x, u8 y, u8 attr);
void main() {{
    // Clear the observed cells in both banks while rendering is disabled.
    lcdc=1; done=0;
    vbk=0; requested_cell=0; row_one_cell=0;
    vbk=1; requested_cell=0; row_one_cell=0; vbk=0;
    input_x={x}; input_y={y}; input_attr=5;
    lcdc={0x91 if lcd_on else 1};
    {intrinsic}({call});
    // Capture the returned bank before selecting attributes for inspection.
    result_bank=vbk; lcdc=1;
    vbk=1; result_requested=requested_cell; result_row_one=row_one_cell;
    done=0xA5;
    while (1) {{ }}
}}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-tile-attributes-', dir=args.output_parent.resolve()))
    cases = []
    # Exercise compile-time CGB selection and the runtime-detection path.
    for mode in ('cgb_only', 'cgb'):
        for label, x, y, unsafe, lcd_on, dynamic in (
            ('safe', 2, 3, False, False, False),
            ('unsafe', 2, 3, True, False, False),
            ('top-right', 31, 0, False, False, True),
            ('bottom-left', 0, 31, True, False, True),
            ('lcd-on', 11, 9, False, True, True),
        ):
            cases.append((mode + '-' + label, mode, 'cgb', x, y, unsafe, lcd_on, dynamic))
    # Both known-DMG and dual-mode ROMs must skip CGB attribute writes on DMG.
    for mode in ('dmg', 'cgb'):
        for unsafe in (False, True):
            cases.append((mode + '-on-dmg-' + ('unsafe' if unsafe else 'safe'),
                          mode, 'dmg', 2, 3, unsafe, False, False))
    report = dict(compiler=str(compiler), compiler_sha256=sha256(compiler),
        emulator=str(emulator), emulator_sha256=sha256(emulator), cases=[],
        scope='Complete compiler CLI and KOKURA watched CPU-memory results; no physical hardware test.')
    for name, mode, hardware, x, y, unsafe, lcd_on, dynamic in cases:
        directory = output / name
        directory.mkdir()
        source = directory / 'case.c'
        source.write_text(make_source(x, y, unsafe, lcd_on, dynamic), encoding='ascii')
        rom = directory / 'case.gb'
        command = [str(compiler), str(source), '--cgb=' + mode, '--profile=dev',
            '--rst-disable', '--stack-bank=fixed', '--no-cache', '--no-disasm', '-o', str(rom)]
        execute(command, directory, directory / 'compile.log')
        runtime_report = directory / 'runtime.json'
        run_command = [str(emulator), str(rom), '--hardware', hardware, '--run-frames', '2',
            '--watch-window', 'result:0xC700:4', '--dump-report', str(runtime_report)]
        execute(run_command, directory, directory / 'run.log')
        state = json.loads(runtime_report.read_text(encoding='utf-8'))
        watches = [watch for watch in state['watched_memory'] if watch['name'] == 'result']
        # Validate the observation address/length as well as the completion marker.
        if len(watches) != 1 or watches[0]['addr'] != 0xC700 or watches[0]['size'] != 4:
            raise RuntimeError('Missing or unexpected result watch: ' + name)
        actual = watches[0]['preview_bytes']
        expected = [255, 0, 0, 165] if hardware == 'dmg' else [254, 5, 0, 165]
        report['cases'].append(dict(name=name, actual=actual, expected=expected,
            passed=actual == expected, command=command, run_command=run_command,
            source_sha256=sha256(source), rom_sha256=sha256(rom),
            runtime_sha256=sha256(runtime_report)))
        # Keep completed results if a later case fails to compile or execute.
        (output / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    failures = [case['name'] for case in report['cases'] if not case['passed']]
    print(json.dumps(dict(report=str(output / 'report.json'), cases=len(cases), failures=failures)))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
