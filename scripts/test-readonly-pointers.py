"""Check final ROM pointer bytes, aggregate padding and rejected nonconstant initializers."""
from pathlib import Path
import argparse,hashlib,json,subprocess,tempfile
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--compiler',type=Path,default=Path(__file__).resolve().parents[1]/'kitaqgb.exe')
parser.add_argument('--output-parent',type=Path)
args=parser.parse_args()
compiler=args.compiler.resolve(strict=True)
parent=args.output_parent.resolve() if args.output_parent else None
if parent:parent.mkdir(parents=True,exist_ok=True)
out=Path(tempfile.mkdtemp(prefix='gb-readonly-pointers-',dir=parent))
reports=[]
for platform in ['gb']:
    payload='''#pragma fixed_bank 2
__prg_rom u8 payload[8]={0xD3,0x6B,0xA7,0x5E,0xB9,0xC2,0x84,0x17};
__prg_rom u16 words[2]={0x3912,0x5F7A};
'''
    table='''typedef __packed struct { u8 mark[4]; const u8* p[6]; u16 tail; } Entry;
#pragma fixed_bank 3
__prg_rom Entry table[2]={{{0xBA,0x42,0x9D,0xE6},
    {payload,payload+2,&payload[3],(const u8*)words+1,(const u8*)&words[1],(payload+4)-1},0x7183}};
'''
    end='''#pragma fixed_bank 0
u8 copy[36];
void __farmemcpy(void* dst,u8 bank,const void* src,u16 len);
void main() { __farmemcpy(copy,3,table,36); while(1){} }
'''
    cases=[('forward',table+payload+end,True),('backward',payload+table+end,True),
        ('null-and-nested',payload+'''typedef __packed struct { const u8* refs[2]; } Pair;
typedef __packed struct { u8 mark[4]; Pair p[2]; u16 tail; } Entry;
#pragma fixed_bank 3
__prg_rom Entry table[2]={{{0xBA,0x42,0x9D,0xE6},{{{payload,0}},{{&payload[2]}}},0x7183}};
'''+end.replace('36','28'),True),
        ('runtime-pointer',payload+'u8* dynamic;\n#pragma fixed_bank 3\n__prg_rom u8* table[1]={dynamic};\n'+end,False),
        ('overflow',payload+'#pragma fixed_bank 3\n__prg_rom u8* table[1]={payload,payload};\n'+end,False),
        ('nonconstant-index',payload+'u8 index;\n#pragma fixed_bank 3\n__prg_rom u8* table[1]={payload+index};\n'+end,False)]
    for name,source,valid in cases:
        folder=out/name;folder.mkdir(exist_ok=True)
        path=folder/'case.c';path.write_text(source,encoding='ascii')
        rom=folder/('case.gb' if platform=='gb' else 'case.nes');rom.unlink(missing_ok=True)
        command=[str(compiler),str(path),'--no-cache','--no-disasm','-o',str(rom)]
        command+=['--cgb=dmg','--profile=dev','--rst-disable','--stack-bank=fixed','--cart=mbc5','--romsize=64k'] if platform=='gb' else ['--mapper=mmc3']
        run=subprocess.run(command,cwd=folder,capture_output=True,timeout=60)
        (folder/'build.log').write_bytes(run.stdout+run.stderr)
        row={'platform':platform,'name':name,'command':command,'exit':run.returncode,'passed':False,'compiler_sha256':hashlib.sha256(compiler.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
        if valid and run.returncode==0 and rom.exists():
            raw=rom.read_bytes();header=0 if platform=='gb' else 16
            payload_at=raw.find(bytes.fromhex('D36BA75EB9C28417'));words_at=raw.find(bytes.fromhex('12397A5F'))
            table_at=raw.find(bytes.fromhex('BA429DE6'))
            def address(pos):return (0x4000 if platform=='gb' else 0x8000)+(pos-header)%0x4000
            ptr=address(payload_at);word=address(words_at)
            expected_ptrs=[ptr,ptr+2,ptr+3,word+1,word+2,ptr+3] if name!='null-and-nested' else [ptr,0,ptr+2,0]
            expected=bytes.fromhex('BA429DE6')+b''.join(p.to_bytes(2,'little') for p in expected_ptrs)+bytes.fromhex('8371')
            expected+=bytes(len(expected))
            # Physical switchable banks are indexed from zero in FC and from one in GB.
            bank_delta=0 if platform=='gb' else 1
            bank_ok=(payload_at-header)//0x4000+bank_delta==2 and (table_at-header)//0x4000+bank_delta==3
            row.update(payload_offset=payload_at,words_offset=words_at,table_offset=table_at,expected_hex=expected.hex(),actual_hex=raw[table_at:table_at+len(expected)].hex(),bank_ok=bank_ok,
                passed=min(payload_at,words_at,table_at)>=0 and bank_ok and raw[table_at:table_at+len(expected)]==expected,
                rom_sha256=hashlib.sha256(raw).hexdigest())
        elif not valid:
            row['passed']=run.returncode!=0 and not rom.exists()
        reports.append(row);print(platform,name,'PASS' if row['passed'] else 'FAIL',flush=True)
(out/'report.json').write_text(json.dumps(reports,indent=2),encoding='utf-8')
print(json.dumps({'report':str(out/'report.json'),'passed':sum(r['passed'] for r in reports),'cases':len(reports)}))
raise SystemExit(0 if all(r['passed'] for r in reports) else 1)
