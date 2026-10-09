// MIT. Independently authored moving box, bar gauge and four-note pattern.
// Define the same height for this translation unit and the renderer.
#define WIRE3D_DMG_HEIGHT 96
#include "wire3d_dmg.h"
#include "audio.h"
#define AUDIO_VBLANK_QUEUE_WRAM0 1
#include "audio_vblank.h"
#pragma bank 0
__location(0xCE10) u8 wf_ticks;
__location(0xCE11) u8 wf_phase;
__location(0xCE12) u8 wf_frame;
__location(0xCEFF) u8 wf_done;
__location(0xFF40) u8 wf_lcdc;
__location(0xFF44) u8 wf_ly;
__location(0x8800) u8 wf_hud_vram[128];
__prg_rom u8 wf_glyphs[128]={0,0,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,3,3,3,3,3,3,3,3,3,3,3,3,0,0,0,0,7,7,7,7,7,7,7,7,7,7,7,7,0,0,0,0,15,15,15,15,15,15,15,15,15,15,15,15,0,0,0,0,31,31,31,31,31,31,31,31,31,31,31,31,0,0,0,0,63,63,63,63,63,63,63,63,63,63,63,63,0,0,0,0,127,127,127,127,127,127,127,127,127,127,127,127,0,0,0,0,255,255,255,255,255,255,255,255,255,255,255,255,0,0};
__prg_rom u8 wf_song[21]={8,36,255,255,255,8,40,255,255,255,8,43,255,255,255,8,47,255,255,255,254};
__prg_rom s16 wf_xyz[24]={-24,-20,0,24,-20,0,24,20,0,-24,20,0,-24,-20,32,24,-20,32,24,20,32,-24,20,32};
u8 wf_sx[8];u8 wf_sy[8];
__prg_rom u8 wf_edges[24]={0,1,1,2,2,3,3,0,4,5,5,6,6,7,7,4,0,4,1,5,2,6,3,7};
#pragma fixed_bank 0
__unsafe void wf_marker(){__asm {RET}}
#pragma fixed_bank -1
#pragma fixed_bank 0
// Called by audio_vblank after its work. No renderer or C scratch in the ISR.
__unsafe void wf_tick_hook(){__asm {
 LD_HL_IMM wf_ticks
 INC_HL_REF
 LDH_A_MEM 68
 CP_IMM 148
 RET_NC
 CP_IMM 144
 RET_C
 LD_A_MEM wf_ticks
 OR_A
 RRA
 OR_A
 RRA
 OR_A
 RRA
 AND_IMM 7
 ADD_A_IMM 128
 LD_MEM_A 0x9A20
 RET
}}
#pragma bank 0
#pragma fixed_bank -1
void main(){u8 i;u8 j;u8 a;u8 b;u8 start;u8 now;u8 previous;u8 elapsed;s16 z;u16 distance;
 wf_done=0;wf_frame=0;wf_ticks=0;distance=0;previous=0;
 Wire3DDMG_Init();
 while(wf_ly>=144){} while(wf_ly<144){} wf_lcdc=0;
 for(i=0;i<128;i++)wf_hud_vram[i]=wf_glyphs[i];wf_lcdc=0x81;
 Audio_Init();AudioVBlank_Init();AudioVBlank_FrameHook=wf_tick_hook;
 AudioVBlank_PlayMusic(wf_song);AudioVBlank_EnableIrq();
 while(wf_frame<48){
  start=wf_ticks;now=start;elapsed=(u8)(now-previous);previous=now;
  distance=(u16)(distance+(u16)elapsed); // simulation advances by elapsed VBlanks
  wf_phase=0;wf_marker();
  Wire3DDMG_BeginFrame();
  wf_phase=1;wf_marker();z=(s16)(64+(distance&63));
  for(i=0;i<8;i++){j=(u8)(i*3);Wire3DDMG_ProjectCameraPoint(wf_xyz[j],wf_xyz[(u8)(j+1)],(s16)(wf_xyz[(u8)(j+2)]+z),&wf_sx[i],&wf_sy[i]);}
  wf_phase=2;wf_marker();
  // Two empty frames remove the old box from both CGB destination banks.
  if((wf_frame&7)!=2 && (wf_frame&7)!=3){
   for(i=0;i<12;i++){j=(u8)(i*2);a=wf_edges[j];b=wf_edges[(u8)(j+1)];
    Wire3DDMG_DrawLine2D(wf_sx[a],wf_sy[a],wf_sx[b],wf_sy[b]);
   }
  }
  // The main-thread queue still works alongside the independent ISR gauge.
  Wire3DDMG_PutBgTile(18,16,(u8)(128+(wf_frame&7)));
  wf_phase=3;wf_marker();
  Wire3DDMG_EndFrameNow();
  wf_phase=4;wf_marker();
  while((u8)(wf_ticks-start)<4){} // pacing; never slow the audio clock
  wf_phase=5;wf_marker();wf_frame++;
 }wf_done=165;while(1){__wait_vblank();}
}
