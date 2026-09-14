"""Build ROMs and check GB tile/attribute coordinates with KOKURA CLI.

Use --compiler, --emulator and --output-parent to select executables and outputs.
The suite uses generated original data, observes both VRAM banks through ROM code,
and retains exact result watches. It does not certify physical hardware behavior.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def digest(path):
    """Bind a result to the exact input or generated artifact."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, directory, log, allow_failure=False):
    """Save process diagnostics and bound compilation/emulation individually."""
    try:
        process = subprocess.run(command, cwd=directory, capture_output=True, timeout=30)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b'') + (error.stderr or b''))
        raise
    log.write_bytes(process.stdout + process.stderr)
    if process.returncode and not allow_failure:
        raise RuntimeError('Command failed: ' + str(log))
    return process.returncode


def prelude():
    return '''// Generated regression fixture; no external game or font data.
__location(0xFF40) u8 lcdc;
__location(0xFF4F) u8 vbk;
__location(0xC700) u8 result[16];
u8 input_x;
u8 input_y;
u8 input_tile;
u8 input_attr;
u16 input_base;
'''


def scalar_fixture(target, combined, unsafe, hardware):
    """Use distinct X, Y, tile and attribute values to expose register mix-ups."""
    suffix = {'default': '', 'at': 'at', 'win': 'win', 'bg': 'bg'}[target]
    intrinsic = '__settile' + suffix + ('cgb' if combined else 'attr')
    intrinsic += '_unsafe' if unsafe else ''
    base = 0x9800 if target == 'default' else 0x9C00
    lcdc = 0x41 if target == 'win' else (9 if target == 'bg' else 1)
    declaration = ('u16 base, ' if target == 'at' else '') + 'u8 x, u8 y, '
    declaration += 'u8 tile, u8 attr' if combined else 'u8 attr'
    arguments = ('input_base, ' if target == 'at' else '') + 'input_x, input_y, '
    arguments += 'input_tile, input_attr' if combined else 'input_attr'
    rows = (3, 1, 2, 5)
    source = prelude() + '\n'.join(f'__location({base + row * 32 + 2}) u8 cell_{row};' for row in rows)
    source += f'\nvoid {intrinsic}({declaration});\nvoid main() {{\n'
    source += f'lcdc={lcdc}; result[15]=0;\n'
    for bank in (0, 1):
        source += f'vbk={bank}; ' + ' '.join(f'cell_{row}=0;' for row in rows) + '\n'
    source += f'vbk=0; input_x=2; input_y=3; input_tile=7; input_attr=5; input_base={base};\n'
    source += f'{intrinsic}({arguments});\nresult[0]=vbk;\n'
    expected = [255 if hardware == 'dmg' else 254]
    # On DMG, selecting VBK has no effect: both reads observe the tile bank.
    for bank in (0, 1):
        source += f'vbk={bank};\n'
        for index, row in enumerate(rows):
            source += f'result[{1 + bank * 4 + index}]=cell_{row};\n'
            value = 0
            if row == 3:
                value = (7 if combined else 0) if bank == 0 or hardware == 'dmg' else 5
            expected.append(value)
    source += 'result[15]=0xA5; while (1) { }\n}\n'
    return source, expected


