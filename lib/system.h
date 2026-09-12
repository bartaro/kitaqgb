#pragma once

typedef void (*SystemCallback)();

extern u16 kq_system_frame;

void __wait_vblank();

void system_init();
void system_wait_vblank();
void system_set_vblank_callback(SystemCallback callback);
u16 system_get_frame();
u8 system_get_frame8();
void system_enable_interrupts();
void system_disable_interrupts();

#define gb_init system_init
#define gb_wait_vblank system_wait_vblank
#define gb_set_vblank_callback system_set_vblank_callback
#define gb_frame_count system_get_frame
#define gb_enable_interrupts system_enable_interrupts
#define gb_disable_interrupts system_disable_interrupts
