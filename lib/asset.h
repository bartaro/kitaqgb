#pragma once

#define ASSET_TYPE_RAW   ((u8)0)
#define ASSET_TYPE_TILES ((u8)1)
#define ASSET_TYPE_MAP   ((u8)2)
#define ASSET_TYPE_SONG  ((u8)3)

// Packed ROM descriptor: type tag, payload bank, bank-window address and byte length.
// The table itself may live in a different bank from the payload.
typedef __packed struct {
    u8 type;
    u8 bank;
    const u8* ptr;
    u16 len;
} AssetDesc;

void __farmemcpy(void* dst, u8 bank, const void* src, u16 len);
void __vram_copy(u16 dst, const void* src, u16 len);

// Retain the descriptor table address, bank and count; the table is not copied.
void asset_set_table(u8 bank, const AssetDesc* table, u8 count);
// Copy one descriptor from its ROM bank into caller storage. Return zero for a
// null destination or an out-of-range ID; table validity is the caller's responsibility.
u8 asset_get(u8 asset_id, AssetDesc* out_desc);
// Copy at most max_len bytes using the asset's bank. Success means the ID was
// valid, even when the payload was truncated to fit the supplied limit.
u8 asset_load_raw(u8 asset_id, void* dst, u16 max_len);
// Accept tile/raw descriptors and transfer their bytes to VRAM. This path passes
// the descriptor pointer directly; callers must arrange the correct visible ROM bank.
u8 asset_load_tiles(u8 asset_id, u16 vram_dst);
// Return the descriptor bank, or zero on lookup failure; zero can also be a valid bank.
u8 asset_get_bank(u8 asset_id);
// Return the stored address without switching banks. Use the descriptor bank
// with far-data helpers before dereferencing banked assets; failure returns null.
const u8* asset_get_ptr(u8 asset_id);
// Return the descriptor byte length, or zero if its ID cannot be resolved.
u16 asset_get_len(u8 asset_id);

