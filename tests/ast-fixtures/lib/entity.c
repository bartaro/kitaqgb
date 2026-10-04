#include "entity.h"

static Entity entity_pool[ENTITY_MAX];

void entity_init()
{
    entity_clear_all();
}

void entity_clear_all()
{
    u8 i = 0;
    while (i < ENTITY_MAX) {
        entity_pool[i].active = 0;
        entity_pool[i].type = 0;
        entity_pool[i].x = 0;
        entity_pool[i].y = 0;
        entity_pool[i].vx = 0;
        entity_pool[i].vy = 0;
        entity_pool[i].state = 0;
        entity_pool[i].timer = 0;
        entity_pool[i].sprite = 0xFF;
        i++;
    }
}

u8 entity_create(u8 type, s16 x, s16 y)
{
    u8 i = 0;
    while (i < ENTITY_MAX) {
        if (entity_pool[i].active == 0) {
            entity_pool[i].active = 1;
            entity_pool[i].type = type;
            entity_pool[i].x = x;
            entity_pool[i].y = y;
            entity_pool[i].vx = 0;
            entity_pool[i].vy = 0;
            entity_pool[i].state = 0;
            entity_pool[i].timer = 0;
            entity_pool[i].sprite = 0xFF;
            return i;
        }
        i++;
    }
    return 0xFF;
}

void entity_destroy(u8 id)
{
    if (id >= ENTITY_MAX) return;
    entity_pool[id].active = 0;
}

Entity* entity_get(u8 id)
{
    if (id >= ENTITY_MAX) return 0;
    if (entity_pool[id].active == 0) return 0;
    return &entity_pool[(__safe_index u8)id];
}

void entity_update_all(EntityFn fn)
{
    u8 i = 0;
    if (fn == 0) return;
    while (i < ENTITY_MAX) {
        if (entity_pool[i].active != 0) {
            fn(i);
        }
        i++;
    }
}

void entity_draw_all(EntityFn fn)
{
    entity_update_all(fn);
}

u8 entity_count_active()
{
    u8 i = 0;
    u8 count = 0;
    while (i < ENTITY_MAX) {
        if (entity_pool[i].active != 0) count++;
        i++;
    }
    return count;
}
