#pragma once

typedef __packed struct {
    s16 x;
    s16 y;
} ChainPoint;

typedef __packed struct {
    ChainPoint* points;
    u8 capacity;
    u8 count;
    u8 head;
} Chain;

// Attach caller-owned ring storage and start with no recorded points.
// The storage must remain valid for the lifetime of the chain.
// Use a valid chain pointer and capacity entries of caller-owned storage.
// Capacity at most 128 keeps every head+age sum in the byte range used by the reader.
void chain_init(Chain* chain, ChainPoint* storage, u8 capacity);
// Forget the recorded points without clearing or freeing the backing storage.
void chain_clear(Chain* chain);
// Move the ring head backward and store the newest point. Once full, each push
// replaces the oldest point; zero capacity is a no-op.
void chain_push_head(Chain* chain, s16 x, s16 y);
// Read a point by age, with index zero denoting the newest point. Return zero
// for an invalid request and clear a non-null output first. Keep head + index
// within the 8-bit range: addition is narrowed before the capacity wrap.
u8 chain_get_segment(const Chain* chain, u8 index, ChainPoint* out);
// Return the number of stored points, or zero for a null chain.
u8 chain_get_count(const Chain* chain);
