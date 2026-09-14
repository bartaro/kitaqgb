// Play a Game Boy pulse-channel sound effect.
// Expected: 042 and a short pulse tone
#include "gb_common.h"
#include "audio.h"
__prg_rom u8 tone[] = { 24,0xF0,24,0xE0,24,0xC0,24,0xA0,24,0x80,24,0x60,0 };
// Start a pulse-channel effect and service Audio_Update once per frame
// so the effect stream advances while 42 remains on screen.
void main() {
    m_init();
    m_text(2,3,"SOUND");
    Audio_Init(); Audio_PlaySFX(tone,1); m_number(42);
    while (1) { m_wait(); Audio_Update(); }
}
