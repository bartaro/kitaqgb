#include "asset.h"

static u8 asset_table_bank;
static const AssetDesc* asset_table;
static u8 asset_table_count;

void asset_set_table(u8 bank, const AssetDesc* table, u8 count)
{
    asset_table_bank = bank;
    asset_table = table;
    asset_table_count = count;
}

u8 asset_get(u8 asset_id, AssetDesc* out_desc)
{
    if (out_desc == 0) return 0;
    if (asset_id >= asset_table_count) return 0;
    __farmemcpy(out_desc, asset_table_bank, asset_table + asset_id, sizeof(AssetDesc));
    return 1;
}

u8 asset_load_raw(u8 asset_id, void* dst, u16 max_len)
{
    AssetDesc desc;
    u16 len;
    if (asset_get(asset_id, &desc) == 0) return 0;
    len = desc.len;
    if (len > max_len) len = max_len;
    __farmemcpy(dst, desc.bank, desc.ptr, len);
    return 1;
}

u8 asset_load_tiles(u8 asset_id, u16 vram_dst)
{
    AssetDesc desc;
    if (asset_get(asset_id, &desc) == 0) return 0;
    if (desc.type != ASSET_TYPE_TILES && desc.type != ASSET_TYPE_RAW) return 0;
    __vram_copy(vram_dst, desc.ptr, desc.len);
    return 1;
}

u8 asset_get_bank(u8 asset_id)
{
    AssetDesc desc;
    if (asset_get(asset_id, &desc) == 0) return 0;
    return desc.bank;
}

const u8* asset_get_ptr(u8 asset_id)
{
    AssetDesc desc;
    if (asset_get(asset_id, &desc) == 0) return 0;
    return desc.ptr;
}

u16 asset_get_len(u8 asset_id)
{
    AssetDesc desc;
    if (asset_get(asset_id, &desc) == 0) return 0;
    return desc.len;
}

