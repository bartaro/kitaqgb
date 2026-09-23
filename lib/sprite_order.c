// Copyright (c) 2026 DAISUKE OBA. MIT License.
#include "sprite_order.h"

// Initialize every state field before accepting storage, so failed setup cannot
// leave an earlier queue active. Item bytes are initialized as they are pushed.
u8 sprite_order_init(SpriteOrder* order, SpriteOrderItem* items, u8 capacity, u8 height)
{
    if (order == 0) return 0;
    order->items = 0;
    order->capacity = 0;
    order->count = 0;
    order->height = 8;
    order->phase = 0;
    if (items == 0 || capacity == 0) return 0;
    if (height != 8 && height != 16) return 0;
    order->items = items;
    order->capacity = capacity;
    order->height = height;
    return 1;
}

// Submission order is rebuilt per frame; fairness continues from the prior
// frame instead of restarting with the same low-index objects every time.
void sprite_order_begin(SpriteOrder* order)
{
    if (order != 0) order->count = 0;
}

// Clip in signed screen space before adding OAM offsets. Hiding solely by X
// would still let off-screen objects consume the hardware's scanline budget.
u8 sprite_order_push(SpriteOrder* order, s16 x, s16 y, u8 tile, u8 flags, u8 priority)
{
    SpriteOrderItem* item;
    if (order == 0) return 0;
    if (order->items == 0 || order->capacity == 0) return 0;
    if (order->height != 8 && order->height != 16) return 0;
    if (order->count >= order->capacity || priority >= SPRITE_ORDER_LEVELS) return 0;
    if (x <= -8 || x >= 160 || y >= 144) return 0;
    if (order->height == 16) {
        if (y <= -16) return 0;
    } else {
        if (y <= -8) return 0;
    }
    item = &order->items[order->count];
    item->oam.y = (u8)(y + 16);
    item->oam.x = (u8)(x + 8);
    item->oam.tile = tile;
    item->oam.flags = flags;
    item->priority = priority;
    order->count++;
    return 1;
}

// Four bounded scans preserve the submission pool and need no sorting buffer.
// Lower-numbered priority bands always precede higher-numbered bands. Within
// each band, the starting position rotates to distribute hardware selection.
u8 sprite_order_build(SpriteOrder* order, SpriteOrderOamEntry* output, u8 limit)
{
    u8 i;
    u8 count;
    u8 phase;
    u8 priority;
    u8 visited;
    u8 written;
    SpriteOrderItem* item;
    if (order == 0 || output == 0) return 0;
    if (order->items == 0 || order->capacity == 0) return 0;
    if (order->count > order->capacity) return 0;
    if (order->height != 8 && order->height != 16) return 0;
    // Clear all hardware slots regardless of the requested emission limit.
    for (i = 0; i < SPRITE_ORDER_OAM_COUNT; i++) {
        output[i].y = 0;
        output[i].x = 0;
        output[i].tile = 0;
        output[i].flags = 0;
    }
    count = order->count;
    if (count == 0) {
        order->phase = 0;
        return 0;
    }
    if (limit > SPRITE_ORDER_OAM_COUNT) limit = SPRITE_ORDER_OAM_COUNT;
    phase = order->phase;
    while (phase >= count) phase = (u8)(phase - count);
    written = 0;
    for (priority = 0; priority < SPRITE_ORDER_LEVELS && written < limit; priority++) {
        i = phase;
        visited = 0;
        while (visited < count && written < limit) {
            item = &order->items[i];
            if (item->priority == priority) {
                output[written].y = item->oam.y;
                output[written].x = item->oam.x;
                output[written].tile = item->oam.tile;
                output[written].flags = item->oam.flags;
                written++;
            }
            visited++;
            i++;
            if (i >= count) i = 0;
        }
    }
    phase++;
    if (phase >= count) phase = 0;
    order->phase = phase;
    return written;
}
