#include "system.h"

u16 kq_system_frame;
static SystemCallback system_vblank_callback;

// Reset the software frame counter and remove any registered callback.
void system_init()
{
    kq_system_frame = 0;
    system_vblank_callback = 0;
}

// Wait for VBlank, count this wrapper call, then invoke the callback synchronously.
// The counter wraps at 16 bits and is not an independent hardware frame clock.
void system_wait_vblank()
{
    __wait_vblank();
    kq_system_frame++;
    if (system_vblank_callback != 0) {
        system_vblank_callback();
    }
}

// Replace the callback used by system_wait_vblank; null disables the callback.
void system_set_vblank_callback(SystemCallback callback)
{
    system_vblank_callback = callback;
}

// Read the software count of completed system_wait_vblank calls.
u16 system_get_frame()
{
    return kq_system_frame;
}

// Return the low byte of the software frame counter for wrapping 8-bit timing.
u8 system_get_frame8()
{
    return (u8)kq_system_frame;
}

// Enable CPU interrupts and return using the inline-assembly function body.
void system_enable_interrupts()
{
    __asm {
        EI
        RET
    }
}

// Disable CPU interrupts and return; callers must explicitly re-enable them later.
void system_disable_interrupts()
{
    __asm {
        DI
        RET
    }
}
