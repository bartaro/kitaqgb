#pragma once

#ifndef ENTITY_MAX
#define ENTITY_MAX ((u8)16)
#endif

typedef __packed struct {
    u8 active;
    u8 type;
    s16 x;
    s16 y;
    s16 vx;
    s16 vy;
    u8 state;
    u8 timer;
    u8 sprite;
} Entity;

typedef void (*EntityFn)(u8 id);

void entity_init();
u8 entity_create(u8 type, s16 x, s16 y);
void entity_destroy(u8 id);
Entity* entity_get(u8 id);
void entity_update_all(EntityFn fn);
void entity_draw_all(EntityFn fn);
u8 entity_count_active();
void entity_clear_all();
