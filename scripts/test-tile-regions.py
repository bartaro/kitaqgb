"""Check GB tile-region transfers with generated original ROMs and KOKURA.

Exercises byte counts, zero dimensions, row/column specializations, page crossings
and neighboring cells. Tests use LCD-off access; they do not certify LCD-on or
physical hardware timing. All build commands and runtime observations are retained.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def digest(path):
    """Bind test evidence to the exact source, executable or output bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fixture(kind, width, height, dynamic):
    """Use distinct payload bytes and inspect copied cells plus untouched neighbors."""
    x, y = 2, 7
    base = 0x9800 if kind == 'map' else 0x9C00
    cells = {}
    count = width * height if kind in ('rect', 'map') else width
    for index in range(count):
        if kind in ('rect', 'map'):
            dx, dy = index % width, index // width
        elif kind == 'col':
            dx, dy = 0, index
        else:
            dx, dy = index, 0
        cells[(y + dy) * 32 + x + dx] = 7 if kind == 'rect' else 7 + index * 2
    # Bound each watch to 16 bytes while checking ends of long rows and columns.
    written = sorted(cells)
    samples = written if len(written) <= 9 else [written[i] for i in (0, 1, width - 1, width, len(written) - 2, len(written) - 1)]
    offsets = sorted(set(samples + [y * 32 + x, y * 32 + x - 1, y * 32 + x + max(width, 1),
        (y - 1) * 32 + x, (y + (width if kind == 'col' else max(height, 1))) * 32 + x]))
    assert len(offsets) <= 15
    source = '''// Original regression fixture: no external game, graphics or font data.
__location(0xFF40) u8 lcdc;
__location(0xC700) u8 result[16];
__location(0xC100) u8 payload[32];
u8 x; u8 y; u8 width; u8 height; u16 mapbase;
void __settile_rect(u8 x, u8 y, u8 w, u8 h, u8 tile);
void __settile_row(u8 x, u8 y, const u8* src, u8 len);
void __settile_col(u8 x, u8 y, const u8* src, u8 len);
void __settilemap_rect(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src);
'''
    source += '\n'.join(f'__location({base + offset}) u8 cell{i};' for i, offset in enumerate(offsets))
    source += '\nvoid main() { lcdc=9; result[15]=0;\n'
    source += ' '.join(f'cell{i}=0xCC;' for i in range(len(offsets))) + '\n'
    source += ' '.join(f'payload[{i}]={7 + i * 2};' for i in range(max(count, 1))) + '\n'
    source += f'x={x}; y={y}; width={width}; height={height}; mapbase={base};\n'
    ax, ay, aw, ah, ab = ('x', 'y', 'width', 'height', 'mapbase') if dynamic else tuple(map(str, (x, y, width, height, base)))
    call = {'rect': f'__settile_rect({ax},{ay},{aw},{ah},7)',
            'row': f'__settile_row({ax},{ay},payload,{aw})',
            'col': f'__settile_col({ax},{ay},payload,{aw})',
            'map': f'__settilemap_rect({ab},{ax},{ay},{aw},{ah},payload)'}[kind]
    source += call + ';\n'
    source += ' '.join(f'result[{i}]=cell{i};' for i in range(len(offsets)))
    source += '\nresult[15]=0xA5; while (1) { } }\n'
    return source, offsets, [cells.get(offset, 0xCC) for offset in offsets]


def run(command, directory, log):
    """Execute one bounded process and preserve its exact diagnostic bytes."""
    process = subprocess.run(command, cwd=directory, capture_output=True, timeout=30)
    log.write_bytes(process.stdout + process.stderr)
    return process.returncode


def main():
    """Build and observe every independent case, retaining failures in the report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-tile-regions-', dir=args.output_parent.resolve()))
    record = dict(compiler=str(compiler), compiler_sha256=digest(compiler), emulator=str(emulator),
        emulator_sha256=digest(emulator), scope='LCD-off KOKURA execution; sampled written/neighbor cells; no physical hardware.', cases=[])
    for kind in ('rect', 'row', 'col', 'map'):
        dimensions = [(n, 1) for n in (0, 1, 3, 9)]
        if kind in ('rect', 'map'):
            dimensions = [(0, 0), (0, 2), (3, 0), (1, 1), (3, 2), (9, 2)]
        if kind == 'map':
            dimensions += [(1, 3), (1, 9)]
        for width, height in dimensions:
            for dynamic in (False, True):
                name = f'{kind}-{width}x{height}-' + ('runtime' if dynamic else 'constant')
                directory = output / name
                directory.mkdir()
                text, offsets, expected = fixture(kind, width, height, dynamic)
                source = directory / 'case.c'
                source.write_text(text, encoding='ascii')
                rom = directory / 'case.gb'
                command = [str(compiler), str(source), '--cgb=dmg', '--profile=dev', '--rst-disable',
                    '--stack-bank=fixed', '--no-cache', '--no-disasm', '-o', str(rom)]
                compile_exit = run(command, directory, directory / 'compile.log')
                case = dict(name=name, expected=expected, map_offsets=offsets, source_sha256=digest(source),
                    command=command, compile_exit=compile_exit, passed=False)
                if compile_exit == 0:
                    runtime = directory / 'runtime.json'
                    run_command = [str(emulator), str(rom), '--hardware', 'dmg', '--run-frames', '4',
                        '--watch-window', 'result:0xC700:16', '--dump-report', str(runtime)]
                    run_exit = run(run_command, directory, directory / 'run.log')
                    case.update(run_exit=run_exit, run_command=run_command, rom_sha256=digest(rom))
                    if run_exit == 0:
                        watches = [w for w in json.loads(runtime.read_text())['watched_memory'] if w['name'] == 'result']
                        assert len(watches) == 1 and watches[0]['addr'] == 0xC700 and watches[0]['size'] == 16
                        actual = watches[0]['preview_bytes']
                        case.update(actual=actual, passed=actual[:len(expected)] == expected and actual[15] == 165,
                            runtime_sha256=digest(runtime))
                record['cases'].append(case)
                (output / 'report.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    failures = [case['name'] for case in record['cases'] if not case['passed']]
    print(json.dumps(dict(report=str(output / 'report.json'), cases=len(record['cases']), failures=failures)))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
