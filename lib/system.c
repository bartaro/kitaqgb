#include "system.h"

u16 kq_system_frame;
static SystemCallback system_vblank_callback;

void system_init()
{
    kq_system_frame = 0;
    system_vblank_callback = 0;
}

void system_wait_vblank()
{
    __wait_vblank();
    kq_system_frame++;
    if (system_vblank_callback != 0) {
        system_vblank_callback();
    }
}

void system_set_vblank_callback(SystemCallback callback)
{
    system_vblank_callback = callback;
}

u16 system_get_frame()
{
    return kq_system_frame;
}

u8 system_get_frame8()
{
    return (u8)kq_system_frame;
}

void system_enable_interrupts()
{
    __asm {
        EI
        RET
    }
}

void system_disable_interrupts()
{
    __asm {
        DI
        RET
    }
}
