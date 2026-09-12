#include "sprite.h"

__wram __aligned(256) SpriteOamEntry kq_sprite_oam[SPRITE_MAX];
u8 kq_sprite_active[SPRITE_MAX];
u8 kq_sprite_used;
u8 kq_sprite_height;
static __hram u8 kq_sprite_dma_stub[10];
static u8 kq_sprite_dma_ready;

typedef void (*SpriteDmaRoutine)();

static u8 sprite_valid(u8 id)
{
    return (u8)(id < SPRITE_MAX);
}

void sprite_init()
{
    u8 i = 0;
    kq_sprite_used = 0;
    kq_sprite_height = 8;
    kq_sprite_dma_ready = 0;
    while (i < SPRITE_MAX) {
        kq_sprite_active[i] = 0;
        kq_sprite_oam[i].y = 0;
        kq_sprite_oam[i].x = 0;
        kq_sprite_oam[i].tile = 0;
        kq_sprite_oam[i].flags = 0;
        i++;
    }
}

static void sprite_install_dma_stub()
{
    kq_sprite_dma_stub[0] = 0x3E;
    kq_sprite_dma_stub[1] = (u8)(((u16)kq_sprite_oam) >> 8);
    kq_sprite_dma_stub[2] = 0xE0;
    kq_sprite_dma_stub[3] = 0x46;
    kq_sprite_dma_stub[4] = 0x06;
    kq_sprite_dma_stub[5] = 40;
    kq_sprite_dma_stub[6] = 0x05;
    kq_sprite_dma_stub[7] = 0x20;
    kq_sprite_dma_stub[8] = 0xFD;
    kq_sprite_dma_stub[9] = 0xC9;
    kq_sprite_dma_ready = 1;
}

u8 sprite_alloc()
{
    u8 i = 0;
    while (i < SPRITE_MAX) {
        if (kq_sprite_active[i] == 0) {
            kq_sprite_active[i] = 1;
            kq_sprite_used++;
            sprite_hide(i);
            return i;
        }
        i++;
    }
    return 0xFF;
}

#ifndef KQ_SPRITE_MINIMAL_RUNTIME
void sprite_free(u8 id)
{
    if (sprite_valid(id) == 0) return;
    if (kq_sprite_active[id] == 0) return;
    kq_sprite_active[id] = 0;
    if (kq_sprite_used != 0) kq_sprite_used--;
    sprite_hide(id);
}

void sprite_set_pos(u8 id, u8 x, u8 y)
{
    if (sprite_valid(id) == 0) return;
    kq_sprite_oam[id].x = (u8)(x + 8);
    kq_sprite_oam[id].y = (u8)(y + 16);
}
#endif

void sprite_set_tile(u8 id, u8 tile)
{
    if (sprite_valid(id) == 0) return;
    kq_sprite_oam[id].tile = tile;
}

void sprite_set_flags(u8 id, u8 flags)
{
    if (sprite_valid(id) == 0) return;
    kq_sprite_oam[id].flags = flags;
}

void sprite_hide(u8 id)
{
    if (sprite_valid(id) == 0) return;
    kq_sprite_oam[id].y = 0;
    kq_sprite_oam[id].x = 0;
}

void sprite_flush_oam_now()
{
    SpriteDmaRoutine dma;
    if (kq_sprite_dma_ready == 0) {
        sprite_install_dma_stub();
    }
    dma = (SpriteDmaRoutine)kq_sprite_dma_stub;
    __asm {
        DI
    }
    dma();
    __asm {
        EI
    }
}

void sprite_flush_oam()
{
    __wait_vblank();
    sprite_flush_oam_now();
}

#ifndef KQ_SPRITE_MINIMAL_RUNTIME
u8 sprite_count_used()
{
    return kq_sprite_used;
}

u8 sprite_max_scanline_count()
{
    u8 line = 0;
    u8 best = 0;
    while (line < 144) {
        u8 i = 0;
        u8 count = 0;
        u8 ly_oam = (u8)(line + 16);
        while (i < SPRITE_MAX) {
            u8 sy = kq_sprite_oam[i].y;
            if (kq_sprite_active[i] != 0 && sy != 0) {
                if (ly_oam >= sy && ly_oam < (u8)(sy + kq_sprite_height)) {
                    count++;
                }
            }
            i++;
        }
        if (count > best) best = count;
        line++;
    }
    return best;
}

u8 sprite_warn_scanline_overflow()
{
    return (u8)(sprite_max_scanline_count() > 10);
}

u8 metasprite_draw(u8 first_id, u8 x, u8 y, const MetaSpritePart* parts, u8 count)
{
    u8 i = 0;
    while (i < count) {
        u8 id = (u8)(first_id + i);
        if (id >= SPRITE_MAX) return i;
        kq_sprite_active[id] = 1;
        sprite_set_pos(id, (u8)(x + parts[i].dx), (u8)(y + parts[i].dy));
        sprite_set_tile(id, parts[i].tile);
        sprite_set_flags(id, parts[i].flags);
        i++;
    }
    if ((u8)(first_id + count) > kq_sprite_used) kq_sprite_used = (u8)(first_id + count);
    return count;
}

u8 anim_update(SpriteAnim* anim)
{
    if (anim == 0) return 0;
    if (anim->frame_count == 0) return anim->first_tile;
    if (anim->ticks_per_frame == 0) return (u8)(anim->first_tile + anim->frame);

    anim->ticks++;
    if (anim->ticks >= anim->ticks_per_frame) {
        anim->ticks = 0;
        anim->frame++;
        if (anim->frame >= anim->frame_count) anim->frame = 0;
    }
    return (u8)(anim->first_tile + anim->frame);
}
#endif
