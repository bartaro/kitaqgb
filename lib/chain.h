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

// Return target minus current along the shortest wrapped route.
// Supply size 1..256 and coordinates in 0..size-1; inputs are not normalized.
// Exactly half a field retains the direct displacement sign.
s16 chain_wrap_delta(u8 target, u8 current, u16 size);

// Articulated body: one current pose per joint, including the head at index 0.
// Caller-owned byte arrays keep the inner loop compact on both GB and FC.
// Heading sectors run clockwise: 0=right, 4=down, 8=left, 12=up.
// Coordinates are pixels; each wrapped dimension accepts 1..256 pixels.
typedef __packed struct {
    u8* x;
    u8* y;
    u8* heading;
    u8 capacity;
    u8 count;
    u16 width;
    u16 height;
} ChainBody;

// Attach three non-overlapping capacity-byte arrays. Return 1 on success.
// Invalid storage, zero capacity or dimensions outside 1..256 disable the body.
u8 chain_body_init(ChainBody* body, u8* x, u8* y, u8* heading, u8 capacity, u16 width, u16 height);
// Seed count joints in a straight pose behind the head; count includes the head.
// Return 0 without changing the pose if count is zero or exceeds capacity.
// Coordinates wrap to the playfield and headings are masked to 0..15.
u8 chain_body_reset(ChainBody* body, u8 count, u8 head_x, u8 head_y, u8 heading);
// Install the caller's head pose, then pull joints from front to rear.
// Each joint aims four pixels behind its updated predecessor and receives that
// predecessor's OLD heading. Each axis moves 1 pixel, or 2 when error exceeds 4.
// Call once per simulation tick; this does not advance head physics or render.
void chain_body_step(ChainBody* body, u8 head_x, u8 head_y, u8 heading);
// Append a copy of the current tail; later steps separate the new joint.
// Return 1 on growth, or 0 for an empty, disabled or full body.
u8 chain_body_grow(ChainBody* body);
// Remove all joints while retaining caller-owned storage and playfield settings.
void chain_body_clear(ChainBody* body);
