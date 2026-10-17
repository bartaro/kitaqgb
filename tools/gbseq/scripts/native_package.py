"""Package only tested binaries, documentation, fonts and their full notices."""
from pathlib import Path
import argparse, hashlib, json, os, platform, shutil, struct, subprocess, sys, tempfile, wave
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--platform',required=True);a=p.parse_args()
dest=ROOT/'dist'/a.platform;dest.mkdir(parents=True,exist_ok=True)
suffix='.exe' if os.name=='nt' else ''
library='gbseq.dll' if os.name=='nt' else ('libgbseq.dylib' if sys.platform=='darwin' else 'libgbseq.so')
names=['gbseq'+suffix,'gbseq-cli'+suffix,library]
for name in names:shutil.copy2(ROOT/'target/release'/name,dest/name)
for name in ['LICENSE','THIRD_PARTY_NOTICES.md','LICENSE_AUDIT.json']:
    shutil.copy2(ROOT/name,dest/name)
for name in ['licenses','assets','docs','python']:
    shutil.copytree(ROOT/name,dest/name,dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__'))
for file in ROOT.glob('README*.md'):shutil.copy2(file,dest/file.name)
rustdocs=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())/'share/doc/rust'
for file in [rustdocs/'COPYRIGHT-library.html',*(rustdocs/'licenses').rglob('*')]:
    if file.is_file():
        out=dest/'licenses/native-rust-toolchain'/file.relative_to(rustdocs)
        out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(file,out)
proof=dest/'verification';proof.mkdir(exist_ok=True)
cli=dest/('gbseq-cli'+suffix)
def run(*args,expected=0):
    result=subprocess.run([str(cli),*map(str,args)],capture_output=True,text=True,encoding='utf-8',timeout=120)
    assert result.returncode==expected,(args,result.stdout,result.stderr)
    return result.stdout
tests=[]
with tempfile.TemporaryDirectory() as tmp:
    tmp=Path(tmp)
    for players in [1,2,4]:
        song=tmp/f'{players}.gbseq'; wav=tmp/f'{players}.wav'; saves=tmp/f'saves{players}'
        run('new',song,'--bars','1','--bpm','150','--players',players,'--no-loop')
        for track in range(1,players*4+1):
            run('add-note',song,'--track',track,'--start','0','--length','2','--pitch','24','--out',song)
        run('validate',song)
        run('render-wav',song,wav,'--all-players')
        with wave.open(str(wav)) as w:
            data=w.readframes(w.getnframes())
            assert w.getnchannels()==2 and w.getsampwidth()==2
            samples=struct.unpack('<'+'h'*(len(data)//2),data)
            assert max(samples)!=min(samples)
            info={'players':players,'sample_rate':w.getframerate(),'frames':w.getnframes(),
                  'pcm_sha256':hashlib.sha256(data).hexdigest()}
        run('export-link16-sav',song,saves)
        images=list(saves.glob('*.sav'))
        assert len(images)==players and all(f.stat().st_size==131072 for f in images)
        run('export-kitaqgb',song,tmp/f'c{players}','--player',players)
        before=song.read_bytes()
        bad=subprocess.run([str(cli),'add-note',str(song),'--track','1','--start','0','--out',str(song)],capture_output=True)
        assert bad.returncode!=0 and before==song.read_bytes(),'invalid overlapping edit changed source'
        tests.append(info)
    os.environ['GBSEQ_NATIVE_LIBRARY']=str((dest/library).resolve())
    sys.path.insert(0,str(dest/'python'))
    from gbseq import Project
    project=Project();project.data['measures']=1
    project.add_note(track=0,start=0,length=2,pitch=24,velocity=12)
    project.validate();project.save(str(tmp/'python.gbseq'))
    run('validate',tmp/'python.gbseq')
report={'passed':True,'platform':a.platform,'system':platform.platform(),
 'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
 'rustc':subprocess.check_output(['rustc','-vV'],text=True).strip(),
 'audio_tests':tests,'python_ctypes_tested':True,'invalid_edit_preserves_source':True,
 'binaries':{name:hashlib.sha256((dest/name).read_bytes()).hexdigest() for name in names},
 'physical_hardware_tested':False,'audio_output_device_tested':False}
(proof/'native.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps(report,indent=2))
