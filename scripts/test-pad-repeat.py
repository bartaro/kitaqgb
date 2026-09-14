"""Check GB key-repeat state transitions with original scripted input updates.

The ROM supplies synthetic key masks rather than relying on host keyboard timing.
KOKURA observes return values and final counters after 512 updates. This verifies
the scheduler logic, not controller electrical behavior or physical hardware.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def digest(path):
    """Associate results with exact input and output bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fixture(kind, delay, interval):
    """Sample the first repeat, saturation boundary and later long-interval repeats."""
    key = 8 if kind == 'down' else 1
    name = {'lr': '__padrep_lr', 'down': '__padrep_down', 'mask': '__padrep'}[kind]
    extra_parameter = ', u8 mask' if kind == 'mask' else ''
    extra_argument = ', 3' if kind == 'mask' else ''
    steps = (1, 2, 3, 253, 254, 255, 256, 257, 258, 509, 510, 512)
    text = f'''// Original scheduler regression: scripted masks, no commercial game data.
__location(0xC700) u8 result[16];
__location(0xC100) u8 state[5];
u16 step;
u8 movement;
void __padrep_init(u8* state, u8 das, u8 arr);
u8 {name}(u8* state, u8 keys, u8 trigger{extra_parameter});
void main() {{
result[15]=0;
__padrep_init(state,{delay},{interval});
result[0]={name}(state,{key},{key}{extra_argument});
for (step=1; step<=512; step++) {{
    movement={name}(state,{key},0{extra_argument});
'''
    text += '\n'.join(f'    if (step=={step}) result[{index + 1}]=movement;' for index, step in enumerate(steps))
    text += '\n}\nresult[13]=state[3]; result[14]=state[4]; result[15]=0xA5; while (1) { } }\n'
    # The initial press is followed by a repeat at DAS, then every ARR updates.
    expected = [key]
    expected += [key if step >= delay and (step == delay or interval == 0 or (step - delay) % interval == 0) else 0 for step in steps]
    expected += [255, 0 if interval == 0 else (512 - delay) % interval, 165]
    return text, expected


def run(command, directory, log):
    """Bound each process and retain diagnostics for both passing and failing cases."""
    process = subprocess.run(command, cwd=directory, capture_output=True, timeout=40)
    log.write_bytes(process.stdout + process.stderr)
    return process.returncode


def main():
    """Compile and execute all scheduler variants without hiding independent failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--emulator', type=Path, required=True)
    parser.add_argument('--output-parent', type=Path, required=True)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-pad-repeat-', dir=args.output_parent.resolve()))
    record = dict(compiler=str(compiler), compiler_sha256=digest(compiler), emulator=str(emulator),
        emulator_sha256=digest(emulator), cases=[], scope='Synthetic update sequences in KOKURA; no physical controller or hardware test.')
    for kind in ('lr', 'down', 'mask'):
        for delay in (2, 254, 255):
            for interval in (0, 3, 255):
                name = f'{kind}-das{delay}-arr{interval}'
                directory = output / name
                directory.mkdir()
                text, expected = fixture(kind, delay, interval)
                source = directory / 'case.c'
                source.write_text(text, encoding='ascii')
                rom = directory / 'case.gb'
                command = [str(compiler), str(source), '--cgb=dmg', '--profile=dev', '--rst-disable',
                    '--stack-bank=fixed', '--no-cache', '--no-disasm', '-o', str(rom)]
                compile_exit = run(command, directory, directory / 'compile.log')
                case = dict(name=name, command=command, compile_exit=compile_exit, source_sha256=digest(source), expected=expected, passed=False)
                if compile_exit == 0:
                    runtime = directory / 'runtime.json'
                    run_command = [str(emulator), str(rom), '--hardware', 'dmg', '--run-frames', '20',
                        '--watch-window', 'result:0xC700:16', '--dump-report', str(runtime)]
                    run_exit = run(run_command, directory, directory / 'run.log')
                    case.update(run_exit=run_exit, run_command=run_command, rom_sha256=digest(rom))
                    if run_exit == 0:
                        watches = [w for w in json.loads(runtime.read_text())['watched_memory'] if w['name'] == 'result']
                        assert len(watches) == 1 and watches[0]['addr'] == 0xC700 and watches[0]['size'] == 16
                        actual = watches[0]['preview_bytes']
                        case.update(actual=actual, passed=actual == expected, runtime_sha256=digest(runtime))
                record['cases'].append(case)
                (output / 'report.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    failures = [c['name'] for c in record['cases'] if not c['passed']]
    print(json.dumps(dict(report=str(output / 'report.json'), cases=len(record['cases']), failures=failures)))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
