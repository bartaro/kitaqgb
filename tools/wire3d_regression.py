"""MIT. Build and verify original wire3d clocked examples using KOKURA.
No commercial ROM, art, music or saved state is required or read.
"""
from pathlib import Path
import argparse,json,subprocess,statistics,sys
R=Path(__file__).resolve().parents[1]
from wire3d_metrics import summarize

def compiler_command(cmd):
 return [cmd[0],"compile",*cmd[1:]] if COMPILER_KIND=="rust" else cmd

def projection_build(tag,source,lib):
 d=B/tag;d.mkdir(exist_ok=True);m=d/"main.c";m.write_text(source,encoding="utf-8");rom=d/"probe.gb"
 cmd=[COMPILER,lib,m,"-I",R/"lib","-o",rom,"-O1","--stack-bank=fixed","--stack-reserve=192","--rst-disable","--no-disasm","--romsize=256k","--cart=mbc1"]
 p=subprocess.run(list(map(str,compiler_command(cmd))),cwd=d,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (d/"build.log").write_bytes(p.stdout);p.check_returncode();return rom

def projection_run(rom):
 c=Core(KokuraLibrary(DLL));c.load_rom_path(rom)
 for _ in range(667):
  c.run_frames(30);c.drain_audio_frames()
  if c.peek8(0xCEFF)==165:break
 assert c.peek8(0xCEFF)==165,"Projection test did not finish"
 data=list(c.read_block(0xCE00,4));c.close()
 failures=data[0]+256*data[1];cases=data[2]+256*data[3]
 assert failures==0 and cases==5140,(failures,cases)
 return {"cases":cases,"failures":failures}
vals=[-32768,-513,-256,-221,-121,-120,-119,-64,-1,0,1,31,64,119,120,121,220,256,512,32767]
def projection_source(kind,height):
 p='Wire3DDMG' if kind=='dmg' else 'Wire3DCGB';macro='WIRE3D_DMG_HEIGHT' if kind=='dmg' else 'WIRE3DCGB_HEIGHT'
 return f'#define {macro} {height}\n#include "wire3d_{kind}.h"\n#pragma bank 0\n__prg_rom s16 inputs[20]={{'+','.join(map(str,vals))+'};\n'+'''__location(0xCE00) u16 failures;
__location(0xCE02) u16 checked;
__location(0xCEFF) u8 done;
u8 clamp(s16 v,u8 hi){if(v<0)return 0;if(v>(s16)hi)return hi;return (u8)v;}
void main(){u8 i;u8 sx;u8 sy;u8 ok;s16 z;s16 v;s16 w;s16 factor;s16 ox;s16 oy;
 failures=0;checked=0;done=0;z=0;
 while(z<257){i=0;while(i<20){v=inputs[i];w=inputs[(u8)(19-i)];sx=222;sy=111;
 ok=PREFIX_ProjectCameraPoint(v,w,z,&sx,&sy);
 if(z<8 || z>255){if(ok || sx!=222 || sy!=111)failures++;}
 else{if(v < -120)v=-120;if(v>120)v=120;if(w < -120)w=-120;if(w>120)w=120;
 factor=(s16)(1536/z);ox=(s16)((v*factor)>>5);oy=(s16)((w*factor)>>5);
 if(!ok || sx!=clamp((s16)(64+ox),127) || sy!=clamp((s16)(CENTER-oy),BOTTOM))failures++;}
 checked++;i++;}z++;}done=165;while(1){__wait_vblank();}}
'''.replace('PREFIX',p).replace('CENTER',str(height//2)).replace('BOTTOM',str(height-1))


def stamp(s):return s['clocks']['frames']*70224+s['clocks']['frame_phase_ppu_cycles']
def symbols(d):return {x['name']:x for x in json.loads((d/'probe.dbg2.json').read_text(encoding='utf-8'))['symbols']}
def build(kind,height,mode='sparse'):
 tag=f'clocked_{kind}_{height}_{mode}';d=B/tag;d.mkdir(exist_ok=True)
 source=(R/f'examples/wire3d_clocked/clocked_{kind}_{height}.c').read_text(encoding='utf-8')
 if mode=='full' and kind=='cgb':source=source.replace('BeginFrameSparse','BeginFrame').replace('EndFrameSparseNow','EndFrame')
 m=d/'main.c';m.write_text(source,encoding='utf-8');lib=B/f'lib_{kind}_{height}.c'
 text=(R/f'lib/wire3d_{kind}.c').read_text(encoding='utf-8')
 macro='WIRE3D_DMG_HEIGHT' if kind=='dmg' else 'WIRE3DCGB_HEIGHT'
 lib.write_text(f'#define {macro} {height}\n'+('#define WIRE3DCGB_STAGE_BANK2_ALLOCATION 1\n' if kind=='cgb' else '')+text,encoding='utf-8')
 audio=R/'lib/wire3d_audio_vblank.c'
 cmd=[COMPILER,R/'examples/wire3d_clocked/hardware.c',lib,R/'examples/wire3d_clocked/audio_glue.c',audio,m,'-I',R/'lib','-o',d/'probe.gb','-O1','--stack-bank=fixed','--stack-reserve=192','--rst-disable','--no-disasm','--romsize=256k','--cart=mbc1','--debug-out='+str(d/'debug')]
 cmd=compiler_command(cmd)
 p=subprocess.run(list(map(str,cmd)),cwd=d,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(d/'build.log').write_bytes(p.stdout)
 if p.returncode:print(p.stdout.decode('utf-8',errors='replace')[-2200:]);p.check_returncode()
 print('BUILD',tag,flush=True);return d

def measure(d,kind,height):
 sy=symbols(d);pc=sy['wf_marker']['start'];c=DebugSession(KokuraLibrary(DLL));c.load_rom_path(d/'probe.gb');c.set_stop_conditions({'breakpoints':[{'pc':pc,'bank':0}]})
 records=[];samples=[];staged=None;hud=set();frequencies=set();audio_nonzero=False;checks=0;last_ticks=None;blank_checks=0
 for step in range(48*6):
  c.run_frames(240);a=c.drain_audio_frames();audio_nonzero=audio_nonzero or any(abs(float(x))>0.00001 for pair in a for x in pair)
  assert c.registers().pc==pc,(d,c.registers(),step)
  s=c.save_state();phase=c.peek8(0xCE11);frame=c.peek8(0xCE12)
  samples.append({'frame':frame,'phase':phase,'ppu_dots':stamp(s),'cpu_cycles':s['clocks']['cycles']})
  assert s['ppu']['lcdc']&128
  vb=s['memory']['vram_banks'];v=[vb[:8192],vb[8192:]] if not isinstance(vb[0],list) else vb
  if kind=='dmg':v=[s['memory']['data'][0x8000:0xA000]]
  hud.add(v[0][0x1A20]);freq=s['apu']['regs'];frequencies.add(tuple(freq[3:5]))
  if phase==3:
   if kind=='dmg':staged=bytes(b for col in range(16) for b in c.read_block(0xD000+256*col,height))
   else:
    old=c.peek8(0xFF70);c.write8(0xFF70,2);staged=bytes(c.read_block(0xD300,height*32));c.write8(0xFF70,old)
  if phase==4:
   assert staged is not None
   if kind=='dmg':actual=bytes(v[0][0x901:0x901+height*32:2])
   else:actual=bytes(v[c.read_block(sy['w3dcgb_display_tile_bank']['start'],1)[0]][0x900:0x900+height*32])
   assert actual==staged,(d,frame,[(i,a,b) for i,(a,b) in enumerate(zip(staged,actual)) if a!=b][:8])
   assert v[0][0x1A12]==128+(frame&7),(d,frame,'HUD queue')
   if kind=='cgb':assert v[0][0x1E12]==128+(frame&7)
   checks+=1
   if (frame&7) in [2,3]:assert not any(actual),(d,frame,'old pixels');blank_checks+=1
   if frame==8:
    c.save_screenshot(str(d/'screen.bmp'))
    try:
     from PIL import Image
     Image.open(d/'screen.bmp').save(d/'screen.png')
    except ImportError:pass
  ret=int.from_bytes(c.read_block(c.registers().sp,2),'little');c.set_stop_conditions({'breakpoints':[{'pc':ret,'bank':0}]});c.run_frames(1);assert c.registers().pc==ret;c.set_stop_conditions({'breakpoints':[{'pc':pc,'bank':0}]})
 c.set_stop_conditions({});c.run_frames(12);c.drain_audio_frames();assert c.peek8(0xCEFF)==165
 assert len(hud)>1 and len(frequencies)>=4 and audio_nonzero,(d,hud,frequencies,audio_nonzero)
 for frame in range(48):
  row=[x for x in samples if x['frame']==frame];assert [x['phase'] for x in row]==list(range(6))
  times=[x['ppu_dots'] for x in row];rec={'frame':frame,'clear_dots':times[1]-times[0],'projection_dots':times[2]-times[1],'raster_dots':times[3]-times[2],'upload_and_presentation_dots':times[4]-times[3],'pacing_wait_dots':times[5]-times[4],'completion_ppu_dots':times[4]}
  records.append(rec)
 intervals=[records[i]['completion_ppu_dots']-records[i-1]['completion_ppu_dots'] for i in range(1,48)]
 report={'profile':d.name,'completed_frames':checks,'blank_history_checks':blank_checks,'stage_matches_vram':True,'hud_values':sorted(hud),'bgm_frequencies':sorted(frequencies),'audio_nonzero':audio_nonzero,'completed_upload_fps':4194304/statistics.mean(intervals),'mean_interval_ppu_frames':statistics.mean(intervals)/70224,'worst_interval_ppu_frames':max(intervals)/70224,'deadline_ppu_frames':4,'deadline_misses':sum(x>4*70224 for x in intervals),'phase_mean_ppu_dots':{key:statistics.mean(x[key] for x in records) for key in records[0] if key.endswith('_dots') and key!='completion_ppu_dots'},'hardware_verified':False}
 (d/'samples.json').write_text(json.dumps(records,indent=2),encoding='utf-8');c.close();print('PASS',json.dumps(report),flush=True);return report

def main():
 global B,COMPILER,COMPILER_KIND,DLL,Core,KokuraLibrary,DebugSession
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument("--compiler",type=Path,required=True);p.add_argument("--compiler-kind",choices=["rust","csharp"],default="rust")
 p.add_argument("--kokura-dll",type=Path,required=True);p.add_argument("--bridge-dir",type=Path,required=True)
 p.add_argument("--output",type=Path,required=True);p.add_argument("--kind",choices=["all","dmg","cgb"],default="all")
 p.add_argument("--height",type=int,choices=[88,96,120]);p.add_argument("--projection-only",action="store_true");a=p.parse_args()
 if a.kind=="cgb" and a.height==120:p.error("CGB height must be 88 or 96")
 COMPILER=a.compiler.resolve();COMPILER_KIND=a.compiler_kind;DLL=a.kokura_dll.resolve();B=a.output.resolve();B.mkdir(parents=True,exist_ok=True)
 sys.path.insert(0,str(a.bridge_dir.resolve()))
 from kokura_bridge import Core as C,KokuraLibrary as K,DebugSession as D
 Core,KokuraLibrary,DebugSession=C,K,D
 report={"projection":{},"runtime":[],"provenance":"Independently authored box, gauge and four notes; no commercial game data."}
 for kind,height in [("dmg",88),("dmg",96),("dmg",120),("cgb",88),("cgb",96)]:
  if a.kind!="all" and a.kind!=kind:continue
  if a.height is not None and a.height!=height:continue
  entry=R/f"examples/wire3d_clocked/renderer_{kind}_{height}.c"
  report["projection"][f"{kind}_{height}"]=projection_run(projection_build(f"projection_{kind}_{height}",projection_source(kind,height),entry))
  if not a.projection_only:
   for mode in (["sparse","full"] if kind=="cgb" else ["sparse"]):
    d=build(kind,height,mode);row=measure(d,kind,height)
    row["metrics"]=summarize(json.loads((d/"samples.json").read_text(encoding="utf-8")))
    report["runtime"].append(row)
  (B/"results.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
 print(json.dumps(report,indent=2))
if __name__=="__main__":main()
