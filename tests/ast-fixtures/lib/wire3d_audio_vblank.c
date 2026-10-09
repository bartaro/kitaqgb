// Compile instead of audio_vblank.c with DMG/CGB wire3d.
// Reserve 80 bytes fixed WRAM and ROM bank 5 for control functions.
#define AUDIO_VBLANK_QUEUE_WRAM0 1
#define AUDIO_VBLANK_CONTROL_BANK5 1
#include "audio_vblank.c"
