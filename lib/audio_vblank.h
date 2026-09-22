#pragma once

// Fixed-bank VBlank BGM driver.
//
// Stream format:
//   delay, ch1_note, ch2_note, ch3_note, ch4_noise_param
//   IMMEDIATE, ch1_note, ch2_note, ch3_note, ch4_noise_param
//   CONTROL, command, arg0, arg1, arg2
//
// Notes are 0..67. To stay inside the useful CGB register range, C6..G6
// (60..67) alias C5..G5 (48..55) in the fixed pitch table. REST leaves a
// channel unchanged, while STOP silences it. END stops either playback mode.
// LOOP jumps to MusicLoopPoint only in direct-pointer mode; a queue producer
// must expand loops itself. IMMEDIATE is recognized only in queue mode.
// Direct streams treat 0xFB as an ordinary delay, and queues treat 0xFE as one.

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
// QueueReset/QueueRefill/QueuePlay provide the foreground producer. The ISR owns
// ReadIndex and decreases Count; use the producer APIs instead of editing the
// indices/count while playback is running. Records contain exactly five bytes.
// MusicPlaying and all enable/pause gates still apply in queue mode.
extern __wram u8 AudioVBlank_QueueMode;
extern __wram u8 AudioVBlank_QueueReadIndex;
extern __wram u8 AudioVBlank_QueueWriteIndex;
extern __wram u8 AudioVBlank_QueueCount;
extern __wram u8 AudioVBlank_QueueUnderruns;
// Queue bytes reside in WRAM bank 1. A producer must select that bank while
// writing and restore the caller bank; publish the record only after all bytes
// are complete. The ISR selects bank 1 itself and restores SVBK before returning.
extern __wramx_bank(1) u8 AudioVBlank_QueueBuffer[AUDIO_VBLANK_QUEUE_BYTES];
// The ISR invokes this hook even when music processing is gated off. It runs
// with SVBK=1 using a direct-address call; keep hook code/data accessible in that
// context and synchronize pointer changes with interrupt execution.
extern __wram AudioVBlankFrameHook AudioVBlank_FrameHook;

// Reset software playback state, queue counters, instruments and the frame hook.
// Call before enabling this ISR; the routine does not mask interrupts or
// initialize the APU registers and cannot synchronize with an active handler.
void AudioVBlank_Init();
// Stop music and select an empty WRAM queue; preserve IE, IME and enable/pause gates.
void AudioVBlank_QueueReset();
// Copy complete records from the currently mapped source; return records accepted.
// Each call attempts at most 16 records and stops when full. Null/zero returns 0.
// Supports ROM and RAM sources, restores SVBK and IE, and never changes IME.
// Single foreground producer only: do not call from an ISR or frame hook.
// LOOP must be expanded by the producer. END occupies a full five-byte record.
u8 __stackcall AudioVBlank_QueueRefill(const u8 *records, u8 count);
// Start/resume queued playback; return 1 on success, 0 for an empty/nonqueue mode.
// Does not enable the interrupt, music gate or clear pause. Queue before starting.
u8 AudioVBlank_QueuePlay();
// Select a directly addressed song and reset its cursors, leaving the playing
// flag unchanged. Song storage must remain accessible to the fixed-bank ISR;
// coordinate this multi-field update with interrupt execution.
void __stackcall AudioVBlank_SetMusic(u8 *song);
// Restart the selected direct song at its beginning, leaving queue mode.
// A null song invokes Stop; this does not enable the music or IRQ gates.
void AudioVBlank_Play();
// Select and restart a directly addressed song. This is not the banked WRAM-queue path.
void __stackcall AudioVBlank_PlayMusic(u8 *song);
// Stop software playback and silence pulse/noise envelopes and the wave DAC.
// Song pointers and the IRQ enable remain unchanged; the routine writes the
// physical channels directly, so coordinate ownership with sound effects.
void AudioVBlank_Stop();
// Gate music processing and clear pending CH1/CH3 recovery on disable.
// No audio register is written here: disabling does not itself silence a note
// already playing or disable the VBlank interrupt.
void __stackcall AudioVBlank_SetEnabled(u8 on);
// Enable the VBlank source in IE and execute EI, enabling CPU interrupts globally.
// Install the correct interrupt vector and initialize driver state first.
void AudioVBlank_EnableIrq();
// Clear only the VBlank bit in IE. Other interrupt sources and current sound remain unchanged.
void AudioVBlank_DisableIrq();
// Recover the borrowed pulse channel (CH1 or CH2) from its latched music event.
// Pending value 2, disabled music, or a non-note latch silences it instead.
// The caller must first establish that the effect has released the channel.
void AudioVBlank_RestoreCh1();
// Recover a latched wave-channel music event after effect release, or disable
// the DAC for a release-only request, disabled music, or a non-note latch.
// The caller owns effect arbitration and clearing the pending flag.
void AudioVBlank_RestoreCh3();
// Request deferred pulse-channel release only when no recovery is pending.
// Preserve value 1, which records a music event suppressed while an effect owned it.
void AudioVBlank_RequestRestoreCh1();
// Request a deferred wave-channel release only if no recovery is pending.
// Preserve value 1, which records a new music event suppressed by the effect.
void AudioVBlank_RequestRestoreCh3();
