"""Compare signed/unsigned byte and word right shifts in byte-valued expressions.

Generated inputs are original. A side-effecting read function checks evaluation
count; known RAM watches verify shifted bytes and a preserved local guard. These
are KOKURA regression tests, not physical hardware certification.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path


def digest(path):
    """Bind the report to the exact source, executable and runtime evidence."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fixture(ctype, value, narrow=False):
    """Read each mutable input once per shift, exercising both direct and call operands."""
    width = 8 if ctype.endswith('8') else 16
    bits = value & ((1 << width) - 1)
    interpreted = bits - (1 << width) if ctype.startswith('s') and bits & (1 << (width - 1)) else bits
    counts = (0, 1, 3, 7)
    text = f'''// Original signedness and byte-result shift fixture.
__location(0xC700) u8 result[16];
{ctype} input;
u8 calls;
{ctype} read_input() {{ calls++; return input; }}
void test() {{
    u8 guard=0x6D;
'''
    for i, count in enumerate(counts):
        direct,call = f'input >> {count}',f'read_input() >> {count}'
        if narrow:
            direct,call = f'(u8)({direct})',f'(u8)({call})'
        text += f'    result[{i}]={direct};\n    result[{i+4}]={call};\n'
    text += '''    result[8]=calls; result[9]=guard; result[15]=0xA5;
}
void main() {
'''
    text += f'    result[15]=0; input={value}; calls=0; test(); while (1) {{ }}\n}}\n'
    expected = [(interpreted >> count) & 255 for count in counts]
    return text, expected + expected + [4, 0x6D]


def run(command, directory, log):
    """Retain raw process diagnostics and bound each compiler/emulator run."""
    p = subprocess.run(command, cwd=directory, capture_output=True, timeout=30)
    log.write_bytes(p.stdout+p.stderr)
    return p.returncode


def main():
    """Record every independent type/input case, including compilation failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    parser.add_argument('--context', choices=('implicit','cast','both'), default='both')
    args = parser.parse_args()
    compiler,emulator = args.compiler.resolve(strict=True),args.emulator.resolve(strict=True)
    args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-right-shifts-',dir=args.output_parent.resolve()))
    record = dict(compiler=str(compiler),compiler_sha256=digest(compiler),emulator=str(emulator),emulator_sha256=digest(emulator),cases=[])
    contexts = (False,True) if args.context=='both' else (args.context=='cast',)
    for ctype,narrow in ((ctype,narrow) for ctype in ('s8','u8','s16','u16') for narrow in contexts):
        for value in (-128,-5,-1,0,1,63,127):
            name=ctype+'-'+str(value)+('-cast' if narrow else '-implicit');directory=output/name;directory.mkdir()
            text,expected=fixture(ctype,value,narrow);source=directory/'case.c';source.write_text(text,encoding='ascii');rom=directory/'case.gb'
            command=[str(compiler),str(source),'--cgb=dmg','--profile=dev','--rst-disable','--stack-bank=fixed','--no-cache','--no-disasm','-o',str(rom)]
            compile_exit=run(command,directory,directory/'compile.log')
            case=dict(name=name,source_sha256=digest(source),expected=expected,command=command,compile_exit=compile_exit,passed=False)
            if compile_exit==0:
                runtime=directory/'runtime.json'
                run_command=[str(emulator),str(rom),'--hardware','dmg','--run-frames','4','--watch-window','result:0xC700:16','--dump-report',str(runtime)]
                run_exit=run(run_command,directory,directory/'run.log')
                case.update(run_command=run_command,run_exit=run_exit,rom_sha256=digest(rom))
                if run_exit==0:
                    watches=[w for w in json.loads(runtime.read_text())['watched_memory'] if w['name']=='result']
                    assert len(watches)==1 and watches[0]['addr']==0xC700 and watches[0]['size']==16
                    actual=watches[0]['preview_bytes']
                    case.update(actual=actual,passed=actual[:10]==expected and actual[15]==165,runtime_sha256=digest(runtime))
            record['cases'].append(case)
            (output/'report.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    failures=[c['name'] for c in record['cases'] if not c['passed']]
    print(json.dumps(dict(report=str(output/'report.json'),cases=len(record['cases']),failures=failures)))
    if failures:raise SystemExit(1)


if __name__=='__main__':main()
