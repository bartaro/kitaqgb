#include "audio_vblank.h"

#pragma fixed_bank 0

extern __hram u8 *Audio_EffectPointer;
extern __hram u8 *Audio_EffectPointer3;
extern __wram u8 Audio_EffectUsesCh2;
extern __hram u8 Audio_Paused;
extern __hram u8 Audio_NoiseEffectActive;
extern __wram u8 Audio_MusicEnabled;
extern __hram u8 Audio_Ch1Sweep;
extern __hram u8 Audio_Pan;

__location(0xFFFF) u8 AudioVBlank_IE;

// Playback gates, direct-stream cursors and cached hardware parameters live in
// fixed WRAM. Foreground producers must coordinate updates with the ISR.
__wram u8 AudioVBlank_MusicPlaying;
__wram u8 AudioVBlank_MusicEnabled;
__wram u8 AudioVBlank_MusicDelay;
__wram u8 *AudioVBlank_MusicStart;
__wram u8 *AudioVBlank_MusicPointer;
__wram u8 *AudioVBlank_MusicLoopPoint;
__wram u8 AudioVBlank_Step;
__wram u8 AudioVBlank_Ch2Note;
__wram u8 AudioVBlank_Ch1Note;
__wram u8 AudioVBlank_Ch3Note;
__wram u8 AudioVBlank_Ch2LatchedNote;
__wram u8 AudioVBlank_Ch1LatchedNote;
__wram u8 AudioVBlank_Ch3LatchedNote;
__wram u8 AudioVBlank_RestoreCh1Pending;
__wram u8 AudioVBlank_RestoreCh3Pending;
__wram u8 AudioVBlank_Ch4Param;
__wram u8 AudioVBlank_Ch1Envelope;
__wram u8 AudioVBlank_Ch2Envelope;
__wram u8 AudioVBlank_Ch4Envelope;
__wram u8 AudioVBlank_Ch1Duty;
__wram u8 AudioVBlank_Ch2Duty;
__wram u8 AudioVBlank_Ch1Sweep;
__wram u8 AudioVBlank_Ch3Level;
__wram u8 AudioVBlank_ControlCommand;
__wram u8 AudioVBlank_ControlArg0;
__wram u8 AudioVBlank_ControlArg1;
__wram u8 AudioVBlank_ControlArg2;
// The queue stores five-byte records in 16 reusable slots. Count distinguishes
// empty from full when the read and write indices are equal; underruns wrap at 255.
__wram u8 AudioVBlank_QueueMode;
__wram u8 AudioVBlank_QueueReadIndex;
__wram u8 AudioVBlank_QueueWriteIndex;
__wram u8 AudioVBlank_QueueCount;
__wram u8 AudioVBlank_QueueUnderruns;
// The ISR selects SVBK=1 before consuming the queue. Keep only the large
// payload in switchable WRAM so existing fixed-WRAM control ABI is unchanged.
__wramx_bank(1) u8 AudioVBlank_QueueBuffer[AUDIO_VBLANK_QUEUE_BYTES];
__wram AudioVBlankFrameHook AudioVBlank_FrameHook;

__prg_rom u8 AudioVBlank_FreqLo[68] = {
    0x2C, 0x9C, 0x06, 0x6B, 0xC9, 0x23, 0x77, 0xC6, 0x12, 0x56, 0x9B, 0xDA,
    0x16, 0x4E, 0x83, 0xB5, 0xE5, 0x11, 0x3B, 0x63, 0x89, 0xAC, 0xCE, 0xED,
    0x0A, 0x27, 0x42, 0x5B, 0x72, 0x89, 0x9E, 0xB2, 0xC4, 0xD6, 0xE7, 0xF7,
    0x06, 0x14, 0x21, 0x2D, 0x39, 0x44, 0x4F, 0x59, 0x62, 0x6B, 0x73, 0x7B,
    0x82, 0x89, 0x90, 0x96, 0x9C, 0xA2, 0xA7, 0xAC,
    0xB1, 0xB5, 0xB9, 0xBD,
    /* C6..G6 fold to C5..G5 without an IRQ-time branch. */
    0x82, 0x89, 0x90, 0x96, 0x9C, 0xA2, 0xA7, 0xAC
};

__prg_rom u8 AudioVBlank_FreqHi[68] = {
    0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07
};

