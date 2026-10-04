#pragma fixed_bank 1

void AudioVBlank_Init()
{
    AudioVBlank_MusicEnabled = 1;
    AudioVBlank_MusicPlaying = 0;
    AudioVBlank_MusicDelay = 0;
    AudioVBlank_MusicStart = 0;
    AudioVBlank_MusicPointer = 0;
    AudioVBlank_MusicLoopPoint = 0;
    AudioVBlank_Step = 0;
    AudioVBlank_Ch2Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch1Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch3Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch2LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_Ch1LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_Ch3LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_RestoreCh1Pending = 0;
    AudioVBlank_RestoreCh3Pending = 0;
    AudioVBlank_Ch4Param = AUDIO_VBLANK_REST;
    AudioVBlank_Ch1Envelope = 0xF1;
    AudioVBlank_Ch2Envelope = 0xF2;
    AudioVBlank_Ch4Envelope = 0xF1;
    AudioVBlank_Ch1Duty = 0x80;
    AudioVBlank_Ch2Duty = 0x80;
    AudioVBlank_Ch1Sweep = 0x00;
    AudioVBlank_Ch3Level = 0x20;
    AudioVBlank_ControlCommand = 0;
    AudioVBlank_ControlArg0 = 0;
    AudioVBlank_ControlArg1 = 0;
    AudioVBlank_ControlArg2 = 0;
    AudioVBlank_QueueMode = 0;
    AudioVBlank_QueueReadIndex = 0;
    AudioVBlank_QueueWriteIndex = 0;
    AudioVBlank_QueueCount = 0;
    AudioVBlank_QueueUnderruns = 0;
    AudioVBlank_FrameHook = 0;
}

void __stackcall AudioVBlank_SetMusic(u8 *song)
{
    AudioVBlank_MusicStart = song;
    AudioVBlank_MusicPointer = song;
    AudioVBlank_MusicLoopPoint = song;
    AudioVBlank_MusicDelay = 0;
    AudioVBlank_Step = 0;
    AudioVBlank_QueueMode = 0;
}

void AudioVBlank_Play()
{
    if (AudioVBlank_MusicStart == 0) {
        AudioVBlank_Stop();
        return;
    }
    AudioVBlank_MusicPointer = AudioVBlank_MusicStart;
    AudioVBlank_MusicLoopPoint = AudioVBlank_MusicStart;
    AudioVBlank_MusicDelay = 0;
    AudioVBlank_Step = 0;
    AudioVBlank_QueueMode = 0;
    AudioVBlank_MusicPlaying = 1;
}

void __stackcall AudioVBlank_PlayMusic(u8 *song)
{
    AudioVBlank_SetMusic(song);
    AudioVBlank_Play();
}

void AudioVBlank_Stop()
{
    AudioVBlank_MusicPlaying = 0;
    AudioVBlank_MusicDelay = 0;
    AudioVBlank_Step = 0;
    AudioVBlank_Ch2Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch1Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch3Note = AUDIO_VBLANK_REST;
    AudioVBlank_Ch2LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_Ch1LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_Ch3LatchedNote = AUDIO_VBLANK_STOP;
    AudioVBlank_RestoreCh1Pending = 0;
    AudioVBlank_RestoreCh3Pending = 0;
    AudioVBlank_Ch4Param = AUDIO_VBLANK_REST;
    __asm {
        XOR_A
        LDH_MEM_A 0x12
        LDH_MEM_A 0x17
        LDH_MEM_A 0x1C
        LDH_MEM_A 0x1A
        LDH_MEM_A 0x21
    }
}

void __stackcall AudioVBlank_SetEnabled(u8 on)
{
    if (on != 0) AudioVBlank_MusicEnabled = 1;
    else {
        AudioVBlank_MusicEnabled = 0;
        // Do not carry a stale SFX-recovery request into a later song.
        AudioVBlank_RestoreCh1Pending = 0;
        AudioVBlank_RestoreCh3Pending = 0;
    }
}

void AudioVBlank_EnableIrq()
{
    AudioVBlank_IE = (u8)(AudioVBlank_IE | 0x01);
    __asm { EI }
}

void AudioVBlank_DisableIrq()
{
    AudioVBlank_IE = (u8)(AudioVBlank_IE & 0xFE);
}

#pragma fixed_bank -1
