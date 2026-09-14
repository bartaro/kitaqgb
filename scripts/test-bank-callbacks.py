"""Exercise real far callbacks and bank-qualified library reads in KOKURA."""
from pathlib import Path
import argparse, hashlib, json, subprocess, tempfile

REPO = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--compiler', type=Path, default=REPO/'kitaqgb.exe')
parser.add_argument('--emulator', type=Path, required=True)
parser.add_argument('--output-parent', type=Path)
parser.add_argument('--hardware', choices=['dmg','cgb'], default='dmg')
args = parser.parse_args()
compiler = args.compiler.resolve(strict=True)
emulator = args.emulator.resolve(strict=True)
if args.output_parent: args.output_parent.mkdir(parents=True,exist_ok=True)
output = Path(tempfile.mkdtemp(prefix='gb-bank-callbacks-',dir=args.output_parent)).resolve()

PREFIX = '''#pragma fixed_bank 0
#include "bank.c"
__location(0xC700) u8 result[16];
u8 chosen_bank; const void* chosen_func;
#pragma fixed_bank 1
__prg_rom u8 home1[2]={0xA1,0x11};
#pragma fixed_bank 2
__prg_rom u8 home2[2]={0xB2,0x22};
#pragma fixed_bank 3
__prg_rom u8 payload[4]={0xD3,0x7A,0x5C,0x96};
void callback() { u8 local; local=0x39; result[0]++; result[1]=local; }
#pragma fixed_bank 0
'''

def case(name, declarations, body, expected):
    """Keep completion and restored mapping checks common to every fixture."""
    source = PREFIX + declarations + '\n#pragma fixed_bank 0\nvoid main() {\n'
    source += 'bank_switch(2); result[0]=0; result[1]=0;\n' + body
    source += '\nresult[12]=home2[0]; result[13]=*(u8*)0xFF82; result[14]=bank_get_current();'
    source += '\nresult[15]=0xA5; while(1){}\n}\n'
    return name, source, dict(expected) | {12:0xB2,13:2,14:2,15:0xA5}

cases = [
    case('pointer-literal', '', '__farcall_ptr(3,callback);', {0:1,1:0x39}),
    case('pointer-runtime', '', 'chosen_bank=3; chosen_func=callback; __farcall_ptr(chosen_bank,chosen_func);', {0:1,1:0x39}),
    case('library-wrapper', '', 'far_call(3,callback);', {0:1,1:0x39}),
    case('caller-local', 'void invoke() { u8 guard; guard=0x6D; far_call(3,callback); result[2]=guard; }', 'invoke();', {0:1,1:0x39,2:0x6D}),
    case('stack-caller', 'void __stackcall invoke(u8 bank,const void* pointer) { u8 guard; guard=0x6D; __farcall_ptr(bank,pointer); result[2]=guard; }', 'invoke(3,callback);', {0:1,1:0x39,2:0x6D}),
    case('stack-target', '#pragma fixed_bank 3\nvoid __stackcall stack_target() { result[0]++; result[1]=0x39; }', '__farcall_ptr(3,stack_target);', {0:1,1:0x39}),
    case('repeated', '', 'u8 i; for(i=0;i<100;i++) { far_call(3,callback); }', {0:100,1:0x39}),
    case('side-effects', 'u8 get_bank() { result[2]++; return 3; }\nconst void* get_pointer() { result[3]++; return callback; }', 'result[2]=0;result[3]=0;__farcall_ptr(get_bank(),get_pointer());', {0:1,1:0x39,2:1,3:1}),
    case('nested', '''#pragma fixed_bank 1
void inner() { result[0]++; result[1]=home1[0]; }
#pragma fixed_bank 3
void outer() { far_call(1,inner); result[2]=payload[0]; result[3]=*(u8*)0xFF82; }
''', 'far_call(3,outer);', {0:1,1:0xA1,2:0xD3,3:3}),
    case('banked-caller', '''#pragma fixed_bank 2
void invoke() { far_call(3,callback); result[2]=home2[0]; result[3]=*(u8*)0xFF82; }
''', 'invoke();', {0:1,1:0x39,2:0xB2,3:2}),
    case('fixed-target', 'void fixed_target() { result[0]++; result[1]=0x39; }', 'far_call(0,fixed_target);', {0:1,1:0x39}),
    case('named-farcall', '', '__farcall(3,callback);', {0:1,1:0x39}),
    case('named-word-return', '#pragma fixed_bank 3\nu16 word_callback() { return 0xB739; }', 'u16 value;value=__farcall(3,word_callback);result[0]=(u8)value;result[1]=(u8)(value>>8);', {0:0x39,1:0xB7}),
    case('data-wrappers', '', '''u16 word; u8 bytes[4]; BankPtr pointer;
result[0]=far_data_read8(3,payload); word=far_data_read16(3,payload+1);
result[1]=(u8)word;result[2]=(u8)(word>>8);
far_data_read(3,payload,bytes,4);result[3]=bytes[3];
farptr_make(&pointer,3,payload+1);farptr_make(0,1,home1);
result[4]=pointer.bank;result[5]=farptr_read8(pointer);word=farptr_read16(pointer);
result[6]=(u8)word;result[7]=(u8)(word>>8);farptr_read(pointer,bytes,3);result[8]=bytes[2];
bytes[0]=0xE7;far_data_read(3,payload,bytes,0);result[9]=bytes[0];
''', {0:0xD3,1:0x7A,2:0x5C,3:0x96,4:3,5:0x7A,6:0x7A,7:0x5C,8:0x96,9:0xE7}),
]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