// Service fixed-bank music during VBlank, preserving CPU registers and SVBK.
// Accept either a directly addressable record stream or the 16-slot WRAM queue;
// no ROM bank switch is performed. The frame hook runs even when music is gated off.
__unsafe void __kq_vblank_vector()
{
    __asm {
        PUSH_AF
        PUSH_BC
        PUSH_DE
        PUSH_HL
        LDH_A_MEM 0x70
        PUSH_AF
        LD_A_IMM 1
        LDH_MEM_A 0x70
        /* B remains live across an IMMEDIATE record chain. Zero means the
           legacy APU phase wait has not run yet in this VBlank. */
        LD_B_IMM 0

        LD_A_MEM AudioVBlank_MusicPlaying
        OR_A
        JP_Z audiovb_irq_done
        LD_A_MEM AudioVBlank_MusicEnabled
        OR_A
        JP_Z audiovb_irq_done
        LD_A_MEM Audio_MusicEnabled
        OR_A
        JP_Z audiovb_irq_done
        LD_A_MEM Audio_Paused
        OR_A
        JP_NZ audiovb_irq_done

        /* A pending value of 1 applies a BGM event that arrived while the SFX
           owned the channel.  Value 2 only releases the SFX channel; it must
           not retrigger an older, already-heard sustained note. */
        LD_A_MEM AudioVBlank_RestoreCh1Pending
        OR_A
        JR_Z audiovb_irq_restore_ch3_pending
        LD_A_MEM Audio_EffectPointer
        OR_A
        JR_NZ audiovb_irq_restore_ch3_pending
        LD_A_MEM Audio_EffectPointer+1
        OR_A
        JR_NZ audiovb_irq_restore_ch3_pending
        CALL AudioVBlank_RestoreCh1
        XOR_A
        LD_MEM_A AudioVBlank_RestoreCh1Pending
        LD_MEM_A Audio_EffectUsesCh2

audiovb_irq_restore_ch3_pending:
        LD_A_MEM AudioVBlank_RestoreCh3Pending
        OR_A
        JR_Z audiovb_irq_restore_pending_done
        LD_A_MEM Audio_EffectPointer3
        OR_A
        JR_NZ audiovb_irq_restore_pending_done
        LD_A_MEM Audio_EffectPointer3+1
        OR_A
        JR_NZ audiovb_irq_restore_pending_done
        CALL AudioVBlank_RestoreCh3
        XOR_A
        LD_MEM_A AudioVBlank_RestoreCh3Pending

// Recover released effect channels before consuming the frame delay, so a
// suppressed music event need not wait for the next scheduled stream record.
audiovb_irq_restore_pending_done:

        LD_A_MEM AudioVBlank_MusicDelay
        OR_A
        JR_Z audiovb_irq_step
        DEC_A
        LD_MEM_A AudioVBlank_MusicDelay
        JP audiovb_irq_done

audiovb_irq_step:
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JR_NZ audiovb_irq_queue_mode
        JP audiovb_irq_pointer_stream
audiovb_irq_queue_mode:
        LD_A_MEM AudioVBlank_QueueCount
        OR_A
        JR_NZ audiovb_irq_queue_ready
        LD_A_MEM AudioVBlank_QueueUnderruns
        INC_A
        LD_MEM_A AudioVBlank_QueueUnderruns
        JP audiovb_irq_done

// Claim one queued record and compute its address as base + index * 5.
// Wrap the read index modulo 16 before decoding the record.
audiovb_irq_queue_ready:
        DEC_A
        LD_MEM_A AudioVBlank_QueueCount
        LD_A_MEM AudioVBlank_QueueReadIndex
        LD_E_A
        LD_D_IMM 0
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_DE
        LD_DE_IMM AudioVBlank_QueueBuffer
        ADD_HL_DE
        LD_A_MEM AudioVBlank_QueueReadIndex
        INC_A
        AND_IMM 0x0F
        LD_MEM_A AudioVBlank_QueueReadIndex
        LD_A_HL
        INC_HL
        CP_IMM 0xFC
        JP_NZ audiovb_irq_queue_not_control
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlCommand
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg0
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg1
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg2
// Dispatch predecoded control arguments. These use hardware-ready encodings,
// not the legacy API duty/pan representation; unrecognized controls are skipped.
audiovb_irq_control_apply:
        LD_A_MEM AudioVBlank_ControlCommand
        CP_IMM 0x01
        JR_Z audiovb_irq_control_reset
        JP audiovb_irq_control_not_reset
// Reset instrument and mix defaults, install the triangle wave, and defer
// the first timed record by the legacy startup delay.
audiovb_irq_control_reset:
        LD_A_IMM 0x80
        LD_MEM_A AudioVBlank_Ch1Duty
        LD_MEM_A AudioVBlank_Ch2Duty
        LD_A_IMM 0xF1
        LD_MEM_A AudioVBlank_Ch1Envelope
        LD_MEM_A AudioVBlank_Ch4Envelope
        LD_A_IMM 0xF2
        LD_MEM_A AudioVBlank_Ch2Envelope
        XOR_A
        LD_MEM_A AudioVBlank_Ch1Sweep
        LD_MEM_A Audio_Ch1Sweep
        LD_A_IMM 0x20
        LD_MEM_A AudioVBlank_Ch3Level
        LD_A_IMM 0xFD
        LD_MEM_A AudioVBlank_Ch1LatchedNote
        LD_MEM_A AudioVBlank_Ch3LatchedNote
        LD_A_IMM 0xFF
        LD_MEM_A Audio_Pan
        LDH_MEM_A 0x25
        CALL audiovb_irq_load_wave0
        /* Legacy PlayMusic resets the wave before its first service tick. */
        LD_A_IMM 0x02
        LD_MEM_A AudioVBlank_MusicDelay
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JP_Z audiovb_irq_store_ptr
        JP audiovb_irq_done

audiovb_irq_control_not_reset:
        CP_IMM 0x11
        JR_Z audiovb_irq_control_inst
        JP audiovb_irq_control_not_inst
audiovb_irq_control_inst:
        LD_A_MEM AudioVBlank_ControlArg0
        OR_A
        JR_NZ audiovb_irq_control_inst_not_ch1
        LD_A_MEM AudioVBlank_ControlArg1
        LD_MEM_A AudioVBlank_Ch1Duty
        LD_A_MEM AudioVBlank_ControlArg2
        LD_MEM_A AudioVBlank_Ch1Envelope
        JP audiovb_irq_control_done
audiovb_irq_control_inst_not_ch1:
        CP_IMM 0x01
        JR_NZ audiovb_irq_control_inst_not_ch2
        LD_A_MEM AudioVBlank_ControlArg1
        LD_MEM_A AudioVBlank_Ch2Duty
        LD_A_MEM AudioVBlank_ControlArg2
        LD_MEM_A AudioVBlank_Ch2Envelope
        JP audiovb_irq_control_done
audiovb_irq_control_inst_not_ch2:
        CP_IMM 0x02
        JR_NZ audiovb_irq_control_inst_ch3
        LD_A_MEM AudioVBlank_ControlArg2
        LD_MEM_A AudioVBlank_Ch4Envelope
        JP audiovb_irq_control_done
audiovb_irq_control_inst_ch3:
        LD_A_MEM AudioVBlank_ControlArg2
        LD_MEM_A AudioVBlank_Ch3Level
        JP audiovb_irq_control_done

audiovb_irq_control_not_inst:
        CP_IMM 0x12
        JP_NZ audiovb_irq_control_not_pan
        LD_A_MEM AudioVBlank_ControlArg0
        LD_MEM_A Audio_Pan
        LDH_MEM_A 0x25
        JP audiovb_irq_control_done
audiovb_irq_control_not_pan:
        CP_IMM 0x13
        JP_NZ audiovb_irq_control_not_sweep
        LD_A_MEM AudioVBlank_ControlArg0
        LD_MEM_A AudioVBlank_Ch1Sweep
        LD_MEM_A Audio_Ch1Sweep
        JP audiovb_irq_control_done
// This ISR supports two preset wave payloads: zero selects triangle, and any
// nonzero wave argument selects the saw preset.
audiovb_irq_control_not_sweep:
        CP_IMM 0x14
        JP_NZ audiovb_irq_control_not_wave
        LD_A_MEM AudioVBlank_ControlArg0
        OR_A
        JR_NZ audiovb_irq_control_wave_not_0
        CALL audiovb_irq_load_wave0
        JP audiovb_irq_control_done
audiovb_irq_control_wave_not_0:
        CALL audiovb_irq_load_wave1
        JP audiovb_irq_control_done

audiovb_irq_control_not_wave:
        CP_IMM 0x17
        JP_NZ audiovb_irq_control_not_master
        LD_A_MEM AudioVBlank_ControlArg0
        LDH_MEM_A 0x24
        JP audiovb_irq_control_done
audiovb_irq_control_not_master:
        CP_IMM 0x18
        JP_NZ audiovb_irq_control_not_ch1_duty
        LD_A_MEM AudioVBlank_ControlArg0
        LD_MEM_A AudioVBlank_Ch1Duty
        JP audiovb_irq_control_done
audiovb_irq_control_not_ch1_duty:
        CP_IMM 0x19
        JP_NZ audiovb_irq_control_not_ch2_duty
        LD_A_MEM AudioVBlank_ControlArg0
        LD_MEM_A AudioVBlank_Ch2Duty
        JP audiovb_irq_control_done
audiovb_irq_control_not_ch2_duty:
        CP_IMM 0x1A
        JR_NZ audiovb_irq_control_done
        LD_A_MEM AudioVBlank_ControlArg0
        LD_MEM_A AudioVBlank_Ch3Level

// Control records do not consume a timed music step. Continue with another
// available queue record, or with the following direct-stream record.
audiovb_irq_control_done:
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JR_Z audiovb_irq_pointer_control_done
        LD_A_MEM AudioVBlank_QueueCount
        OR_A
        JP_NZ audiovb_irq_queue_ready
        JP audiovb_irq_done
audiovb_irq_pointer_control_done:
        /* Pointer streams keep HL on the byte after the control arguments.
           Apply the following timed record in this same VBlank. */
        JP audiovb_irq_read_record

audiovb_irq_queue_not_control:
        CP_IMM 0xFF
        JR_NZ audiovb_irq_queue_maybe_immediate
        /* Queue-mode end markers stop only after every earlier record. */
        XOR_A
        LD_MEM_A AudioVBlank_MusicPlaying
        LDH_MEM_A 0x12
        LDH_MEM_A 0x17
        LDH_MEM_A 0x1C
        LDH_MEM_A 0x1A
        LDH_MEM_A 0x21
        JP audiovb_irq_done

audiovb_irq_queue_maybe_immediate:
        CP_IMM 0xFB
        JR_NZ audiovb_irq_queue_regular
        LD_MEM_A AudioVBlank_ControlCommand
        XOR_A
        JP audiovb_irq_active
audiovb_irq_queue_regular:
        LD_C_A
        XOR_A
        LD_MEM_A AudioVBlank_ControlCommand
        LD_A_C
        JP audiovb_irq_active

// Load a directly addressable cursor. The stream must contain valid records;
// loop/control chains have no byte limit and must eventually yield or stop.
audiovb_irq_pointer_stream:
        LD_A_MEM AudioVBlank_MusicPointer
        LD_L_A
        LD_A_MEM AudioVBlank_MusicPointer+1
        LD_H_A
audiovb_irq_read_record:
        LD_A_HL
        INC_HL
        CP_IMM 0xFE
        JR_NZ audiovb_irq_not_loop
        LD_A_MEM AudioVBlank_MusicLoopPoint
        LD_L_A
        LD_A_MEM AudioVBlank_MusicLoopPoint+1
        LD_H_A
        JR audiovb_irq_read_record

audiovb_irq_not_loop:
        CP_IMM 0xFF
        JR_NZ audiovb_irq_pointer_maybe_control
        XOR_A
        LD_MEM_A AudioVBlank_MusicPlaying
        LDH_MEM_A 0x12
        LDH_MEM_A 0x17
        LDH_MEM_A 0x1C
        LDH_MEM_A 0x1A
        LDH_MEM_A 0x21
        JP audiovb_irq_store_ptr

audiovb_irq_pointer_maybe_control:
        CP_IMM 0xFC
        JR_NZ audiovb_irq_active
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlCommand
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg0
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg1
        INC_HL
        LD_A_HL
        LD_MEM_A AudioVBlank_ControlArg2
        INC_HL
        JP audiovb_irq_control_apply

// Read the common timed payload in CH1, CH2, CH3, CH4 order and store its delay.
// Queue-only IMMEDIATE records enter here with zero delay and continue this frame.
audiovb_irq_active:
        LD_MEM_A AudioVBlank_MusicDelay
        LD_A_HL
        INC_HL
        LD_MEM_A AudioVBlank_Ch1Note
        LD_A_HL
        INC_HL
        LD_MEM_A AudioVBlank_Ch2Note
        LD_A_HL
        INC_HL
        LD_MEM_A AudioVBlank_Ch3Note
        LD_A_HL
        INC_HL
        LD_MEM_A AudioVBlank_Ch4Param

audiovb_irq_store_ptr:
        /* Queue-mode HL points into the WRAM queue, not into a music stream.
           Writing it back here corrupts an external producer's source cursor. */
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JR_NZ audiovb_irq_store_ptr_done
        LD_A_L
        LD_MEM_A AudioVBlank_MusicPointer
        LD_A_H
        LD_MEM_A AudioVBlank_MusicPointer+1
audiovb_irq_store_ptr_done:
        LD_A_MEM AudioVBlank_MusicPlaying
        OR_A
        JP_Z audiovb_irq_done
        /* Match the legacy polling driver's APU frame-sequencer phase while
           keeping the trigger itself inside the VBlank service window.
           Queue-only games near their VBlank budget may reduce the wait count;
           note timing remains VBlank-locked. */
        LD_A_B
        OR_A
        JR_NZ audiovb_irq_legacy_phase_ready
        LD_B_IMM 109
audiovb_irq_legacy_phase_wait:
        DEC_B
        JR_NZ audiovb_irq_legacy_phase_wait
        LD_B_IMM 1
audiovb_irq_legacy_phase_ready:
        LD_A_MEM AudioVBlank_Step
        INC_A
        LD_MEM_A AudioVBlank_Step

        LD_A_MEM AudioVBlank_Ch2Note
        CP_IMM 0xFD
        JR_NZ audiovb_irq_ch2_note
        LD_MEM_A AudioVBlank_Ch2LatchedNote
        LD_A_MEM Audio_EffectUsesCh2
        OR_A
        JR_Z audiovb_irq_ch2_stop_now
        LD_A_MEM Audio_EffectPointer
        LD_E_A
        LD_A_MEM Audio_EffectPointer+1
        OR_E
        JR_Z audiovb_irq_ch2_stop_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh1Pending
        JR audiovb_irq_skip_ch2
audiovb_irq_ch2_stop_now:
        XOR_A
        LDH_MEM_A 0x17
        JR audiovb_irq_skip_ch2
audiovb_irq_ch2_note:
        CP_IMM 0x44
        JR_NC audiovb_irq_skip_ch2
        LD_MEM_A AudioVBlank_Ch2LatchedNote
        LD_C_A
        LD_A_MEM Audio_EffectUsesCh2
        OR_A
        JR_Z audiovb_irq_ch2_note_now
        LD_A_MEM Audio_EffectPointer
        LD_E_A
        LD_A_MEM Audio_EffectPointer+1
        OR_E
        JR_Z audiovb_irq_ch2_note_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh1Pending
        JR audiovb_irq_skip_ch2
audiovb_irq_ch2_note_now:
        LD_A_MEM AudioVBlank_Ch2Duty
        LDH_MEM_A 0x16
        LD_A_MEM AudioVBlank_Ch2Envelope
        LDH_MEM_A 0x17
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x18
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x19

// Apply CH1 after CH2. A pulse effect owns only the selected physical channel;
// new music notes or stops on that channel are latched for deferred recovery.
audiovb_irq_skip_ch2:
        LD_A_MEM AudioVBlank_Ch1Note
        CP_IMM 0xFD
        JR_NZ audiovb_irq_ch1_note
        LD_MEM_A AudioVBlank_Ch1LatchedNote
        LD_A_MEM Audio_EffectUsesCh2
        OR_A
        JR_NZ audiovb_irq_ch1_stop_now
        LD_A_MEM Audio_EffectPointer
        LD_E_A
        LD_A_MEM Audio_EffectPointer+1
        OR_E
        JR_Z audiovb_irq_ch1_stop_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh1Pending
        JR audiovb_irq_maybe_ch3
audiovb_irq_ch1_stop_now:
        XOR_A
        LDH_MEM_A 0x12
        JR audiovb_irq_maybe_ch3
audiovb_irq_ch1_note:
        CP_IMM 0x44
        JR_NC audiovb_irq_maybe_ch3
        LD_MEM_A AudioVBlank_Ch1LatchedNote
        LD_C_A
        LD_A_MEM Audio_EffectUsesCh2
        OR_A
        JR_NZ audiovb_irq_ch1_note_now
        LD_A_MEM Audio_EffectPointer
        LD_E_A
        LD_A_MEM Audio_EffectPointer+1
        OR_E
        JR_Z audiovb_irq_ch1_note_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh1Pending
        JR audiovb_irq_maybe_ch3
audiovb_irq_ch1_note_now:
        LD_A_MEM AudioVBlank_Ch1Sweep
        LDH_MEM_A 0x10
        LD_A_MEM AudioVBlank_Ch1Duty
        LDH_MEM_A 0x11
        LD_A_MEM AudioVBlank_Ch1Envelope
        LDH_MEM_A 0x12
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x13
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x14

// For wave notes, 0xFD means stop, indices below 0x44 trigger, and other values
// hold the current note. An active wave effect postpones note/stop register writes.
audiovb_irq_maybe_ch3:
        LD_A_MEM AudioVBlank_Ch3Note
        CP_IMM 0xFD
        JR_NZ audiovb_irq_ch3_note
        LD_MEM_A AudioVBlank_Ch3LatchedNote
        LD_A_MEM Audio_EffectPointer3
        LD_E_A
        LD_A_MEM Audio_EffectPointer3+1
        OR_E
        JR_Z audiovb_irq_ch3_stop_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh3Pending
        JR audiovb_irq_maybe_noise
audiovb_irq_ch3_stop_now:
        XOR_A
        LDH_MEM_A 0x1A
        JR audiovb_irq_maybe_noise
audiovb_irq_ch3_note:
        CP_IMM 0x44
        JR_NC audiovb_irq_maybe_noise
        LD_MEM_A AudioVBlank_Ch3LatchedNote
        LD_C_A
        LD_A_MEM Audio_EffectPointer3
        LD_E_A
        LD_A_MEM Audio_EffectPointer3+1
        OR_E
        JR_Z audiovb_irq_ch3_note_now
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_RestoreCh3Pending
        JR audiovb_irq_maybe_noise
audiovb_irq_ch3_note_now:
        LD_A_IMM 0x80
        LDH_MEM_A 0x1A
        XOR_A
        LDH_MEM_A 0x1B
        LD_A_MEM AudioVBlank_Ch3Level
        LDH_MEM_A 0x1C
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x1D
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x1E

// Leave CH4 untouched while a noise effect owns it. Otherwise 0xFD stops,
// 0xFF holds, and other values become raw noise polynomial parameters.
audiovb_irq_maybe_noise:
        LD_A_MEM Audio_NoiseEffectActive
        OR_A
        JR_NZ audiovb_irq_done
        LD_A_MEM AudioVBlank_Ch4Param
        CP_IMM 0xFD
        JR_NZ audiovb_irq_ch4_note
        XOR_A
        LDH_MEM_A 0x21
        JR audiovb_irq_done
audiovb_irq_ch4_note:
        CP_IMM 0xFF
        JR_Z audiovb_irq_done
        LD_A_IMM 0x20
        LDH_MEM_A 0x20
        LD_A_MEM AudioVBlank_Ch4Envelope
        LDH_MEM_A 0x21
        LD_A_MEM AudioVBlank_Ch4Param
        LDH_MEM_A 0x22
        LD_A_IMM 0x80
        LDH_MEM_A 0x23

audiovb_irq_done:
        /* IMMEDIATE records preserve repeated notes from one legacy music
           service tick. Consume the following record in this same VBlank. */
        LD_A_MEM AudioVBlank_ControlCommand
        CP_IMM 0xFB
        JR_NZ audiovb_irq_really_done
        XOR_A
        LD_MEM_A AudioVBlank_ControlCommand
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JR_Z audiovb_irq_really_done
        LD_A_MEM AudioVBlank_QueueCount
        OR_A
        JP_NZ audiovb_irq_queue_ready
// Invoke the optional hook through a manually stacked return address. The hook
// must be accessible without changing ROM banks and must return normally; SVBK
// still selects bank 1 until the interrupt epilogue restores the saved value.
audiovb_irq_really_done:
        LD_A_MEM AudioVBlank_FrameHook
        LD_E_A
        LD_A_MEM AudioVBlank_FrameHook+1
        OR_E
        JR_Z audiovb_irq_no_hook
        LD_A_MEM AudioVBlank_FrameHook+1
        LD_H_A
        LD_L_E
        LD_DE_IMM audiovb_irq_hook_return
        PUSH_DE
        JP_HL
audiovb_irq_hook_return:
audiovb_irq_no_hook:
        POP_AF
        LDH_MEM_A 0x70
        POP_HL
        POP_DE
        POP_BC
        POP_AF
        RETI

// Replace all 16 triangle bytes with the wave DAC disabled, then re-enable it.
// This internal ASM helper returns to the control dispatcher without triggering a note.
audiovb_irq_load_wave0:
        XOR_A
        LDH_MEM_A 0x1A
        LD_A_IMM 0x01
        LDH_MEM_A 0x30
        LD_A_IMM 0x23
        LDH_MEM_A 0x31
        LD_A_IMM 0x45
        LDH_MEM_A 0x32
        LD_A_IMM 0x67
        LDH_MEM_A 0x33
        LD_A_IMM 0x89
        LDH_MEM_A 0x34
        LD_A_IMM 0xAB
        LDH_MEM_A 0x35
        LD_A_IMM 0xCD
        LDH_MEM_A 0x36
        LD_A_IMM 0xEF
        LDH_MEM_A 0x37
        LD_A_IMM 0xFE
        LDH_MEM_A 0x38
        LD_A_IMM 0xDC
        LDH_MEM_A 0x39
        LD_A_IMM 0xBA
        LDH_MEM_A 0x3A
        LD_A_IMM 0x98
        LDH_MEM_A 0x3B
        LD_A_IMM 0x76
        LDH_MEM_A 0x3C
        LD_A_IMM 0x54
        LDH_MEM_A 0x3D
        LD_A_IMM 0x32
        LDH_MEM_A 0x3E
        LD_A_IMM 0x10
        LDH_MEM_A 0x3F
        LD_A_IMM 0x80
        LDH_MEM_A 0x1A
        RET

// Replace all 16 saw bytes with the wave DAC disabled, then re-enable it.
audiovb_irq_load_wave1:
        XOR_A
        LDH_MEM_A 0x1A
        LD_A_IMM 0x11
        LDH_MEM_A 0x30
        LD_A_IMM 0x22
        LDH_MEM_A 0x31
        LD_A_IMM 0x33
        LDH_MEM_A 0x32
        LD_A_IMM 0x44
        LDH_MEM_A 0x33
        LD_A_IMM 0x55
        LDH_MEM_A 0x34
        LD_A_IMM 0x66
        LDH_MEM_A 0x35
        LD_A_IMM 0x77
        LDH_MEM_A 0x36
        LD_A_IMM 0x88
        LDH_MEM_A 0x37
        LD_A_IMM 0x99
        LDH_MEM_A 0x38
        LD_A_IMM 0xAA
        LDH_MEM_A 0x39
        LD_A_IMM 0xBB
        LDH_MEM_A 0x3A
        LD_A_IMM 0xCC
        LDH_MEM_A 0x3B
        LD_A_IMM 0xDD
        LDH_MEM_A 0x3C
        LD_A_IMM 0xEE
        LDH_MEM_A 0x3D
        LD_A_IMM 0xFF
        LDH_MEM_A 0x3E
        XOR_A
        LDH_MEM_A 0x3F
        LD_A_IMM 0x80
        LDH_MEM_A 0x1A
        RET
    }
}

