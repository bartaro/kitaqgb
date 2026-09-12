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

void chain_init(Chain* chain, ChainPoint* storage, u8 capacity);
void chain_clear(Chain* chain);
void chain_push_head(Chain* chain, s16 x, s16 y);
u8 chain_get_segment(const Chain* chain, u8 index, ChainPoint* out);
u8 chain_get_count(const Chain* chain);
