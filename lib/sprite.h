#pragma once

// Keep the default 40 for the standard DMA path. Hardware always transfers 160 bytes,
// even if this array is shortened; a custom layout must provide valid bytes for that full transfer.
#ifndef SPRITE_MAX
#define SPRITE_MAX 40
#endif

#define SPRITE_FLAG_PAL1     ((u8)0x10)
#define SPRITE_FLAG_XFLIP    ((u8)0x20)
#define SPRITE_FLAG_YFLIP    ((u8)0x40)
#define SPRITE_FLAG_PRIORITY ((u8)0x80)

// Packed hardware order is Y, X, tile, flags; screen coordinates need the GB OAM offsets.
typedef __packed struct {
    u8 y;
    u8 x;
    u8 tile;
    u8 flags;
} SpriteOamEntry;

extern __wram __aligned(256) SpriteOamEntry kq_sprite_oam[SPRITE_MAX];
extern u8 kq_sprite_active[SPRITE_MAX];
extern u8 kq_sprite_used;
extern u8 kq_sprite_height;

// Signed pixel displacements from a common origin; final byte coordinates wrap rather than clip.
typedef __packed struct {
    s8 dx;
    s8 dy;
    u8 tile;
    u8 flags;
} MetaSpritePart;

// Initialize frame/ticks within their configured ranges. This animation selects consecutive tile IDs;
// it does not change sprite size or automatically step by two tiles in 8x16 mode.
typedef __packed struct {
    u8 first_tile;
    u8 frame_count;
    u8 frame;
    u8 ticks;
    u8 ticks_per_frame;
} SpriteAnim;

void __wait_vblank();
void __oam_dma(u16 src_ptr);

// Clear allocation and shadow OAM state and reset the DMA-stub installation flag.
// The default height is eight pixels; the hardware display mode is configured elsewhere.
void sprite_init();
// Claim the first free slot, hide it, and return its ID; return 0xFF when full.
// Previously stored tile/flag bytes are retained until the caller replaces them.
u8 sprite_alloc();
// Release an allocated valid slot and hide it. Repeated frees do not decrement the count.
void sprite_free(u8 id);
// Convert screen coordinates to OAM coordinates by adding X=8 and Y=16.
// The byte additions wrap; this updates shadow storage even for an unallocated slot.
void sprite_set_pos(u8 id, u8 x, u8 y);
// Set a valid shadow slot's tile byte; allocation and hardware transfer are separate.
void sprite_set_tile(u8 id, u8 tile);
// Replace a valid shadow slot's complete hardware attribute byte.
void sprite_set_flags(u8 id, u8 flags);
// Move a valid shadow slot off screen without freeing it.
void sprite_hide(u8 id);
// Install the HRAM stub lazily and execute OAM DMA with interrupts disabled.
// This routine does not wait for VBlank and always enables interrupts afterward;
// it does not restore the caller's previous interrupt-enable state.
void sprite_flush_oam_now();
// Wait for VBlank, then transfer shadow OAM using the immediate DMA routine.
void sprite_flush_oam();
// Return the maintained allocation counter. Metasprite placement also adjusts
// this value as a high-water mark, so mixed allocation styles need care.
u8 sprite_count_used();
// Report whether the software overlap estimate exceeds ten sprites on a scanline.
u8 sprite_warn_scanline_overflow();
// Estimate the largest overlap of active, non-hidden shadow sprites across
// visible scanlines using the configured sprite height. This is a software
// overlap count, not a rendered-pixel or hardware-priority simulation.
u8 sprite_max_scanline_count();
// Place consecutive parts starting at first_id and activate their slots.
// Return the number written if the slot limit is reached. Callers must keep
// ID arithmetic within the byte range and manage overlap with existing allocations.
u8 metasprite_draw(u8 first_id, u8 x, u8 y, const MetaSpritePart* parts, u8 count);
// Advance animation ticks/frame with wrapping frame selection and return the
// resulting tile. Zero ticks-per-frame freezes animation; null state returns zero.
u8 anim_update(SpriteAnim* anim);
