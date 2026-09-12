#pragma once

// Fixed-bank VBlank BGM driver.
//
// Stream format:
//   delay, ch2_note, ch1_note, ch3_note, ch4_noise_param
//   IMMEDIATE, ch2_note, ch1_note, ch3_note, ch4_noise_param
//   CONTROL, command, arg0, arg1, arg2
//
// Notes are 0..67. To stay inside the useful CGB register range, C6..G6
// (60..67) alias C5..G5 (48..55) in the fixed pitch table. REST leaves a
// channel unchanged, while STOP silences it. Use LOOP as a delay byte to jump
// back to the song start and END as a delay byte for a one-shot song end.

#define AUDIO_VBLANK_REST ((u8)0xFF)
#define AUDIO_VBLANK_END  ((u8)0xFF)
#define AUDIO_VBLANK_LOOP ((u8)0xFE)
#define AUDIO_VBLANK_STOP ((u8)0xFD)
#define AUDIO_VBLANK_CONTROL ((u8)0xFC)
#define AUDIO_VBLANK_IMMEDIATE ((u8)0xFB)
#define AUDIO_VBLANK_CONTROL_RESET ((u8)0x01)

#define AUDIO_VBLANK_W8 ((u8)15)
#define AUDIO_VBLANK_WQ ((u8)30)
#define AUDIO_VBLANK_WH ((u8)60)

#define AUDIO_VBLANK_QUEUE_RECORDS 16
#define AUDIO_VBLANK_QUEUE_BYTES 80

typedef void (*AudioVBlankFrameHook)();

extern __wram u8 AudioVBlank_MusicPlaying;
extern __wram u8 AudioVBlank_MusicEnabled;
extern __wram u8 AudioVBlank_MusicDelay;
extern __wram u8 *AudioVBlank_MusicStart;
extern __wram u8 *AudioVBlank_MusicPointer;
extern __wram u8 *AudioVBlank_MusicLoopPoint;
extern __wram u8 AudioVBlank_Step;
extern __wram u8 AudioVBlank_Ch2Note;
extern __wram u8 AudioVBlank_Ch1Note;
extern __wram u8 AudioVBlank_Ch3Note;
extern __wram u8 AudioVBlank_Ch2LatchedNote;
extern __wram u8 AudioVBlank_Ch1LatchedNote;
extern __wram u8 AudioVBlank_Ch3LatchedNote;
// Sound effects temporarily own CH1/CH2/CH3.  A pending value of 1 means a BGM
// note/stop event was suppressed and must be applied once the effect ends;
// value 2 means release the effect channel without retriggering an old note.
extern __wram u8 AudioVBlank_RestoreCh1Pending;
extern __wram u8 AudioVBlank_RestoreCh3Pending;
extern __wram u8 AudioVBlank_Ch4Param;
extern __wram u8 AudioVBlank_Ch1Envelope;
extern __wram u8 AudioVBlank_Ch2Envelope;
extern __wram u8 AudioVBlank_Ch4Envelope;
extern __wram u8 AudioVBlank_Ch1Duty;
extern __wram u8 AudioVBlank_Ch2Duty;
extern __wram u8 AudioVBlank_Ch1Sweep;
extern __wram u8 AudioVBlank_Ch3Level;
extern __wram u8 AudioVBlank_ControlCommand;
extern __wram u8 AudioVBlank_ControlArg0;
extern __wram u8 AudioVBlank_ControlArg1;
extern __wram u8 AudioVBlank_ControlArg2;
extern __wram u8 AudioVBlank_QueueMode;
extern __wram u8 AudioVBlank_QueueReadIndex;
extern __wram u8 AudioVBlank_QueueWriteIndex;
extern __wram u8 AudioVBlank_QueueCount;
extern __wram u8 AudioVBlank_QueueUnderruns;
extern __wramx_bank(1) u8 AudioVBlank_QueueBuffer[AUDIO_VBLANK_QUEUE_BYTES];
extern __wram AudioVBlankFrameHook AudioVBlank_FrameHook;

void AudioVBlank_Init();
void __stackcall AudioVBlank_SetMusic(u8 *song);
void AudioVBlank_Play();
void __stackcall AudioVBlank_PlayMusic(u8 *song);
void AudioVBlank_Stop();
void __stackcall AudioVBlank_SetEnabled(u8 on);
void AudioVBlank_EnableIrq();
void AudioVBlank_DisableIrq();
void AudioVBlank_RestoreCh1();
void AudioVBlank_RestoreCh3();
void AudioVBlank_RequestRestoreCh1();
void AudioVBlank_RequestRestoreCh3();
