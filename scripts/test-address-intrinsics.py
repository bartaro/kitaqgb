"""Test GB map indices and bank-qualified byte/word reads with original ROM data.

Select KITAQGB and KOKURA executables with --compiler and --emulator.
All source, ROM and runtime evidence is retained under --output-parent or a
temporary directory. This validates emulator behavior, not physical hardware.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def digest(path):
    """Associate results with exact source and executable bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command,directory,log):
    """Bound compiler/emulator execution and retain byte-safe diagnostics."""
    try:
        process=subprocess.run(command,cwd=directory,capture_output=True,timeout=40)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b'')+(error.stderr or b'')); raise
    log.write_bytes(process.stdout+process.stderr)
    return process.returncode


def map_cases():
    """Exercise constant, general and power-of-two paths with independent expected values."""
    header='''// Original map-index fixture.
__location(0xC700) u8 result[8];
u8 x; u8 y; u8 width; u16 wide_x; u16 answer;
u16 __map_index(u8 x, u8 y, u8 width);
u8 read_x() { result[2]++; return x; }
u8 read_y() { result[2]++; return y; }
u8 read_width() { result[2]++; return width; }
void main() { result[7]=0; result[2]=0; x=2; y=3; width=5; wide_x=258;
'''
    cases=[
        ('map-constant','', '__map_index(2,3,5)',17,0),
        ('map-general','', '__map_index(x,y,width)',17,0),
        ('map-power-two','', '__map_index(x,y,8)',26,0),
        ('map-power-two-word-x','', '__map_index((u8)(wide_x+1),y,8)',27,0),
        ('map-general-calls','', '__map_index(read_x(),read_y(),read_width())',17,3),
        ('map-power-two-calls','', '__map_index(read_x(),read_y(),8)',26,2),
        ('map-zero-width','width=0;', '__map_index(x,y,width)',2,0),
        ('map-max-product','x=255; y=255; width=255;', '__map_index(x,y,width)',65280,0),
        ('map-power-two-zero-x','', '__map_index(0,y,8)',24,0),
        ('map-byte-width-truncation','', '__map_index(x,y,256)',2,0),
    ]
    for name,setup,expression,expected,calls in cases:
        source=header+setup+'\nanswer='+expression+';\nresult[0]=(u8)answer; result[1]=(u8)(answer>>8);\n'
        source+='result[7]=0xA5; while(1) { }\n}\n'
        yield name,source,{0:expected&255,1:expected>>8,2:calls,7:165}


def rectangle_cases():
    """Check both constant-folded outcomes against runtime half-open bounds."""
    cases=[('inside',(10,20,10,20,3,2)),('right',(13,20,10,20,3,2)),
           ('bottom',(10,22,10,20,3,2)),('left',(9,20,10,20,3,2)),
           ('empty-width',(10,20,10,20,0,2)),('empty-height',(10,20,10,20,3,0)),
           ('high-coordinate',(255,1,250,0,10,2)),('no-wrap',(0,1,250,0,10,2))]
    for name,values in cases:
        x,y,rx,ry,rw,rh=values
        answer=int(x>=rx and y>=ry and x-rx<rw and y-ry<rh)
        setup=''.join(f'{n}={v};' for n,v in zip(['x','y','rx','ry','rw','rh'],values))
        source='''// Half-open bounds: literals and RAM values must agree.
__location(0xC700) u8 result[8];
u8 x;u8 y;u8 rx;u8 ry;u8 rw;u8 rh;
u8 __xy_in_rect(u8 x,u8 y,u8 rx,u8 ry,u8 rw,u8 rh);
void main(){'''+setup+'result[0]=__xy_in_rect('+','.join(map(str,values))+');'
        source+='result[1]=__xy_in_rect(x,y,rx,ry,rw,rh);result[7]=0xA5;while(1){}}'
        yield 'rectangle-'+name,source,{0:answer,1:answer,7:165}


