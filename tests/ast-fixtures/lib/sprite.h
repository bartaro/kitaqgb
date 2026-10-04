#pragma once

#ifndef SPRITE_MAX
#define SPRITE_MAX 40
#endif

#define SPRITE_FLAG_PAL1     ((u8)0x10)
#define SPRITE_FLAG_XFLIP    ((u8)0x20)
#define SPRITE_FLAG_YFLIP    ((u8)0x40)
#define SPRITE_FLAG_PRIORITY ((u8)0x80)

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

typedef __packed struct {
    s8 dx;
    s8 dy;
    u8 tile;
    u8 flags;
} MetaSpritePart;

typedef __packed struct {
    u8 first_tile;
    u8 frame_count;
    u8 frame;
    u8 ticks;
    u8 ticks_per_frame;
} SpriteAnim;

void __wait_vblank();
void __oam_dma(u16 src_ptr);

void sprite_init();
u8 sprite_alloc();
void sprite_free(u8 id);
void sprite_set_pos(u8 id, u8 x, u8 y);
void sprite_set_tile(u8 id, u8 tile);
void sprite_set_flags(u8 id, u8 flags);
void sprite_hide(u8 id);
void sprite_flush_oam_now();
void sprite_flush_oam();
u8 sprite_count_used();
u8 sprite_warn_scanline_overflow();
u8 sprite_max_scanline_count();
u8 metasprite_draw(u8 first_id, u8 x, u8 y, const MetaSpritePart* parts, u8 count);
u8 anim_update(SpriteAnim* anim);
