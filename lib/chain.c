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
