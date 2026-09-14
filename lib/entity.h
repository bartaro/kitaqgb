#pragma once

// Keep capacity in 1..255: IDs and scan counters are bytes and FF is reserved for failure.
// Use the same definition in every compilation unit.
#ifndef ENTITY_MAX
#define ENTITY_MAX ((u8)16)
#endif

// Position, velocity, state and timer units are game-defined; the pool does not advance them.
// The sprite field is only bookkeeping and does not allocate or release OAM entries.
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

// An entity callback receives a slot ID, not a persistent object handle.
typedef void (*EntityFn)(u8 id);

// Reset every slot before the game starts using the pool.
void entity_init();
// Returns a reusable slot ID, or 0xFF if no slot is free.
u8 entity_create(u8 type, s16 x, s16 y);
// Release a valid slot without clearing its payload. Reallocation resets the payload;
// a saved pointer or ID must not be treated as a persistent entity identity.
void entity_destroy(u8 id);
// Returns null for an invalid or inactive ID; the returned pointer aliases reusable pool storage.
Entity* entity_get(u8 id);
// Call fn for each slot that is active when the scan reaches it; null is a no-op.
// Callbacks may change the pool, so newly activated later slots can run in this pass.
void entity_update_all(EntityFn fn);
// Use the same active-slot traversal for drawing; the callback supplies all rendering.
void entity_draw_all(EntityFn fn);
// Count occupied slots without changing entity state.
u8 entity_count_active();
// Release every entity and restore deterministic defaults, including the no-sprite sentinel.
void entity_clear_all();