def buffered_fixture(count, hardware):
    """Check both copied rows, end columns and untouched neighboring columns."""
    source = prelude() + '''
__location(0xC100) u8 tiles[1024];
__location(0xC800) u8 attrs[1024];
u16 i;
u8 copy_count;
void __settilebg16cgb_flush(const u8* tiles, const u8* attrs, u8 y, u8 x, u8 count);
'''
    # Y is a 16-pixel cell row, so rows 6 and 7 are copied from both RAM buffers.
    offsets = [6 * 32 + 2, 6 * 32 + 1 + count, 7 * 32 + 2, 7 * 32 + 1 + count,
               6 * 32 + 1, 6 * 32 + 2 + count]
    source += '\n'.join(f'__location({0x9C00 + offset}) u8 cell_{n};' for n, offset in enumerate(offsets))
    source += '\nvoid main() { lcdc=9; result[15]=0;\n'
    source += 'for (i=0; i<1024; i++) { tiles[i]=7; attrs[i]=5; }\n'
    source += 'for (i=224; i<256; i++) { tiles[i]=9; attrs[i]=6; }\n'
    for bank in (0, 1):
        source += f'vbk={bank}; ' + ' '.join(f'cell_{n}=0;' for n in range(6)) + '\n'
    # Exercise a tiny constant-copy expansion and a dynamic shared-copy path.
    argument = str(count) if count == 2 else 'copy_count'
    source += f'vbk=0; copy_count={count}; __settilebg16cgb_flush(tiles, attrs, 3, 2, {argument});\n'
    source += 'result[0]=vbk;\n'
    expected = [255 if hardware == 'dmg' else 254]
    for bank in (0, 1):
        source += f'vbk={bank};\n'
        for n in range(6):
            source += f'result[{1 + bank * 6 + n}]=cell_{n};\n'
        expected += [7, 7, 9, 9, 0, 0] if bank == 0 or hardware == 'dmg' else [5, 5, 6, 6, 0, 0]
    source += 'result[15]=0xA5; while (1) { }\n}\n'
    return source, expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    parser.add_argument('--filter', default='', help='Run only case names containing this text.')
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-tile-coordinates-', dir=args.output_parent.resolve()))
    cases = []
    for mode in ('cgb_only', 'cgb'):
        for target in ('default', 'at', 'win', 'bg'):
            for combined in (False, True):
                for unsafe in (False, True):
                    name = mode + '-' + target + ('-combined' if combined else '-attr') + ('-unsafe' if unsafe else '-safe')
                    cases.append((name, mode, 'cgb', *scalar_fixture(target, combined, unsafe, 'cgb')))
    for mode in ('dmg', 'cgb'):
        for target in ('default', 'at', 'win', 'bg'):
            cases.append((mode + '-dmg-' + target, mode, 'dmg', *scalar_fixture(target, True, False, 'dmg')))
    for mode, hardware in (('cgb_only', 'cgb'), ('cgb', 'cgb'), ('dmg', 'dmg'), ('cgb', 'dmg')):
        for count in (2, 9):
            cases.append((mode + '-' + hardware + '-buffer-' + str(count), mode, hardware, *buffered_fixture(count, hardware)))
    record = dict(compiler=str(compiler), compiler_sha256=digest(compiler),
        emulator=str(emulator), emulator_sha256=digest(emulator), cases=[],
        scope='Generated ROM execution and KOKURA CPU-memory watches; no physical hardware.')
    cases = [case for case in cases if args.filter in case[0]]
    if not cases:
        parser.error('The filter selected no cases.')
    for name, mode, hardware, text, expected in cases:
        directory = output / name
        directory.mkdir()
        source = directory / 'case.c'
        source.write_text(text, encoding='ascii')
        rom = directory / 'case.gb'
        command = [str(compiler), str(source), '--cgb=' + mode, '--profile=dev', '--rst-disable',
            '--stack-bank=fixed', '--no-cache', '--no-disasm', '-o', str(rom)]
        compile_exit = run(command, directory, directory / 'compile.log', allow_failure=True)
        if compile_exit:
            # Keep testing independent cases so one build error does not conceal others.
            record['cases'].append(dict(name=name, passed=False, compile_exit=compile_exit,
                command=command, source_sha256=digest(source), compile_log_sha256=digest(directory / 'compile.log')))
            (output / 'report.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
            continue
        report_path = directory / 'runtime.json'
        run_command = [str(emulator), str(rom), '--hardware', hardware, '--run-frames', '20' if '-buffer-' in name else '3',
            '--watch-window', 'result:0xC700:16', '--dump-report', str(report_path)]
        run(run_command, directory, directory / 'run.log')
        report = json.loads(report_path.read_text(encoding='utf-8'))
        watches = [w for w in report['watched_memory'] if w['name'] == 'result']
        if len(watches) != 1 or watches[0]['addr'] != 0xC700 or watches[0]['size'] != 16:
            raise RuntimeError('Missing result watch: ' + name)
        actual = watches[0]['preview_bytes']
        passed = len(actual) == 16 and actual[:len(expected)] == expected and actual[15] == 165
        record['cases'].append(dict(name=name, expected=expected, actual=actual, passed=passed,
            source_sha256=digest(source), rom_sha256=digest(rom), runtime_sha256=digest(report_path),
            command=command, run_command=run_command))
        (output / 'report.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    failures = [case['name'] for case in record['cases'] if not case['passed']]
    print(json.dumps(dict(report=str(output / 'report.json'), cases=len(cases), failures=failures)))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
