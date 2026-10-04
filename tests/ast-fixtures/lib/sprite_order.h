#pragma once
// Copyright (c) 2026 DAISUKE OBA. MIT License.

// Hardware-compatible layout without requiring the sprite allocator or its
// global storage. Cast a sprite.h shadow buffer to this type when combining
// the libraries; both layouts contain exactly Y, X, tile and flags bytes.
typedef __packed struct {
    u8 y;
    u8 x;
    u8 tile;
    u8 flags;
} SpriteOrderOamEntry;

#define SPRITE_ORDER_LEVELS 4
#define SPRITE_ORDER_OAM_COUNT 40

// One submitted, visible sprite. Attributes retain their hardware meaning;
// priority 0..3 controls OAM selection order, not the BG-over-OBJ flag.
typedef __packed struct {
    SpriteOrderOamEntry oam;
    u8 priority;
} SpriteOrderItem;

// Caller-owned foreground state. Capacity is 1..255 items; count includes only
// accepted submissions. Rebuild the same logical submission list each frame so
// phase rotation distributes selection among equal-priority sprites.
typedef struct {
    SpriteOrderItem* items;
    u8 capacity;
    u8 count;
    u8 height;
    u8 phase;
} SpriteOrder;

// Bind writable item storage and select the hardware sprite height (8 or 16).
// Return 1 on success. Invalid storage, zero capacity or another height returns
// 0 and leaves a non-null state disabled. A null state returns 0 without writes.
u8 sprite_order_init(SpriteOrder* order, SpriteOrderItem* items, u8 capacity, u8 height);

// Discard this frame's submissions while preserving storage, height and phase.
// A null state is ignored. Call once before submitting the next frame's sprites.
void sprite_order_begin(SpriteOrder* order);

// Submit one sprite at signed, top-left SCREEN pixel coordinates. Preserve the
// tile and attribute bytes; use priority 0 (first) through 3 (last). Fully
// off-screen sprites, invalid priorities, disabled state and full storage return
// 0 without consuming a slot. Partly visible sprites are accepted. Return 1 on
// acceptance. Hardware 8x16 mode still ignores the tile index's low bit.
u8 sprite_order_push(SpriteOrder* order, s16 x, s16 y, u8 tile, u8 flags, u8 priority);

// Write a complete 160-byte shadow OAM: ascending priority, then cyclic order
// within each priority. Clamp limit to 40, return the number emitted and zero
// all unused entries (including Y) so they cannot consume scanline slots.
// Advance phase once for a nonempty valid queue, even when limit is zero.
// Empty queues reset phase to zero. Null/disabled/corrupt state or null output
// returns 0 without modifying output or phase.
// Output must provide 40 entries and must not overlap item/state storage.
// This only writes RAM. Build in foreground, then transfer during VBlank.
// DMG overlap priority also depends on X: this cannot override that hardware
// rule. OAM order chooses the first ten sprites per scanline on both DMG/CGB.
// It neither updates sprite_alloc's bookkeeping nor performs OAM DMA.
u8 sprite_order_build(SpriteOrder* order, SpriteOrderOamEntry* output, u8 limit);
