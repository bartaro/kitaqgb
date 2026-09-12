#pragma once

#ifndef VRAM_QUEUE_MAX
#define VRAM_QUEUE_MAX 32
#endif

#define VRAM_CMD_BG_TILE   ((u8)1)
#define VRAM_CMD_BG_RECT   ((u8)2)
#define VRAM_CMD_BG_BLOCK  ((u8)3)
#define VRAM_CMD_MEMCPY    ((u8)4)
#define VRAM_CMD_MEMSET    ((u8)5)

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

void vram_init();
void vram_clear_queue();
u8 vram_queue_bg_tile(u8 x, u8 y, u8 tile);
u8 vram_queue_tile(u8 x, u8 y, u8 tile);
u8 vram_queue_bg_rect(u8 x, u8 y, u8 w, u8 h, u8 tile);
u8 vram_queue_bg_block(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src);
u8 vram_queue_memcpy(u16 dst, const void* src, u16 len);
u8 vram_queue_tiles(u16 dst, const void* src, u16 len);
u8 vram_queue_memset(u16 dst, u8 value, u16 len);
void vram_flush_now();
void vram_flush();
u8 vram_get_queue_used();
u8 vram_get_queue_free();
u8 vram_get_overflowed();