def far_cases():
    """Read distinct banked data and check a normal bank-one read before and after the helper."""
    for bank in (0,1,2):
        for word in (False,True):
            intrinsic='__farpeek16' if word else '__farpeek8'
            return_type='u16' if word else 'u8'
            source=f'''// Original far-read fixture; home provides a bank-restoration observation.
#pragma fixed_bank 1
const u8 home[2]={{0x39,0x6C}};
#pragma fixed_bank {bank}
const u8 payload[4]={{0xA7,0x5C,0xD2,0x81}};
#pragma fixed_bank 0
__location(0xC700) u8 result[8];
u8 requested_bank; const u8* requested_address; u16 answer;
void __bankswitch(u8 bank);
{return_type} {intrinsic}(u8 bank,const void* address);
u16 read_value() {{
    u8 guard; u16 value;
    guard=0x6D;
    value={intrinsic}(requested_bank,requested_address);
    result[4]=guard;
    return value;
}}
void main() {{
    result[7]=0; __bankswitch(1);
    requested_bank={bank}; requested_address=payload+1;
    result[2]=home[0];
    answer=read_value();
    result[0]=(u8)answer; result[1]=(u8)(answer>>8);
    result[3]=home[0]; result[7]=0xA5;
    while(1) {{ }}
}}
'''
            yield f'far{16 if word else 8}-bank-{bank}',source,{0:0x5C,1:0xD2 if word else 0,2:0x39,3:0x39,4:0x6D,7:165}


def main():
    """Run each generated case and keep successful evidence even if another case fails."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler',required=True,type=Path)
    parser.add_argument('--emulator',required=True,type=Path)
    parser.add_argument('--output-parent',type=Path)
    args=parser.parse_args(); compiler=args.compiler.resolve(strict=True); emulator=args.emulator.resolve(strict=True)
    if args.output_parent: args.output_parent.mkdir(parents=True,exist_ok=True)
    output=Path(tempfile.mkdtemp(prefix='gb-address-intrinsics-',dir=args.output_parent)).resolve()
    report=dict(compiler=str(compiler),compiler_sha256=digest(compiler),emulator=str(emulator),
        emulator_sha256=digest(emulator),cases=[],scope='Complete compiler CLI and KOKURA DMG RAM watches; no hardware.')
    for name,text,expected in [*map_cases(),*rectangle_cases(),*far_cases()]:
        directory=output/name; directory.mkdir(); source=directory/'case.c'; rom=directory/'case.gb'
        source.write_text(text,encoding='ascii')
        command=[str(compiler),str(source),'--cgb=dmg','--profile=dev','--rst-disable',
            '--stack-bank=fixed','--no-cache','--no-disasm','-o',str(rom)]
        rc=run(command,directory,directory/'compile.log')
        case=dict(name=name,expected=expected,passed=False,compile_exit=rc,command=command,source_sha256=digest(source))
        if rc==0:
            runtime=directory/'runtime.json'
            run_command=[str(emulator),str(rom),'--hardware','dmg','--run-frames','4',
                '--watch-window','result:0xC700:8','--dump-report',str(runtime)]
            run_exit=run(run_command,directory,directory/'run.log');case['run_exit']=run_exit
            if run_exit==0:
                state=json.loads(runtime.read_text(encoding='utf-8'))
                watches=[w for w in state['watched_memory'] if w['name']=='result']
                if len(watches)!=1 or watches[0]['addr']!=0xC700 or watches[0]['size']!=8:
                    raise RuntimeError('Invalid result watch: '+name)
                actual=watches[0]['preview_bytes']
                case.update(actual=actual,passed=all(actual[index]==value for index,value in expected.items()),
                    rom_sha256=digest(rom),runtime_sha256=digest(runtime),run_command=run_command)
        report['cases'].append(case)
        (output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    failures=[c['name'] for c in report['cases'] if not c['passed']]
    print(json.dumps(dict(report=str(output/'report.json'),cases=len(report['cases']),failures=failures)))
    if failures: raise SystemExit(1)


if __name__=='__main__': main()
