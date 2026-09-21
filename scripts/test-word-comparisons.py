"""Exercise comparison results in word stores, returns, arguments and arithmetic."""
from pathlib import Path
import argparse,subprocess,json,hashlib
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--compiler',type=Path,default=Path(__file__).resolve().parents[1]/'kitaqgb.exe');p.add_argument('--emulator',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);compiler=a.compiler.resolve();emulator=a.emulator.resolve();expected=[];body=[]
    for x,y,pairs in [('x8','y8',[(0,0),(0,255),(255,0)]),('sx8','sy8',[(0,0),(-128,127),(127,-128)]),('x16','y16',[(0,0),(255,256),(65535,0)]),('sx16','sy16',[(0,0),(-32768,32767),(32767,-32768)])]:
        for l,r in pairs:
            body.append(f'{x}={l};{y}={r};')
            for op,value in [('==',l==r),('!=',l!=r),('<',l<r),('<=',l<=r),('>',l>r),('>=',l>=r)]:
                body.append(f'result[{len(expected)}]=({x}{op}{y});');expected.append(int(value))
    extras=[('x8=8;', '(x8&8)!=0',1),('x8=0;', '(x8&8)!=0',0),('calls=0;', 'tick()!=tick()',1),('', 'calls',2),('x16=256;y16=255;', 'word_compare()',1),('', 'identity(x16>y16)',1),('', '500+(x16>y16)',501)]
    for setup,expr,value in extras:body.append(setup+f'result[{len(expected)}]={expr};');expected.append(value)
    expected.append(0xA55A)
    source=out/'case.c';source.write_text('''// Original comparison regression: no external assets.
__location(0xC600) u16 result[80];
u8 x8;u8 y8;s8 sx8;s8 sy8;u16 x16;u16 y16;s16 sx16;s16 sy16;u8 calls;
u8 tick(){calls++;return calls;}
u16 word_compare(){return x16>y16;}
u16 identity(u16 value){return value;}
void main(){'''+''.join(body)+'result[79]=0xA55A;while(1){}}',encoding='utf-8')
    rom=out/'case.gb';build=subprocess.run([str(compiler),str(source),'-o',str(rom),'--cgb=cgb','--profile=dev','--rst-disable','--stack-bank=fixed','--no-cache','--no-disasm'],cwd=out,capture_output=True,timeout=120)
    (out/'build.txt').write_bytes(build.stdout+build.stderr);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();report={'compiler_sha256':sha(compiler),'source_sha256':sha(source),'build_exit':build.returncode,'records':[]}
    if build.returncode==0:
        report['rom_sha256']=sha(rom);report['emulator_sha256']=sha(emulator)
        for mode in ['dmg','cgb']:
            state=out/(mode+'.json');args=[str(emulator),str(rom),'--hardware',mode,'--run-frames','20','--dump-report',str(state),'--report-sections','meta,watched_memory','--watch-fields','preview']
            for offset in range(0,160,16):args+=['--watch-window',f'r{offset}:{0xC600+offset}:16']
            run=subprocess.run(args,cwd=out,capture_output=True,timeout=120);assert run.returncode==0
            watches={w['name']:w['preview_bytes'] for w in json.loads(state.read_text(encoding='utf-8'))['watched_memory']};raw=sum((watches['r'+str(i)] for i in range(0,160,16)),[]);actual=[raw[i]+256*raw[i+1] for i in range(0,160,2)]
            row={'mode':mode,'actual':actual,'expected':expected,'passed':actual==expected};report['records'].append(row);print(mode,'PASS' if row['passed'] else 'FAIL',[(i,x,y) for i,(x,y) in enumerate(zip(actual,expected)) if x!=y],flush=True)
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    raise SystemExit(0 if build.returncode==0 and all(r['passed'] for r in report['records']) else 1)
if __name__=='__main__':main()
