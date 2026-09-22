// Foreground producer for the five-byte VBlank record queue.
// Only one foreground producer may call these routines. The ISR consumes records.
#pragma fixed_bank 1

__wram u8 AudioVBlank_QueueStaging[5];
__wram u8 AudioVBlank_QueueAccepted;

// Publish one staged record with all interrupt sources temporarily masked in IE.
// IME is never changed. No stack access or call occurs while SVBK is switched,
// so this also works with a caller stack or source in another WRAM bank.
static u8 AudioVBlank_QueuePublish()
{
    AudioVBlank_QueueAccepted = 0;
    __asm {
        LD_A_MEM 0xFFFF
        LD_B_A
        XOR_A
        LD_MEM_A 0xFFFF
        LD_A_MEM AudioVBlank_QueueMode
        OR_A
        JR_Z avbq_publish_done
        LD_A_MEM AudioVBlank_QueueCount
        CP_IMM 16
        JR_NC avbq_publish_done
        LD_A_MEM AudioVBlank_QueueWriteIndex
        AND_IMM 15
        LD_E_A
        LD_D_IMM 0
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_DE
        LD_DE_IMM AudioVBlank_QueueBuffer
        ADD_HL_DE
        LDH_A_MEM 0x70
        LD_C_A
        LD_A_IMM 1
        LDH_MEM_A 0x70
        LD_DE_IMM AudioVBlank_QueueStaging
        LD_A_DE
        LDI_HL_A
        INC_DE
        LD_A_DE
        LDI_HL_A
        INC_DE
        LD_A_DE
        LDI_HL_A
        INC_DE
        LD_A_DE
        LDI_HL_A
        INC_DE
        LD_A_DE
        LD_HL_A
        LD_A_C
        LDH_MEM_A 0x70
        LD_A_MEM AudioVBlank_QueueWriteIndex
        INC_A
        AND_IMM 15
        LD_MEM_A AudioVBlank_QueueWriteIndex
        // Count is changed only after the complete payload is visible.
        LD_A_MEM AudioVBlank_QueueCount
        INC_A
        LD_MEM_A AudioVBlank_QueueCount
        LD_A_IMM 1
        LD_MEM_A AudioVBlank_QueueAccepted
avbq_publish_done:
        LD_A_B
        LD_MEM_A 0xFFFF
    }
    return AudioVBlank_QueueAccepted;
}

// Stop current music, select an empty queue and clear its underrun history.
// Keep all enable/pause gates and the caller's interrupt-enable state intact.
void AudioVBlank_QueueReset()
{
    u8 saved_ie;
    saved_ie = AudioVBlank_IE;
    AudioVBlank_IE = 0;
    AudioVBlank_Stop();
    AudioVBlank_QueueMode = 1;
    AudioVBlank_QueueReadIndex = 0;
    AudioVBlank_QueueWriteIndex = 0;
    AudioVBlank_QueueCount = 0;
    AudioVBlank_QueueUnderruns = 0;
    AudioVBlank_IE = saved_ie;
}

// Copy up to sixteen complete records from the caller's currently mapped source.
// The return value is the number accepted, not bytes; advance src by result*5.
// Staging before the bank change supports sources in any selected WRAM bank.
u8 __stackcall AudioVBlank_QueueRefill(const u8 *records, u8 count)
{
    u8 accepted;
    u8 i;
    if (records == 0 || count == 0 || AudioVBlank_QueueMode == 0) return 0;
    if (count > AUDIO_VBLANK_QUEUE_RECORDS) count = AUDIO_VBLANK_QUEUE_RECORDS;
    accepted = 0;
    while (accepted < count) {
        if (AudioVBlank_QueueCount >= AUDIO_VBLANK_QUEUE_RECORDS) break;
        for (i = 0; i < 5; i++) AudioVBlank_QueueStaging[i] = records[i];
        if (AudioVBlank_QueuePublish() == 0) break;
        records += 5;
        accepted++;
    }
    return accepted;
}

// Start/resume a nonempty queue without changing its cursors or delay.
// Enable music and install/enable the VBlank ISR separately. Empty returns zero.
u8 AudioVBlank_QueuePlay()
{
    u8 saved_ie;
    u8 started;
    saved_ie = AudioVBlank_IE;
    AudioVBlank_IE = 0;
    started = 0;
    if (AudioVBlank_QueueMode != 0 && AudioVBlank_QueueCount != 0) {
        AudioVBlank_MusicPlaying = 1;
        started = 1;
    }
    AudioVBlank_IE = saved_ie;
    return started;
}

#pragma fixed_bank -1
