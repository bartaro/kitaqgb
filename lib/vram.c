#include "vram.h"

VramCommand kq_vram_queue[VRAM_QUEUE_MAX];
u8 kq_vram_queue_used;
u8 kq_vram_queue_overflowed;

// Reserve and initialize one command slot. On exhaustion, latch the overflow flag
// and return null without replacing commands already queued.
static VramCommand* vram_push(u8 kind)
{
    VramCommand* cmd;

    if (kq_vram_queue_used >= VRAM_QUEUE_MAX) {
        kq_vram_queue_overflowed = 1;
        return 0;
    }

    cmd = &kq_vram_queue[(__safe_index u8)kq_vram_queue_used];
    // The slot becomes visible before initialization; never let an interrupt flush this queue while it is being built.
    kq_vram_queue_used++;
    cmd->kind = kind;
    cmd->x = 0;
    cmd->y = 0;
    cmd->w = 0;
    cmd->h = 0;
    cmd->value = 0;
    cmd->dst = 0;
    cmd->src = 0;
    cmd->len = 0;
    return cmd;
}

// Start with an empty command queue and a clear overflow flag.
void vram_init()
{
    kq_vram_queue_used = 0;
    kq_vram_queue_overflowed = 0;
}

// Discard pending commands and clear the latched overflow flag without writing VRAM.
void vram_clear_queue()
{
    kq_vram_queue_used = 0;
    kq_vram_queue_overflowed = 0;
}

// Queue a single background tile write; return zero if no command slot is available.
u8 vram_queue_bg_tile(u8 x, u8 y, u8 tile)
{
    VramCommand* cmd = vram_push(VRAM_CMD_BG_TILE);
    if (cmd == 0) return 0;
    cmd->x = x;
    cmd->y = y;
    cmd->value = tile;
    return 1;
}

// Provide the short alias for a queued background tile write.
u8 vram_queue_tile(u8 x, u8 y, u8 tile)
{
    return vram_queue_bg_tile(x, y, tile);
}

// Queue a rectangle filled with one tile value; coordinates and dimensions are in tiles.
u8 vram_queue_bg_rect(u8 x, u8 y, u8 w, u8 h, u8 tile)
{
    VramCommand* cmd = vram_push(VRAM_CMD_BG_RECT);
    if (cmd == 0) return 0;
    cmd->x = x;
    cmd->y = y;
    cmd->w = w;
    cmd->h = h;
    cmd->value = tile;
    return 1;
}

// Queue a tile-map rectangle while retaining src rather than copying its bytes.
// Keep the source data and its ROM bank accessible until the queue is flushed.
u8 vram_queue_bg_block(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src)
{
    VramCommand* cmd = vram_push(VRAM_CMD_BG_BLOCK);
    if (cmd == 0) return 0;
    cmd->dst = base;
    cmd->x = x;
    cmd->y = y;
    cmd->w = w;
    cmd->h = h;
    cmd->src = src;
    return 1;
}

// Queue a byte transfer and retain the source pointer. The source must remain
// valid and visible until flush; len is measured in bytes, not tiles.
u8 vram_queue_memcpy(u16 dst, const void* src, u16 len)
{
    VramCommand* cmd = vram_push(VRAM_CMD_MEMCPY);
    if (cmd == 0) return 0;
    cmd->dst = dst;
    cmd->src = (const u8*)src;
    cmd->len = len;
    return 1;
}

// Alias the queued byte-copy operation; this does not convert a tile count to bytes.
u8 vram_queue_tiles(u16 dst, const void* src, u16 len)
{
    return vram_queue_memcpy(dst, src, len);
}

// Queue a byte fill and return zero if the command queue is full.
u8 vram_queue_memset(u16 dst, u8 value, u16 len)
{
    VramCommand* cmd = vram_push(VRAM_CMD_MEMSET);
    if (cmd == 0) return 0;
    cmd->dst = dst;
    cmd->value = value;
    cmd->len = len;
    return 1;
}

// Execute pending commands in insertion order without waiting for VBlank here.
// The caller must arrange suitable VRAM access timing and a sufficient transfer budget.
// Only the used count is cleared; overflow stays latched until an explicit reset.
// The consumer reads the live used count and clears it at completion. Serialize producers with this whole operation.
void vram_flush_now()
{
    u8 i = 0;
    while (i < kq_vram_queue_used) {
        VramCommand* cmd = &kq_vram_queue[(__safe_index u8)i];
        if (cmd->kind == VRAM_CMD_BG_TILE) {
            __settile_xy(cmd->x, cmd->y, cmd->value);
        } else if (cmd->kind == VRAM_CMD_BG_RECT) {
            __settile_rect(cmd->x, cmd->y, cmd->w, cmd->h, cmd->value);
        } else if (cmd->kind == VRAM_CMD_BG_BLOCK) {
            __settilemap_rect(cmd->dst, cmd->x, cmd->y, cmd->w, cmd->h, cmd->src);
        } else if (cmd->kind == VRAM_CMD_MEMCPY) {
            __vram_copy(cmd->dst, cmd->src, cmd->len);
        } else if (cmd->kind == VRAM_CMD_MEMSET) {
            __vram_fill(cmd->dst, cmd->value, cmd->len);
        }
        i++;
    }
    kq_vram_queue_used = 0;
}

// Wait for VBlank before executing the queue. Waiting once does not itself limit
// the queued workload to the available VBlank duration.
void vram_flush()
{
    __wait_vblank();
    vram_flush_now();
}

// Return the number of queued commands, not the number of transfer bytes.
u8 vram_get_queue_used()
{
    return kq_vram_queue_used;
}

// Return the total number of command slots, independent of queue usage.
// This is command-buffer capacity, not free space in hardware VRAM.
u8 vram_get_queue_capacity()
{
    return VRAM_QUEUE_MAX;
}

// Return the remaining command-slot capacity.
// This subtraction assumes only the queue API changes used and used never exceeds capacity.
u8 vram_get_queue_free()
{
    return (u8)(VRAM_QUEUE_MAX - kq_vram_queue_used);
}

// Read the overflow latch without clearing it.
u8 vram_get_overflowed()
{
    return kq_vram_queue_overflowed;
}
