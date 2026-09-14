"""Check GB OAM DMA completion, source-page selection and interrupt state."""
from pathlib import Path
import argparse,hashlib,json,subprocess,sys
BASE=Path(__file__).resolve().parents[1];REPOS=BASE.parent
import tempfile
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output-parent',type=Path,required=True)
parser.add_argument('--emulator',type=Path,required=True)
parser.add_argument('--compiler',type=Path,default=REPOS/'kitaqgb/kitaqgb.exe')
args=parser.parse_args();compiler=args.compiler.resolve(strict=True);emulator=args.emulator.resolve(strict=True)
args.output_parent.mkdir(parents=True,exist_ok=True)
out=Path(tempfile.mkdtemp(prefix='gb-oam-dma-',dir=args.output_parent)).resolve()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
body='''void __oam_dma(u16 src_ptr);
void __wait_vblank();
__location(0xC400) u8 dma_source[160];
__location(0xC700) u8 result[16];
u16 requested_source(){result[2]++;return 0xC480;}
void main(){
    u8 i;u8 iteration;
    __asm { DI }

    for(i=0;i<160;i++){dma_source[i]=(u8)(i+0x31);}
    result[0]=0;result[1]=0;result[2]=0;
    *(u8*)0xFF0F=0;*(u8*)0xFFFF=4;
    __wait_vblank();
    INTERRUPT_SETUP
    __oam_dma(requested_source());
    result[0]=0xA5;
    result[3]=*(u8*)0xFFFF;
    result[15]=0x5A;while(1){}
}
'''
records=[]
# Exercise low-byte truncation, repeated initialization and both source buses.
for variant in ['wram-unaligned','wram-aligned','wram-repeated','rom-aligned']:
    source_body=body;expected_calls=1
    if variant=='wram-aligned':source_body=body.replace('0xC480','0xC400')
    if variant=='wram-repeated':
        source_body=body.replace('__oam_dma(requested_source());','for(iteration=0;iteration<32;iteration++){__oam_dma(requested_source());}');expected_calls=32
    if variant=='rom-aligned':
        values=','.join(str((i+0x31)&255) for i in range(160))
        source_body=body.replace('__location(0xC400) u8 dma_source[160];','__aligned(256) __prg_rom u8 dma_source[160]={'+values+'};')
        source_body=source_body.replace('return 0xC480;','return (u16)dma_source;').replace('    for(i=0;i<160;i++){dma_source[i]=(u8)(i+0x31);}','')
    for mode in ['dmg','cgb']:
        for interrupts in [False,True]:
            folder=out/(variant+'-'+mode+('-ime-on' if interrupts else '-ime-off'));folder.mkdir(exist_ok=True)
            src=folder/'case.c';src.write_text(source_body.replace('INTERRUPT_SETUP','__asm {\n EI\n NOP\n }' if interrupts else ''),encoding='ascii')
            rom=folder/'case.gb';rom.unlink(missing_ok=True)
            command=[str(compiler),str(src),'-I',str(REPOS/'kitaqgb/lib'),'--no-cache','--no-disasm','--cgb=cgb','--rst-disable','--stack-bank=fixed','-o',str(rom)]
            run=subprocess.run(command,cwd=folder,capture_output=True,timeout=90);(folder/'build.log').write_bytes(run.stdout+run.stderr)
            row=dict(variant=variant,hardware=mode,interrupts=interrupts,build_exit=run.returncode,build_command=command,source_sha256=sha(src),compiler_sha256=sha(compiler),passed=False)
            if run.returncode==0:
                report=folder/'runtime.json'
                command=[str(emulator),str(rom),'--hardware',mode,'--run-frames','12','--run-until','frame=12&&ly=144','--watch-window','result:0xC700:16','--dump-report',str(report)]+sum([['--watch-window',f'oam{n}:0x{0xFE00+16*n:04X}:16'] for n in range(10)],[])
                run=subprocess.run(command,cwd=folder,capture_output=True,timeout=60);(folder/'run.log').write_bytes(run.stdout+run.stderr)
                row.update(run_exit=run.returncode,run_command=command,emulator_sha256=sha(emulator),rom_sha256=sha(rom))
                if run.returncode==0:
                    state=json.loads(report.read_text(encoding='utf-8'));state={k:state[k] for k in ['meta','cpu','watched_memory','unsupported_opcodes','stop_reason'] if k in state}
                    report.write_text(json.dumps(state,indent=2),encoding='utf-8')
                    actual=next(w['preview_bytes'] for w in state['watched_memory'] if w['name']=='result')
                    oam=sum([next(w['preview_bytes'] for w in state['watched_memory'] if w['name']==f'oam{n}') for n in range(10)],[])
                    row.update(actual=actual,oam=oam,cpu=state['cpu'],frames=state['meta']['frames_executed'])
                    row['passed']=actual[0:4]==[0xA5,0,expected_calls,4] and actual[15]==0x5A and row['frames']==11 and state['cpu']['ime']==interrupts and oam==[(i+0x31)&255 for i in range(160)]

            records.append(row);print(mode,interrupts,'PASS' if row['passed'] else 'FAIL',row.get('actual'),flush=True)
            (out/'report.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
raise SystemExit(0 if all(r['passed'] for r in records) else 1)
