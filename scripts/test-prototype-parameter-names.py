"""Verify definition-local parameter names independently of prototype spelling."""
from pathlib import Path
import argparse,hashlib,json,subprocess
SOURCE='''// Original forward-declaration regression; no external assets.
__location(0xC600) u16 result[16];
u8 oldname;u8 bytes[2];
u8 one(u8 declared);
u8 one(u8 renamed){return renamed+1;}
u16 many(u8 declared_a,u16 declared_b);
u16 many(u8 first,u16 second){return first+second;}
u8 bump(u8 before);
u8 bump(u8 after){u8 *address;address=&after;*address=*address+1;return after;}
u8 repeated(u8 first);
u8 repeated(u8 second);
u8 repeated(u8 third){return third*2;}
u8 repeated(u8 fourth);
u16 __stackcall stacker(u16 before);
u16 __stackcall stacker(u16 after){return after+2;}
u8 alias(u8 oldname);
u8 alias(u8 actual){return actual+oldname;}
u8 direct(u8 value){return value+3;}
u16 element(const u8 *old_source,u8 old_index);
u16 element(const u8 *new_source,u8 index){return new_source[index];}
void main(){oldname=7;bytes[0]=9;bytes[1]=200;
result[0]=one(255);result[1]=many(250,1000);result[2]=bump(255);result[3]=repeated(21);
result[4]=stacker(65535);result[5]=alias(10);result[6]=direct(4);result[7]=element(bytes,1);result[8]=one(0);
result[15]=0xA55A;while(1){}}
'''
def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--compiler',type=Path,default=Path(__file__).resolve().parents[1]/'kitaqgb.exe');ap.add_argument('--emulator',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args()
    base=a.output.resolve();base.mkdir(parents=True,exist_ok=True);compiler=a.compiler.resolve();emu=a.emulator.resolve();sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();records=[]
    expected=[0,1250,0,42,1,17,7,200,1]+[0]*6+[0xA55A]
    for variant,flags in [('default',[]),('unoptimized',['-O0']),('stack',['--abi=stack'])]:
        folder=base/variant;folder.mkdir(exist_ok=True);source=folder/'case.c';source.write_text(SOURCE,encoding='utf-8');rom=folder/'case.gb'
        p=subprocess.run([str(compiler),str(source),'-o',str(rom),'--profile=dev','--rst-disable','--stack-bank=fixed','--cgb=cgb','--no-cache','--no-disasm']+flags,cwd=folder,capture_output=True,timeout=120);(folder/'build.txt').write_bytes(p.stdout+p.stderr)
        if p.returncode:
            records.append({'variant':variant,'build_exit':p.returncode,'passed':False});continue
        for mode in ['dmg','cgb']:
            report=folder/(mode+'.json');cmd=[str(emu),str(rom),'--hardware',mode,'--run-frames','20','--dump-report',str(report),'--report-sections','meta,watched_memory','--watch-fields','preview','--watch-window','r0:50688:16','--watch-window','r16:50704:16']
            run=subprocess.run(cmd,cwd=folder,capture_output=True,timeout=120);assert run.returncode==0
            watches={w['name']:w['preview_bytes'] for w in json.loads(report.read_text(encoding='utf-8'))['watched_memory']};raw=watches['r0']+watches['r16'];actual=[raw[i]+256*raw[i+1] for i in range(0,32,2)]
            row={'variant':variant,'mode':mode,'build_exit':0,'source_sha256':sha(source),'rom_sha256':sha(rom),'actual':actual,'expected':expected,'passed':actual==expected};records.append(row);print(variant,mode,'PASS' if row['passed'] else 'FAIL',actual[:9],flush=True)
    data={'compiler_sha256':sha(compiler),'emulator_sha256':sha(emu),'records':records,'passed':all(r['passed'] for r in records)};(base/'report.json').write_text(json.dumps(data,indent=2),encoding='utf-8')
    raise SystemExit(0 if data['passed'] else 1)
if __name__=='__main__':main()
