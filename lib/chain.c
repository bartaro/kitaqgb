#include "chain.h"

// Attach caller-owned ring storage and start with no recorded points.
// The storage must remain valid for the lifetime of the chain.
void chain_init(Chain* chain, ChainPoint* storage, u8 capacity)
{
    chain->points = storage;
    chain->capacity = capacity;
    chain->count = 0;
    chain->head = 0;
}

// Forget the recorded points without clearing or freeing the backing storage.
void chain_clear(Chain* chain)
{
    chain->count = 0;
    chain->head = 0;
}

// Move the ring head backward and store the newest point. Once full, each push
// replaces the oldest point; zero capacity is a no-op.
void chain_push_head(Chain* chain, s16 x, s16 y)
{
    if (chain == 0) return;
    if (chain->capacity == 0) return;

    if (chain->head == 0) chain->head = (u8)(chain->capacity - 1);
    else chain->head--;

    chain->points[chain->head].x = x;
    chain->points[chain->head].y = y;

    if (chain->count < chain->capacity) chain->count++;
}

// Read a point by age, with index zero denoting the newest point. Return zero
// for an invalid request and clear a non-null output first. Keep head + index
// within the 8-bit range: addition is narrowed before the capacity wrap.
// Do not alias out to backing storage or chain fields: output is cleared before the source is read.
u8 chain_get_segment(const Chain* chain, u8 index, ChainPoint* out)
{
    u8 pos;
    if (out != 0) {
        out->x = 0;
        out->y = 0;
    }

    if (out == 0) return 0;
    if (chain == 0) return 0;
    if (chain->capacity == 0) return 0;
    if (index >= chain->count) return 0;

    pos = (u8)(chain->head + index);
    while (pos >= chain->capacity) {
        pos = (u8)(pos - chain->capacity);
    }
    out->x = chain->points[pos].x;
    out->y = chain->points[pos].y;
    return 1;
}

// Return the number of stored points, or zero for a null chain.
u8 chain_get_count(const Chain* chain)
{
    if (chain == 0) return 0;
    return chain->count;
}

// Use the same integer socket geometry as the GB articulated-snake implementation.
// Branches avoid a banked lookup table and floating-point trigonometry.
static s16 chain_body_offset_x(u8 sector)
{
    if (sector == 0 || sector == 1 || sector == 15) return 4;
    if (sector == 2 || sector == 14) return 3;
    if (sector == 3 || sector == 13) return 1;
    if (sector == 4 || sector == 12) return 0;
    if (sector == 5 || sector == 11) return (s16)(0 - 1);
    if (sector == 6 || sector == 10) return (s16)(0 - 3);
    return (s16)(0 - 4);
}

static s16 chain_body_offset_y(u8 sector)
{
    // Rotate the X component by a quarter turn in the clockwise sector system.
    return chain_body_offset_x((u8)((sector + 12) & 15));
}

static u8 chain_body_wrap(s16 value, u16 size)
{
    // Size is validated at initialization, so neither loop can stall at zero.
    while (value < 0) value = (s16)(value + (s16)size);
    while (value >= (s16)size) value = (s16)(value - (s16)size);
    return (u8)value;
}

static s16 chain_body_delta(u8 target, u8 current, u16 size)
{
    s16 delta;
    s16 half;
    delta = (s16)((s16)target - (s16)current);
    half = (s16)(size >> 1);
    // At exactly half a field, retain the sign of the direct displacement.
    if (delta > half) delta = (s16)(delta - (s16)size);
    else if (delta < (s16)(0 - half)) delta = (s16)(delta + (s16)size);
    return delta;
}

static s16 chain_body_correction(s16 delta)
{
    if (delta > 4) return 2;
    if (delta > 0) return 1;
    if (delta < (s16)(0 - 4)) return (s16)(0 - 2);
    if (delta < 0) return (s16)(0 - 1);
    return 0;
}

u8 chain_body_init(ChainBody* body, u8* x, u8* y, u8* heading, u8 capacity, u16 width, u16 height)
{
    if (body == 0) return 0;
    body->x = x;
    body->y = y;
    body->heading = heading;
    body->count = 0;
    body->capacity = 0;
    body->width = width;
    body->height = height;
    if (x == 0 || y == 0 || heading == 0 || capacity == 0) return 0;
    if (width == 0 || width > 256 || height == 0 || height > 256) return 0;
    body->capacity = capacity;
    return 1;
}

u8 chain_body_reset(ChainBody* body, u8 count, u8 head_x, u8 head_y, u8 heading)
{
    u8 i;
    u8 x;
    u8 y;
    s16 ox;
    s16 oy;
    if (body == 0) return 0;
    if (count == 0 || count > body->capacity) return 0;
    heading = (u8)(heading & 15);
    x = chain_body_wrap((s16)head_x, body->width);
    y = chain_body_wrap((s16)head_y, body->height);
    ox = chain_body_offset_x(heading);
    oy = chain_body_offset_y(heading);
    i = 0;
    while (i < count) {
        body->x[(__safe_index u8)i] = x;
        body->y[(__safe_index u8)i] = y;
        body->heading[(__safe_index u8)i] = heading;
        x = chain_body_wrap((s16)((s16)x - ox), body->width);
        y = chain_body_wrap((s16)((s16)y - oy), body->height);
        i = (u8)(i + 1);
    }
    body->count = count;
    return 1;
}

void chain_body_step(ChainBody* body, u8 head_x, u8 head_y, u8 heading)
{
    u8 i;
    u8 previous;
    u8 pull;
    u8 old;
    u8 x;
    u8 y;
    u8 tx;
    u8 ty;
    s16 sx;
    s16 sy;
    if (body == 0) return;
    if (body->count == 0) return;
    heading = (u8)(heading & 15);
    body->x[0] = chain_body_wrap((s16)head_x, body->width);
    body->y[0] = chain_body_wrap((s16)head_y, body->height);
    body->heading[0] = heading;
    pull = heading;
    i = 1;
    while (i < body->count) {
        previous = (u8)(i - 1);
        old = body->heading[(__safe_index u8)i];
        x = body->x[(__safe_index u8)i];
        y = body->y[(__safe_index u8)i];
        tx = chain_body_wrap((s16)((s16)body->x[(__safe_index u8)previous] - chain_body_offset_x(pull)), body->width);
        ty = chain_body_wrap((s16)((s16)body->y[(__safe_index u8)previous] - chain_body_offset_y(pull)), body->height);
        sx = chain_body_correction(chain_body_delta(tx, x, body->width));
        sy = chain_body_correction(chain_body_delta(ty, y, body->height));
        body->heading[(__safe_index u8)i] = pull;
        body->x[(__safe_index u8)i] = chain_body_wrap((s16)((s16)x + sx), body->width);
        body->y[(__safe_index u8)i] = chain_body_wrap((s16)((s16)y + sy), body->height);
        // Save before overwriting: a bend advances by one joint per update.
        pull = old;
        i = (u8)(i + 1);
    }
}

u8 chain_body_grow(ChainBody* body)
{
    u8 tail;
    u8 next;
    if (body == 0) return 0;
    next = body->count;
    if (next == 0 || next >= body->capacity) return 0;
    tail = (u8)(next - 1);
    body->x[(__safe_index u8)next] = body->x[(__safe_index u8)tail];
    body->y[(__safe_index u8)next] = body->y[(__safe_index u8)tail];
    body->heading[(__safe_index u8)next] = body->heading[(__safe_index u8)tail];
    body->count = (u8)(next + 1);
    return 1;
}

void chain_body_clear(ChainBody* body)
{
    if (body == 0) return;
    body->count = 0;
}
