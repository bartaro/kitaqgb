#pragma once
// Copyright (c) 2026 DAISUKE OBA. MIT License.
// ZX0-compatible forward v2 streams; format designed by Einar Saukas.
#define ZX0_OK 0
#define ZX0_BAD_ARGUMENT 1
#define ZX0_TRUNCATED 2
#define ZX0_BAD_STREAM 3
#define ZX0_OUTPUT_FULL 4
#define ZX0_DISPLAY_ACTIVE 5
// Shared last-call status. These foreground routines are not reentrant.
extern u8 zx0_error;
// Return decoded bytes, or zero on error (possibly after partial writes).
// Source and output must not overlap or cross their current CPU bank windows.
// Capacity and packed_size are byte counts. No prefix/backward/v1 streams.
u16 zx0_decompress(void* dst, u16 capacity, const void* src, u16 packed_size);
// Decode a KQA1 raw/RLE/ZX0 asset produced with --format=auto.
// Empty raw assets return zero with zx0_error==ZX0_OK.
u16 asset_decompress(void* dst, u16 capacity, const void* src, u16 packed_size);
// Decode directly into the selected GB VRAM bank ($8000..$9FFF), with LCD off.
// Preserve LCDC, VBK and interrupt state. Output must fit before $A000.
u16 zx0_decompress_vram(u16 vram_addr, const void* src, u16 packed_size);
