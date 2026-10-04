#include "vram.h"

VramCommand kq_vram_queue[VRAM_QUEUE_MAX];
u8 kq_vram_queue_used;
u8 kq_vram_queue_overflowed;

static VramCommand* vram_push(u8 kind)
{
    VramCommand* cmd;

    if (kq_vram_queue_used >= VRAM_QUEUE_MAX) {
        kq_vram_queue_overflowed = 1;
        return 0;
    }

    cmd = &kq_vram_queue[(__safe_index u8)kq_vram_queue_used];
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

void vram_init()
{
    kq_vram_queue_used = 0;
    kq_vram_queue_overflowed = 0;
}

void vram_clear_queue()
{
    kq_vram_queue_used = 0;
    kq_vram_queue_overflowed = 0;
}

u8 vram_queue_bg_tile(u8 x, u8 y, u8 tile)
{
    VramCommand* cmd = vram_push(VRAM_CMD_BG_TILE);
    if (cmd == 0) return 0;
    cmd->x = x;
    cmd->y = y;
    cmd->value = tile;
    return 1;
}

u8 vram_queue_tile(u8 x, u8 y, u8 tile)
{
    return vram_queue_bg_tile(x, y, tile);
}

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

u8 vram_queue_memcpy(u16 dst, const void* src, u16 len)
{
    VramCommand* cmd = vram_push(VRAM_CMD_MEMCPY);
    if (cmd == 0) return 0;
    cmd->dst = dst;
    cmd->src = (const u8*)src;
    cmd->len = len;
    return 1;
}

u8 vram_queue_tiles(u16 dst, const void* src, u16 len)
{
    return vram_queue_memcpy(dst, src, len);
}

u8 vram_queue_memset(u16 dst, u8 value, u16 len)
{
    VramCommand* cmd = vram_push(VRAM_CMD_MEMSET);
    if (cmd == 0) return 0;
    cmd->dst = dst;
    cmd->value = value;
    cmd->len = len;
    return 1;
}

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

void vram_flush()
{
    __wait_vblank();
    vram_flush_now();
}

u8 vram_get_queue_used()
{
    return kq_vram_queue_used;
}

u8 vram_get_queue_free()
{
    return (u8)(VRAM_QUEUE_MAX - kq_vram_queue_used);
}

u8 vram_get_overflowed()
{
    return kq_vram_queue_overflowed;
}
