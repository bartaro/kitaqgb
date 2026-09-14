#include "entity.h"

// Fixed-capacity storage keeps entity addresses stable and avoids heap allocation.
// IDs are reusable slot indices; 0xFF denotes allocation failure or no sprite.
static Entity entity_pool[ENTITY_MAX];

// Reset every slot before the game starts using the pool.
void entity_init()
{
    entity_clear_all();
}

// Release every entity and restore deterministic defaults, including the no-sprite sentinel.
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

// Claim the first inactive slot, initialize its state, and return its index.
// Return 0xFF when the pool is full; callers must check before using the ID.
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

// Release a valid slot without clearing its payload. Reallocation resets the payload;
// a saved pointer or ID must not be treated as a persistent entity identity.
void entity_destroy(u8 id)
{
    if (id >= ENTITY_MAX) return;
    // Deactivation only releases this pool slot; the caller handles sprite/resource cleanup.
    entity_pool[id].active = 0;
}

// Return a pointer only for a currently active slot. Invalid or inactive IDs return null.
// The pointer aliases pool storage and may refer to a different entity after slot reuse.
Entity* entity_get(u8 id)
{
    if (id >= ENTITY_MAX) return 0;
    if (entity_pool[id].active == 0) return 0;
    return &entity_pool[(__safe_index u8)id];
}

// Call fn for each slot that is active when the scan reaches it; null is a no-op.
// Callbacks may change the pool, so newly activated later slots can run in this pass.
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

// Use the same active-slot traversal for drawing; the callback supplies all rendering.
void entity_draw_all(EntityFn fn)
{
    entity_update_all(fn);
}

// Count occupied slots without changing entity state.
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
