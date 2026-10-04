#pragma once

// Declarations for compiler-intrinsic CGB tile helpers.
// Canonical intrinsic entry points use the "__" prefix.
// Attr writes no-op on DMG. Combined tile+attr writes still write the tile on DMG.

#define KQ_CGB_ATTR_PAL_MASK 0x07
#define KQ_CGB_ATTR_BANK1 0x08
#define KQ_CGB_ATTR_XFLIP 0x20
#define KQ_CGB_ATTR_YFLIP 0x40
#define KQ_CGB_ATTR_PRIORITY 0x80

#define KQ_CGB_ATTR_NONE ((u8)0)
#define KQ_CGB_ATTR_PAL(n) ((u8)((n) & KQ_CGB_ATTR_PAL_MASK))
#define KQ_CGB_ATTR_BANK(bank) ((u8)((bank) ? KQ_CGB_ATTR_BANK1 : 0))
#define KQ_CGB_ATTR_XFLIP_IF(enabled) ((u8)((enabled) ? KQ_CGB_ATTR_XFLIP : 0))
#define KQ_CGB_ATTR_YFLIP_IF(enabled) ((u8)((enabled) ? KQ_CGB_ATTR_YFLIP : 0))
#define KQ_CGB_ATTR_PRIORITY_IF(enabled) ((u8)((enabled) ? KQ_CGB_ATTR_PRIORITY : 0))
#define KQ_CGB_ATTR(palette, bank, xflip, yflip, priority) ((u8)(((palette) & KQ_CGB_ATTR_PAL_MASK) | ((bank) ? KQ_CGB_ATTR_BANK1 : 0) | ((xflip) ? KQ_CGB_ATTR_XFLIP : 0) | ((yflip) ? KQ_CGB_ATTR_YFLIP : 0) | ((priority) ? KQ_CGB_ATTR_PRIORITY : 0)))

u8 __cgb_is_cgb();
void __cgb_safe_set_vbk(u8 value);

void __settile(u8 x, u8 y, u8 tile);
void __settile_unsafe(u8 x, u8 y, u8 tile);
void __settileat(u16 base, u8 x, u8 y, u8 tile);
void __settileat_unsafe(u16 base, u8 x, u8 y, u8 tile);
void __settilewin(u8 x, u8 y, u8 tile);
void __settilewin_unsafe(u8 x, u8 y, u8 tile);
void __settilebg(u8 x, u8 y, u8 tile);
void __settilebg_unsafe(u8 x, u8 y, u8 tile);

void __settile_bulk(u16 dest, const u8* src, u8 count);
void __settile_bulk_fast(u16 dest, const u8* src, u8 count);

void __settileattr(u8 x, u8 y, u8 attr);
void __settileattr_unsafe(u8 x, u8 y, u8 attr);
void __settilecgb(u8 x, u8 y, u8 tile, u8 attr);
void __settilecgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

void __settileatattr(u16 base, u8 x, u8 y, u8 attr);
void __settileatattr_unsafe(u16 base, u8 x, u8 y, u8 attr);
void __settileatcgb(u16 base, u8 x, u8 y, u8 tile, u8 attr);
void __settileatcgb_unsafe(u16 base, u8 x, u8 y, u8 tile, u8 attr);

void __settilewinattr(u8 x, u8 y, u8 attr);
void __settilewinattr_unsafe(u8 x, u8 y, u8 attr);
void __settilewincgb(u8 x, u8 y, u8 tile, u8 attr);
void __settilewincgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

void __settilebgattr(u8 x, u8 y, u8 attr);
void __settilebgattr_unsafe(u8 x, u8 y, u8 attr);
void __settilebgcgb(u8 x, u8 y, u8 tile, u8 attr);
void __settilebgcgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

void __settileattr_bulk(u16 dest, const u8* src, u8 count);
void __settileattr_bulk_fast(u16 dest, const u8* src, u8 count);
void __settilecgb_bulk(u16 dest, const u8* tile_src, const u8* attr_src, u8 count);
void __settilecgb_bulk_fast(u16 dest, const u8* tile_src, const u8* attr_src, u8 count);
