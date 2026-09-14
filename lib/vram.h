#pragma once

// Keep this capacity in 1..255 in all units; counts and capacity/free return values are bytes.
#ifndef VRAM_QUEUE_MAX
#define VRAM_QUEUE_MAX 32
#endif

#define VRAM_CMD_BG_TILE   ((u8)1)
#define VRAM_CMD_BG_RECT   ((u8)2)
#define VRAM_CMD_BG_BLOCK  ((u8)3)
#define VRAM_CMD_MEMCPY    ((u8)4)
#define VRAM_CMD_MEMSET    ((u8)5)

// A command owns metadata only. src is a retained near pointer with no bank number or copied payload.
typedef __packed struct {
    u8 kind;
    u8 x;
    u8 y;
    u8 w;
    u8 h;
    u8 value;
    u16 dst;
    const u8* src;
    u16 len;
} VramCommand;

extern VramCommand kq_vram_queue[VRAM_QUEUE_MAX];
extern u8 kq_vram_queue_used;
extern u8 kq_vram_queue_overflowed;

void __wait_vblank();
void __settile_xy(u8 x, u8 y, u8 tile);
void __settile_rect(u8 x, u8 y, u8 w, u8 h, u8 tile);
void __settilemap_rect(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src);
void __vram_copy(u16 dst, const void* src, u16 len);
void __vram_fill(u16 dst, u8 value, u16 len);

// Start with an empty command queue and a clear overflow flag.
void vram_init();
// Discard pending commands and clear the latched overflow flag without writing VRAM.
void vram_clear_queue();
// Queue a single background tile write; return zero if no command slot is available.
u8 vram_queue_bg_tile(u8 x, u8 y, u8 tile);
// Provide the short alias for a queued background tile write.
u8 vram_queue_tile(u8 x, u8 y, u8 tile);
// Queue a rectangle filled with one tile value; coordinates and dimensions are in tiles.
u8 vram_queue_bg_rect(u8 x, u8 y, u8 w, u8 h, u8 tile);
// Queue a tile-map rectangle while retaining src rather than copying its bytes.
// Keep the source data and its ROM bank accessible until the queue is flushed.
u8 vram_queue_bg_block(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src);
// Queue a byte transfer and retain the source pointer. The source must remain
// valid and visible until flush; len is measured in bytes, not tiles.
u8 vram_queue_memcpy(u16 dst, const void* src, u16 len);
// Alias the queued byte-copy operation; this does not convert a tile count to bytes.
u8 vram_queue_tiles(u16 dst, const void* src, u16 len);
// Queue a byte fill and return zero if the command queue is full.
u8 vram_queue_memset(u16 dst, u8 value, u16 len);
// Execute pending commands in insertion order without waiting for VBlank here.
// The caller must arrange suitable VRAM access timing and a sufficient transfer budget.
// Only the used count is cleared; overflow stays latched until an explicit reset.
void vram_flush_now();
// Wait for VBlank before executing the queue. Waiting once does not itself limit
// the queued workload to the available VBlank duration.
void vram_flush();
// Return the number of queued commands, not the number of transfer bytes.
u8 vram_get_queue_used();
// Return the total number of command slots, independent of queue usage.
// This is command-buffer capacity, not free space in hardware VRAM.
u8 vram_get_queue_capacity();
// Return the remaining command-slot capacity.
u8 vram_get_queue_free();
// Read the overflow latch without clearing it.
u8 vram_get_overflowed();
