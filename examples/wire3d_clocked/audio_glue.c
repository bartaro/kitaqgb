// MIT. Minimal audio_vblank integration for this independent BGM-only example.
// Full games may instead compile their usual audio/SFX modules.
#pragma bank 0
__hram u8 *Audio_EffectPointer;
__hram u8 *Audio_EffectPointer3;
__wram u8 Audio_EffectUsesCh2;
__hram u8 Audio_Paused;
__hram u8 Audio_NoiseEffectActive;
__wram u8 Audio_MusicEnabled;
__hram u8 Audio_Ch1Sweep;
__hram u8 Audio_Pan;
__location(0xFF26) u8 wf_nr52;
__location(0xFF24) u8 wf_nr50;
__location(0xFF25) u8 wf_nr51;
void Audio_Init(){
 Audio_EffectPointer=0;Audio_EffectPointer3=0;Audio_EffectUsesCh2=0;
 Audio_Paused=0;Audio_NoiseEffectActive=0;Audio_MusicEnabled=1;
 Audio_Ch1Sweep=0;Audio_Pan=0;wf_nr52=128;wf_nr50=119;wf_nr51=255;
}