// Recover the borrowed pulse channel (CH1 or CH2) from its latched music event.
// Pending value 2, disabled music, or a non-note latch silences it instead.
// The caller must first establish that the effect has released the channel.
void AudioVBlank_RestoreCh1()
{
    __asm {
        LD_A_MEM Audio_EffectUsesCh2
        OR_A
        JR_NZ audiovb_restore_ch2
        LD_A_MEM AudioVBlank_RestoreCh1Pending
        CP_IMM 2
        JR_Z audiovb_restore_ch1_off
        LD_A_MEM AudioVBlank_MusicPlaying
        OR_A
        JR_Z audiovb_restore_ch1_off
        LD_A_MEM AudioVBlank_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch1_off
        LD_A_MEM Audio_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch1_off
        LD_A_MEM AudioVBlank_Ch1LatchedNote
        CP_IMM 0x44
        JR_NC audiovb_restore_ch1_off
        LD_C_A
        LD_A_MEM AudioVBlank_Ch1Sweep
        LDH_MEM_A 0x10
        LD_A_MEM AudioVBlank_Ch1Duty
        LDH_MEM_A 0x11
        LD_A_MEM AudioVBlank_Ch1Envelope
        LDH_MEM_A 0x12
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x13
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x14
        JR audiovb_restore_ch1_done
audiovb_restore_ch1_off:
        XOR_A
        LDH_MEM_A 0x12
audiovb_restore_ch1_done:
        JR audiovb_restore_square_done

audiovb_restore_ch2:
        LD_A_MEM AudioVBlank_RestoreCh1Pending
        CP_IMM 2
        JR_Z audiovb_restore_ch2_off
        LD_A_MEM AudioVBlank_MusicPlaying
        OR_A
        JR_Z audiovb_restore_ch2_off
        LD_A_MEM AudioVBlank_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch2_off
        LD_A_MEM Audio_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch2_off
        LD_A_MEM AudioVBlank_Ch2LatchedNote
        CP_IMM 0x44
        JR_NC audiovb_restore_ch2_off
        LD_C_A
        LD_A_MEM AudioVBlank_Ch2Duty
        LDH_MEM_A 0x16
        LD_A_MEM AudioVBlank_Ch2Envelope
        LDH_MEM_A 0x17
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x18
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x19
        JR audiovb_restore_square_done
audiovb_restore_ch2_off:
        XOR_A
        LDH_MEM_A 0x17
audiovb_restore_square_done:
    }
}

