#pragma once

// This callback runs in the caller of system_wait_vblank, not as an installed ISR.
// Its plain pointer has no ROM bank component; keep the target callable in that mapping.
typedef void (*SystemCallback)();

extern u16 kq_system_frame;

void __wait_vblank();

// Reset the software frame counter and remove any registered callback.
void system_init();
// Wait for VBlank, count this wrapper call, then invoke the callback synchronously.
// The counter wraps at 16 bits and is not an independent hardware frame clock.
void system_wait_vblank();
// Replace the callback used by system_wait_vblank; null disables the callback.
void system_set_vblank_callback(SystemCallback callback);
// Read the software count of completed system_wait_vblank calls.
u16 system_get_frame();
// Return the low byte of the software frame counter for wrapping 8-bit timing.
u8 system_get_frame8();
// Enable CPU interrupts and return using the inline-assembly function body.
void system_enable_interrupts();
// Disable CPU interrupts and return; callers must explicitly re-enable them later.
void system_disable_interrupts();

#define gb_init system_init
#define gb_wait_vblank system_wait_vblank
#define gb_set_vblank_callback system_set_vblank_callback
#define gb_frame_count system_get_frame
#define gb_enable_interrupts system_enable_interrupts
#define gb_disable_interrupts system_disable_interrupts
