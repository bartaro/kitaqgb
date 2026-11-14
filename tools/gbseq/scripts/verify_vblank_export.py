"""Build exported C with the native KITAQGB compiler and exercise its ISR in KOKURA."""
from pathlib import Path
import argparse, hashlib, json, re, subprocess, sys

p=argparse.ArgumentParser()
p.add_argument('output',type=Path)
p.add_argument('--cli',type=Path,required=True)
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--library',type=Path)
p.add_argument('--kokura',type=Path,required=True)
p.add_argument('--legacy',action='store_true')
p.add_argument('--fixed-wram',action='store_true')
p.add_argument('--kokura-library',type=Path,help='Explicit kokura_capi DLL/shared-library path')
a=p.parse_args()
dest=a.output.resolve();dest.mkdir(parents=True,exist_ok=True)
cli=a.cli.resolve();compiler=a.compiler.resolve()
song=dest/'input.gbseq'
def run(*args): return subprocess.check_output([str(cli),*map(str,args)],text=True,encoding='utf-8')
run('new',song,'--bars',1,'--bpm',120,'--players',1,'--no-loop')
for lane,pitch in enumerate([14,23,31,4],1):
    run('add-note',song,'--track',lane,'--start',0,'--length',8,'--pitch',pitch,'--out',song)
src=json.loads(song.read_text(encoding='utf-8'))
src['tracks'][2]['notes'][0]['effects'].update(wave_level=2,wave_preset=1)
src['tracks'][3]['notes'][0]['effects'].update(envelope_period=3,noise_shift=4,noise_divisor=3)
src['tracks'][3]['notes'][0]['velocity']=9
song.write_text(json.dumps(src),encoding='utf-8')
report=json.loads(run('export-vblank',song,dest,'--strict',*(['--driver-dir',a.library.resolve()] if a.library else [])))
folder=Path(report['directory'])/'P1'
args=(folder/'build.txt').read_text().split()[1:]
if a.legacy or a.fixed_wram:
    # KITAQGB supports source-level configuration, not a GCC-style -D flag.
    for srcfile in ('../driver/audio_vblank.c','music.c'):
        wrapper='legacy_'+Path(srcfile).name
        defines=('#define AUDIO_VBLANK_LEGACY_INST_ORDER 1\n' if a.legacy else '')+('#define AUDIO_VBLANK_QUEUE_WRAM0 1\n' if a.fixed_wram else '')
        (folder/wrapper).write_text(defines+'#include "'+srcfile+'"\n',encoding='utf-8')
        args[args.index(srcfile)]=wrapper
result=subprocess.run([str(compiler),*args],cwd=folder,capture_output=True)
(dest/'compile.log').write_bytes(result.stdout+result.stderr)
assert result.returncode==0,(dest/'compile.log')
rom=folder/'player.gb'
lines=(folder/'player.map').read_text(encoding='utf-8').splitlines()
def sym(key):
    return next(int(l.split()[0],16) for l in lines if re.search(r'\s'+key+r'\s*$',l))
fields=['MusicPlaying','Ch3Level','Ch4Envelope','QueueUnderruns','Step','Ch1Note','Ch2Note','Ch3Note','Ch4Param']
addrs={k:sym('AudioVBlank_'+k) for k in fields}
sys.path.insert(0,str(a.kokura.resolve()/'python'))
from kokura_bridge import Core,KokuraLibrary
lib=KokuraLibrary(str(a.kokura_library.resolve() if a.kokura_library else a.kokura.resolve()/'target/release/kokura_capi.dll'))
states=[]
with Core(lib) as c:
    c.load_rom_path(rom)
    for f in range(report['frames']+100):
        c.run_frames(1)
        row={k:c.read8(v) for k,v in addrs.items()}
        states.append(dict(f=f,**row,nr32=c.read8(0xff1c)&0x60,nr42=c.read8(0xff21),nr52=c.read8(0xff26)&15))
active=[s for s in states if s['MusicPlaying']]
(dest/'trace.json').write_text(json.dumps(states),encoding='utf-8')
assert active and any(s['Ch3Level']==0x40 and s['Ch4Envelope']==0x93 and s['nr32']==0x40 and s['nr42']==0x93 and s['nr52']==15 for s in active),'Wrong physical APU registers or no audible channels'
assert any([s['Ch1Note'],s['Ch2Note'],s['Ch3Note'],s['Ch4Param']]==[14,23,31,0x43] for s in active),'Wrong note columns'
assert states[-1]['MusicPlaying']==0 and states[-1]['nr52']==0 and not any(s['QueueUnderruns'] for s in states),'song never ended cleanly'
assert abs(len(active)-report['frames'])<=1,(len(active), report['frames'])
proof=dict(rom=rom.name,sha256=hashlib.sha256(rom.read_bytes()).hexdigest(),driver=report['driver_source'],legacy=a.legacy,fixed_wram=a.fixed_wram,frames=len(active),expected_frames=report['frames'],notes=[14,23,31,67],ch3_nr32=64,ch4_nr42=147,four_channels_on=True,queue_underruns=0,one_shot_stop=True)
(dest/'verification.json').write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8')
print(json.dumps(proof))
