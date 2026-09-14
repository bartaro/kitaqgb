"""Check GB memcpy/memset zero counts and size-specialization boundaries.

Supply --compiler and --emulator paths. Generated original ROMs and complete
KOKURA reports are retained under --output-parent or a temporary directory.
This verifies emulator RAM behavior; it does not certify physical hardware.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def digest(path):
    """Tie evidence to the exact source or generated artifact."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command,directory,log):
    """Run a bounded tool process and retain locale-independent diagnostic bytes."""
    try:
        process=subprocess.run(command,cwd=directory,capture_output=True,timeout=40)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b'')+(error.stderr or b''))
        raise
    log.write_bytes(process.stdout+process.stderr)
    return process.returncode


def fixture(fill,small,count,dynamic):
    """Observe copied/filled bytes and untouched boundary bytes after a completion marker."""
    name=('__memset' if fill else '__memcpy')+('_small' if small else '')
    count_type='u8' if small else 'u16'
    second='u8 value' if fill else 'const u8* source'
    value='0x5A' if fill else 'source'
    argument='read_count()' if dynamic else str(count)
    indices=(0,1,16,17,255,256,259)
    source=f'''// Original memory-count regression; no external assets.
__location(0xC400) u8 source[260];
__location(0xC800) u8 destination[260];
__location(0xC700) u8 result[8];
u16 i;
{count_type} requested;
{count_type} read_count() {{ return requested; }}
void {name}(u8* destination, {second}, {count_type} count);
void main() {{
result[7]=0; requested={count};
for(i=0;i<260;i++) {{ source[i]=(u8)(i+3); destination[i]=0xCC; }}
{name}(destination,{value},{argument});
'''
    source+='\n'.join(f'result[{n}]=destination[{index}];' for n,index in enumerate(indices))
    source+='\nresult[7]=0xA5; while(1) { }\n}\n'
    def expected_byte(index):
        return (0x5A if fill else (index+3)&255) if index<count else 0xCC
    return source,[expected_byte(i) for i in indices]+[165],[expected_byte(i) for i in range(8)]


def main():
    """Compare constant and runtime lengths through both byte- and word-count APIs."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler',required=True,type=Path)
    parser.add_argument('--emulator',required=True,type=Path)
    parser.add_argument('--output-parent',type=Path)
    args=parser.parse_args()
    compiler=args.compiler.resolve(strict=True); emulator=args.emulator.resolve(strict=True)
    if args.output_parent: args.output_parent.mkdir(parents=True,exist_ok=True)
    output=Path(tempfile.mkdtemp(prefix='gb-memory-counts-',dir=args.output_parent)).resolve()
    report=dict(compiler=str(compiler),compiler_sha256=digest(compiler),emulator=str(emulator),
        emulator_sha256=digest(emulator),cases=[],scope='Actual compiler CLI and KOKURA DMG memory watches; no hardware claim.')
    for fill in (False,True):
        for small in (False,True):
            for count in (0,1,17,255 if small else 256):
                for dynamic in (False,True):
                    name=('fill' if fill else 'copy')+('-byte' if small else '-word')+f'-{count}'+('-runtime' if dynamic else '-constant')
                    directory=output/name; directory.mkdir()
                    source=directory/'case.c'; rom=directory/'case.gb'
                    text,expected,prefix=fixture(fill,small,count,dynamic)
                    source.write_text(text,encoding='ascii')
                    command=[str(compiler),str(source),'--cgb=dmg','--profile=dev','--rst-disable',
                        '--stack-bank=fixed','--no-cache','--no-disasm','-o',str(rom)]
                    rc=run(command,directory,directory/'compile.log')
                    case=dict(name=name,fill=fill,small=small,count=count,dynamic=dynamic,expected=expected,
                        expected_prefix=prefix,command=command,compile_exit=rc,passed=False,source_sha256=digest(source))
                    if rc==0:
                        runtime=directory/'runtime.json'
                        run_command=[str(emulator),str(rom),'--hardware','dmg','--run-frames','4',
                            '--watch-window','result:0xC700:8','--watch-window','destination:0xC800:8','--dump-report',str(runtime)]
                        run_exit=run(run_command,directory,directory/'run.log')
                        case['run_exit']=run_exit
                        if run_exit==0:
                            state=json.loads(runtime.read_text(encoding='utf-8'))
                            watches={w['name']:w for w in state['watched_memory']}
                            if watches['result']['addr']!=0xC700 or watches['destination']['addr']!=0xC800:
                                raise RuntimeError('Invalid watch addresses: '+name)
                            actual=watches['result']['preview_bytes']; actual_prefix=watches['destination']['preview_bytes']
                            case.update(actual=actual,actual_prefix=actual_prefix,passed=actual==expected and actual_prefix==prefix,
                                rom_sha256=digest(rom),runtime_sha256=digest(runtime),run_command=run_command)
                    report['cases'].append(case)
                    (output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    failures=[c['name'] for c in report['cases'] if not c['passed']]
    print(json.dumps(dict(report=str(output/'report.json'),cases=len(report['cases']),failures=failures)))
    if failures: raise SystemExit(1)


if __name__=='__main__':
    main()
