"""Check CGB palette byte pairs and untouched neighbors in KOKURA with LCD off.

The generated ROMs contain original test code only. This validates palette RAM
readback, not visible rendering, LCD-on timing or physical hardware.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def digest(path):
    """Identify the exact inputs and retained runtime evidence."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fixture(channel, slot, mode):
    """Initialize all colors, overwrite one, then read it and its two neighbors."""
    index = 0xFF68 if channel == 'bg' else 0xFF6A
    source = '''#include "cgb_palette.h"
__location(0xFF40) u8 lcdc;
__location(0xC700) u8 result[16];
u8 i;
u16 color;
u8 palette;
u8 entry;
'''
    source += f'__location({index}) u8 pi;\n__location({index+1}) u8 pd;\n'
    source += 'void main() { lcdc=0; result[15]=0; pi=128;\n'
    source += 'for(i=0;i<64;++i) { pd=0x33; }\n'
    source += f'color=0x12AB; palette={slot//4}; entry={slot%4};\n'
    if mode == 'color':
        call = f'cgb_{channel}_color(palette,entry,color)'
        value = 0x12AB
    elif mode == 'rgb':
        call = f'cgb_{channel}_rgb(palette,entry,11,21,4)'
        value = 11 | (21 << 5) | (4 << 10)
    elif mode == 'masked':
        call = f'cgb_{channel}_color({slot//4+8},{slot%4+4},color)'
        value = 0x12AB
    else:
        call = f'cgb_{channel}_colors(palette,entry,&color,1)'
        value = 0x12AB
    source += call + ';\n'
    for n, offset in enumerate(range(-2, 4)):
        source += f'pi={(slot*2+offset)%64}; result[{n}]=pd;\n'
    source += 'result[15]=0xA5; while(1) { } }\n'
    return source, [0x33, 0x33, value & 255, value >> 8, 0x33, 0x33]


def run(command, directory, log):
    """Bound each process and preserve diagnostics without locale conversion."""
    result = subprocess.run(command, cwd=directory, capture_output=True, timeout=30,
                            creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
    log.write_bytes(result.stdout + result.stderr)
    return result.returncode


def main():
    """Use separate ROMs for first/last slots, RGB packing and the bulk control."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    lib = Path(__file__).resolve().parents[1] / 'lib'
    args.output_parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='gb-cgb-palette-', dir=args.output_parent.resolve()))
    record = dict(compiler=str(compiler), compiler_sha256=digest(compiler),
                  emulator=str(emulator), emulator_sha256=digest(emulator),
                  library_sha256={p.name: digest(p) for p in (lib/'cgb_palette.c', lib/'cgb_palette.h')},
                  scope='CGB palette RAM readback with LCD off; no display or physical hardware claim.', cases=[])
    for channel in ('bg', 'obj'):
        for slot, mode in ((0, 'color'), (31, 'color'), (13, 'rgb'), (31, 'masked'), (31, 'bulk')):
            directory = root / f'{channel}-{slot}-{mode}'
            directory.mkdir()
            source, expected = fixture(channel, slot, mode)
            (directory/'case.c').write_text(source, encoding='ascii')
            rom = directory/'case.gbc'
            command = [str(compiler), str(directory/'case.c'), str(lib/'cgb_palette.c'),
                       '-I', str(lib), '--cgb=cgb', '--profile=dev', '--rst-disable',
                       '--stack-bank=fixed', '--no-cache', '--no-disasm', '-o', str(rom)]
            code = run(command, directory, directory/'compile.log')
            case = dict(name=directory.name, command=command, compile_exit=code,
                        source_sha256=digest(directory/'case.c'), expected=expected, passed=False)
            if code == 0:
                runtime = directory/'runtime.json'
                command = [str(emulator), str(rom), '--hardware', 'cgb', '--run-frames', '4',
                           '--watch-window', 'result:0xC700:16', '--dump-report', str(runtime)]
                code = run(command, directory, directory/'run.log')
                case.update(run_command=command, run_exit=code, rom_sha256=digest(rom))
                if code == 0:
                    watches = [w for w in json.loads(runtime.read_text(encoding='utf-8'))['watched_memory']
                               if w['name'] == 'result']
                    assert len(watches) == 1 and watches[0]['addr'] == 0xC700 and watches[0]['size'] == 16
                    actual = watches[0]['preview_bytes']
                    case.update(actual=actual, runtime_sha256=digest(runtime),
                                passed=actual[:6] == expected and actual[15] == 165)
            record['cases'].append(case)
            (root/'report.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    failures = [c['name'] for c in record['cases'] if not c['passed']]
    print(json.dumps(dict(report=str(root/'report.json'), checks=len(record['cases']), failures=failures)))
    return 1 if failures else 0


if __name__ == '__main__':
    raise SystemExit(main())