records = []
for name, source, expected in cases:
    folder=output/name; folder.mkdir(exist_ok=True)
    src=folder/'case.c'; src.write_text(source,encoding='ascii')
    rom=folder/'case.gb'; rom.unlink(missing_ok=True)
    command=[str(compiler),str(src),'-I',str(REPO/'lib'),'--no-cache','--no-disasm',
             '--cgb=cgb','--profile=dev','--rst-disable','--stack-bank=fixed',
             '--cart=mbc5','--romsize=64k','-o',str(rom)]
    run=subprocess.run(command,cwd=folder,capture_output=True,timeout=60)
    (folder/'build.log').write_bytes(run.stdout+run.stderr)
    row=dict(name=name,build_exit=run.returncode,command=command,expected=expected,
             source_sha256=digest(src),passed=False)
    if run.returncode==0:
        runtime=folder/'runtime.json'; runtime.unlink(missing_ok=True)
        command=[str(emulator),str(rom),'--hardware',args.hardware,'--run-frames','12',
                 '--watch-window','result:0xC700:16','--dump-report',str(runtime)]
        run=subprocess.run(command,cwd=folder,capture_output=True,timeout=60)
        (folder/'run.log').write_bytes(run.stdout+run.stderr)
        row.update(run_exit=run.returncode,run_command=command,rom_sha256=digest(rom))
        if run.returncode==0:
            state=json.loads(runtime.read_text(encoding='utf-8'))
            state={k:state[k] for k in ('meta','cpu','watched_memory','unsupported_opcodes','stop_reason') if k in state}
            runtime.write_text(json.dumps(state,indent=2),encoding='utf-8')
            watched=[w for w in state['watched_memory'] if w['name']=='result']
            assert len(watched)==1 and watched[0]['addr']==0xC700 and watched[0]['size']==16
            actual=watched[0]['preview_bytes']
            row.update(actual=actual,frames=state['meta']['frames_executed'])
            row['passed']=row['frames']==12 and all(actual[k]==v for k,v in expected.items())
    records.append(row)
    print(name,'PASS' if row['passed'] else 'FAIL',row.get('actual'),flush=True)
    report=dict(hardware=args.hardware,compiler_sha256=digest(compiler),emulator_sha256=digest(emulator),
                library_sha256={p.name:digest(p) for p in (REPO/'lib/bank.c',REPO/'lib/bank.h')},cases=records)
    (output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
# Reject statically typed callbacks that require arguments the intrinsic cannot pass.
for name,declaration,target,intrinsic,diagnostic_text in [
    ('parameter-required','void target(u8 value) { result[0]=value; }','target','__farcall_ptr','callback with no parameters'),
    ('typed-pointer-parameter','void target(u8 value) { result[0]=value; }\ntypedef void (*Callback)(u8); Callback pointer;','pointer','__farcall_ptr','callback with no parameters'),
    ('named-parameter','void target(u8 value) { result[0]=value; }','target','__farcall','callback with no parameters'),
    ('named-variable','u16 pointer;','pointer','__farcall','declared function name'),
]:
    folder=output/name;folder.mkdir()
    src=folder/'case.c';rom=folder/'case.gb'
    src.write_text(PREFIX+declaration+'\nvoid main() { '+intrinsic+'(3,'+target+'); while(1){} }\n',encoding='ascii')
    command=[str(compiler),str(src),'-I',str(REPO/'lib'),'--no-cache','--no-disasm','-o',str(rom)]
    run=subprocess.run(command,cwd=folder,capture_output=True,timeout=60)
    diagnostic=run.stdout+run.stderr;(folder/'build.log').write_bytes(diagnostic)
    passed=run.returncode!=0 and diagnostic_text.encode() in diagnostic and not rom.exists()
    records.append(dict(name=name,passed=passed,build_exit=run.returncode,command=command,source_sha256=digest(src)))
    print(name,'PASS' if passed else 'FAIL',flush=True)
report['cases']=records
(output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('Report:',output/'report.json',flush=True)
if not all(r['passed'] for r in records): raise SystemExit(1)