// Recover a latched wave-channel music event after effect release, or disable
// the DAC for a release-only request, disabled music, or a non-note latch.
// The caller owns effect arbitration and clearing the pending flag.
void AudioVBlank_RestoreCh3()
{
    __asm {
        LD_A_MEM AudioVBlank_RestoreCh3Pending
        CP_IMM 2
        JR_Z audiovb_restore_ch3_off
        LD_A_MEM AudioVBlank_MusicPlaying
        OR_A
        JR_Z audiovb_restore_ch3_off
        LD_A_MEM AudioVBlank_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch3_off
        LD_A_MEM Audio_MusicEnabled
        OR_A
        JR_Z audiovb_restore_ch3_off
        LD_A_MEM AudioVBlank_Ch3LatchedNote
        CP_IMM 0x44
        JR_NC audiovb_restore_ch3_off
        LD_C_A
        LD_A_IMM 0x80
        LDH_MEM_A 0x1A
        XOR_A
        LDH_MEM_A 0x1B
        LD_A_MEM AudioVBlank_Ch3Level
        LDH_MEM_A 0x1C
        LD_A_C
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM AudioVBlank_FreqLo
        ADD_HL_DE
        LD_A_HL
        LDH_MEM_A 0x1D
        LD_HL_IMM AudioVBlank_FreqHi
        ADD_HL_DE
        LD_A_HL
        OR_IMM 0x80
        LDH_MEM_A 0x1E
        JR audiovb_restore_ch3_done
audiovb_restore_ch3_off:
        XOR_A
        LDH_MEM_A 0x1A
audiovb_restore_ch3_done:
    }
}

// Called when an SFX stream ends.  Do not overwrite value 1: the ISR set it
// because a genuinely new BGM event was suppressed while the effect played.
// Otherwise request a VBlank-only release without replaying an old note.
void AudioVBlank_RequestRestoreCh1()
{
    if (AudioVBlank_RestoreCh1Pending == 0)
        AudioVBlank_RestoreCh1Pending = 2;
}

// Request a deferred wave-channel release only if no recovery is pending.
// Preserve value 1, which records a new music event suppressed by the effect.
void AudioVBlank_RequestRestoreCh3()
{
    if (AudioVBlank_RestoreCh3Pending == 0)
        AudioVBlank_RestoreCh3Pending = 2;
}

#pragma fixed_bank -1

#ifndef AUDIO_VBLANK_CORE_ONLY
#include "audio_vblank_control.inc"
#include "audio_vblank_queue.inc"
#endif
