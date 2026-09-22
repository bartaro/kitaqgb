"""Check normalized word-valued logical expressions and short-circuit side effects."""
from pathlib import Path
import argparse,hashlib,json,subprocess
def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--compiler',type=Path,default=Path(__file__).resolve().parents[1]/'kitaqgb.exe');ap.add_argument('--emulator',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args()
    folder=a.output.resolve();folder.mkdir(parents=True,exist_ok=True);compiler=a.compiler.resolve();emu=a.emulator.resolve();body=[];want=[]
    def add(setup,expression,value):
        body.append(setup+f'result[{len(want)}]=({expression});');want.append(value)
    for x,y,pairs in [('x','y',[(0,0),(0,1),(1,0),(256,0),(0,256),(256,65535)]),('a','b',[(0,0),(0,128),(128,0),(2,0),(0,2),(2,128)])]:
        for left,right in pairs:
            for expression,value in [(x+'&&'+y,int(bool(left) and bool(right))),(x+'||'+y,int(bool(left) or bool(right))),('!'+x,int(not left)),('!'+y,int(not right))]:add(f'{x}={left};{y}={right};',expression,value)
    for setup,expression,value,calls in [('x=0;','x&&probe(1)',0,0),('x=2;','x&&probe(0)',0,1),('x=0;','x||probe(7)',1,1),('x=256;','x||probe(0)',1,0),('','probe(0)&&probe(3)',0,1),('','probe(7)||probe(0)',1,1),('','!probe(0)',1,1),('','!!probe(256)',1,1)]:
        add(setup+'calls=0;',expression,value);add('','calls',calls)
    for expression,value in [('word_logic()',1),('identity(x||y)',1),('500+(x&&y)',501),('(x&&y)*1000',1000),('(x&&y)||(a&&!b)',1),('!(x&&y)',0),('!(x||y)',0)]:add('x=256;y=65535;a=0;b=2;',expression,value)
    assert len(want)==71
    expected=want+[0]*(79-len(want))+[0xA55A]
    source=folder/'case.c';source.write_text('// Original logical-expression regression; no external assets.\n__location(0xC600) u16 result[80];\nu16 x;u16 y;u8 a;u8 b;u8 calls;\nu16 probe(u16 value){calls++;return value;}\nu16 identity(u16 value){return value;}\nu16 word_logic(){return x&&y;}\nvoid main(){'+''.join(body)+'result[79]=0xA55A;while(1){}}',encoding='utf-8')
    rom=folder/'case.gb';build=subprocess.run([str(compiler),str(source),'-o',str(rom),'--profile=dev','--rst-disable','--stack-bank=fixed','--cgb=cgb','--no-cache','--no-disasm'],cwd=folder,capture_output=True,timeout=120);(folder/'build.txt').write_bytes(build.stdout+build.stderr)
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();report={'compiler_sha256':sha(compiler),'source_sha256':sha(source),'build_exit':build.returncode,'assertions_per_mode':len(want),'records':[]}
    if build.returncode==0:
        report['rom_sha256']=sha(rom);report['emulator_sha256']=sha(emu)
        for mode in ['dmg','cgb']:
            out=folder/(mode+'.json');args=[str(emu),str(rom),'--hardware',mode,'--run-frames','20','--dump-report',str(out),'--report-sections','meta,watched_memory','--watch-fields','preview']
            for n in range(0,160,16):args+=['--watch-window',f'r{n}:{0xC600+n}:16']
            run=subprocess.run(args,cwd=folder,capture_output=True,timeout=120);assert run.returncode==0
            watches={r['name']:r['preview_bytes'] for r in json.loads(out.read_text(encoding='utf-8'))['watched_memory']};raw=sum((watches['r'+str(n)] for n in range(0,160,16)),[]);actual=[raw[n]+256*raw[n+1] for n in range(0,160,2)];row={'mode':mode,'actual':actual,'expected':expected,'passed':actual==expected};report['records'].append(row)
            print(mode,'PASS' if row['passed'] else 'FAIL',[(i,a,b) for i,(a,b) in enumerate(zip(actual,expected)) if a!=b],flush=True)
    (folder/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    raise SystemExit(0 if build.returncode==0 and all(r['passed'] for r in report['records']) else 1)
if __name__=='__main__':main()
