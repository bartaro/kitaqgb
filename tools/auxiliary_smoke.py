"""Execute release helper tools against frozen C#/Python and format fixtures."""
from pathlib import Path
import argparse, datetime, hashlib, json, os, platform, subprocess, tempfile

p=argparse.ArgumentParser();p.add_argument('--kind',choices=['gb','fc'],required=True);p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--receipt',type=Path,required=True);a=p.parse_args()
root=Path(__file__).resolve().parents[1];bins=a.bin_dir.resolve();prefix='kitaq'+a.kind;extension='.exe'if os.name=='nt'else''
names=['zx0','patch-vblank']if a.kind=='gb'else['zx0','asset-pack','png-index-build','asset-pipeline','rights-name-guard']
programs={name:bins/(prefix+'-'+name+extension)for name in names}
for path in programs.values():assert path.is_file(),path
sha=lambda data:hashlib.sha256(data).hexdigest()
records=[]
def run(name,case,args,exit=0):
 r=subprocess.run([str(programs[name]),*map(str,args)],capture_output=True,timeout=60)
 assert r.returncode==exit,(case,r.returncode,r.stdout[-1600:],r.stderr[-1600:])
 records.append({'case':case,'exit_code':r.returncode,'passed':True})
 return r
with tempfile.TemporaryDirectory(prefix=prefix+'-helpers-')as d:
 t=Path(d);fixtures=root/'tests/auxiliary-fixtures';metadata=json.loads((fixtures/'manifest.json').read_text())
 for name in names:run(name,'version-'+name,['--version'])
 for case in metadata['cases']:
  source=fixtures/(case+'.input');data=source.read_bytes()
  for format,suffix in [('zx0','zx0'),('rle','rle'),('auto','kqa'),('raw','input')]:
   expected=fixtures/(case+'.'+suffix);out=t/'output.bin'
   if out.exists():out.unlink()
   success=expected.exists()and len(expected.read_bytes())<=65535
   run('zx0',case+'-'+format,[source,out,'--format='+format],0 if success else 1)
   if success:assert out.read_bytes()==expected.read_bytes(),(case,format)
   else:assert not out.exists()
  out=t/'output.h';run('zx0',case+'-header',[source,out,'--format=raw','--header=asset_data']);assert out.read_bytes()==(fixtures/(case+'.header')).read_bytes()
  packed=fixtures/(case+'.zx0')
  if packed.exists():
   out=t/'restored.bin';run('zx0',case+'-restore',[packed,out,'--decompress']);assert out.read_bytes()==data
 for case in metadata['decode_cases']:
  source=fixtures/(case+'.invalid');expected=fixtures/(case+'.invalid.decoded');out=t/'invalid.bin'
  if out.exists():out.unlink()
  r=run('zx0',case,[source,out,'--decompress'],0 if expected.exists()else 1)
  if expected.exists():assert out.read_bytes()==expected.read_bytes()
  else:assert (fixtures/(case+'.invalid.decoded.error')).read_text().strip()in r.stderr.decode('utf-8');assert not out.exists()
 source=t/'safe.bin';source.write_bytes(b'original');run('zx0','prevent-overwrite',[source,source],1);assert source.read_bytes()==b'original'
 if a.kind=='gb':
  f=root/'tests/vblank-helper-fixtures'
  for case,extra in [('normal',[]),('no-header',['--no-header-fix'])]:
   out=t/'patched.gb';out.write_bytes((f/'original.gb').read_bytes());run('patch-vblank','patch-'+case,['--rom',out,'--map',f/'symbols.map',*extra]);assert out.read_bytes()==(f/(case+'.gb')).read_bytes()
 else:
  f=root/'tests/asset-helper-fixtures'
  for path in f.iterdir():
   if path.is_file():(t/path.name).write_bytes(path.read_bytes())
  run('asset-pack','asset-pack',[t/'pack.json'])
  for suffix in ['chr','h','c']:assert(t/'actual'/('packed.'+suffix)).read_bytes()==(f/('expected.'+suffix)).read_bytes()
  report=json.loads((t/'actual/report.json').read_text());report.pop('manifest');assert report==json.loads((f/'expected-report.json').read_text())
  run('png-index-build','indexed-png',[t/'png.json'])
  for name in ['bg.chr','bg.nam','bg.atr','sprite.chr']:assert(t/'actual'/name).read_bytes()==(f/('expected-'+name)).read_bytes()
  run('asset-pipeline','asset-pipeline',[t/'pipeline.json']);assert(t/'actual/pipeline.chr').stat().st_size==8192
  deny=t/'deny.local';deny.write_text('STRASSE\n',encoding='utf-8');(t/'scan.html').write_text('Straße\n',encoding='utf-8')
  r=run('rights-name-guard','unicode-rights-match',['--root',t,'--denylist',deny],1);assert b'STRASSE'not in r.stdout and'Straße'not in r.stdout.decode('utf-8')
  (t/'scan.html').write_text('clean\n');run('rights-name-guard','rights-clean',['--root',t,'--denylist',deny])
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'kind':a.kind,'passed':True,'system':platform.system(),'machine':platform.machine(),'executables':[{'name':path.name,'sha256':sha(path.read_bytes()),'bytes':path.stat().st_size}for path in programs.values()],'checks':len(records),'records':records,'scope':'Release executable output parity, bounded malformed input handling, native asset pipeline and publication scanner. Frozen C#/Python oracle data only; no C# or Python helper called at runtime.'}
a.receipt.write_text(json.dumps(receipt,indent=2),encoding='utf-8');print(json.dumps({k:v for k,v in receipt.items()if k not in ['records','executables']},indent=2))
