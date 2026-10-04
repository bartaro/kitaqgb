#pragma once

#define ASSET_TYPE_RAW   ((u8)0)
#define ASSET_TYPE_TILES ((u8)1)
#define ASSET_TYPE_MAP   ((u8)2)
#define ASSET_TYPE_SONG  ((u8)3)

typedef __packed struct {
    u8 type;
    u8 bank;
    const u8* ptr;
    u16 len;
} AssetDesc;

void __farmemcpy(void* dst, u8 bank, const void* src, u16 len);
void __vram_copy(u16 dst, const void* src, u16 len);

void asset_set_table(u8 bank, const AssetDesc* table, u8 count);
u8 asset_get(u8 asset_id, AssetDesc* out_desc);
u8 asset_load_raw(u8 asset_id, void* dst, u16 max_len);
u8 asset_load_tiles(u8 asset_id, u16 vram_dst);
u8 asset_get_bank(u8 asset_id);
const u8* asset_get_ptr(u8 asset_id);
u16 asset_get_len(u8 asset_id);

