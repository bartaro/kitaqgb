#pragma once

// Declarations for compiler-intrinsic CGB tile helpers.
// Canonical intrinsic entry points use the "__" prefix.
// Attr writes no-op on DMG. Combined tile+attr writes still write the tile on DMG.

// These helpers emit compiler code; cgb_tile.c has no runtime bodies. Scalar coordinates address 32x32 maps.
// Safe write paths poll VRAM access; LCD-on paths can enable interrupts after writing.
// Use _unsafe/_fast only with caller-managed writable VRAM timing, especially inside an ISR.
// Tile writes use the selected VRAM bank; choose bank zero before combined tile/attribute operations.
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

// Return the compiler/runtime CGB mode test as a byte Boolean.
u8 __cgb_is_cgb();
// Write the VRAM bank selection only on CGB. Safe here means hardware-mode guarded, not PPU-timed.
void __cgb_safe_set_vbk(u8 value);

// Write a tile number to map 9800, waiting for VRAM access when necessary.
void __settile(u8 x, u8 y, u8 tile);
// Write a tile number to map 9800 without waiting for writable VRAM.
void __settile_unsafe(u8 x, u8 y, u8 tile);
// Write at base+32*y+x with VRAM access checks; supply a valid tile-map base.
void __settileat(u16 base, u8 x, u8 y, u8 tile);
// Use an explicit map base without waiting for VRAM access.
void __settileat_unsafe(u16 base, u8 x, u8 y, u8 tile);
// Select the window map from LCDC bit 6, then write a tile with access checks.
void __settilewin(u8 x, u8 y, u8 tile);
// Select the LCDC window map and write without access waits.
void __settilewin_unsafe(u8 x, u8 y, u8 tile);
// Select the background map from LCDC bit 3, then write with access checks.
void __settilebg(u8 x, u8 y, u8 tile);
// Select the LCDC background map and write without access waits.
void __settilebg_unsafe(u8 x, u8 y, u8 tile);

// Copy count sequential bytes to dest using the current VRAM bank and access-checked transfer.
void __settile_bulk(u16 dest, const u8* src, u8 count);
// Copy count sequential bytes to dest without VRAM access waits; retain source-bank visibility.
void __settile_bulk_fast(u16 dest, const u8* src, u8 count);

// Write attributes in bank one at map 9800 on CGB, then leave VBK zero; skip on DMG. Use the access-checked write path.
void __settileattr(u8 x, u8 y, u8 attr);
// Write attributes in bank one at map 9800 on CGB, then leave VBK zero; skip on DMG. No VRAM access waits are inserted.
void __settileattr_unsafe(u8 x, u8 y, u8 attr);
// Write tile and CGB attributes at map 9800; DMG receives only the tile write. Use the access-checked write path.
void __settilecgb(u8 x, u8 y, u8 tile, u8 attr);
// Write tile and CGB attributes at map 9800; DMG receives only the tile write. No VRAM access waits are inserted.
void __settilecgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

// Write attributes in bank one at the explicit map base on CGB, then leave VBK zero; skip on DMG. Use the access-checked write path.
void __settileatattr(u16 base, u8 x, u8 y, u8 attr);
// Write attributes in bank one at the explicit map base on CGB, then leave VBK zero; skip on DMG. No VRAM access waits are inserted.
void __settileatattr_unsafe(u16 base, u8 x, u8 y, u8 attr);
// Write tile and CGB attributes at the explicit map base; DMG receives only the tile write. Use the access-checked write path.
void __settileatcgb(u16 base, u8 x, u8 y, u8 tile, u8 attr);
// Write tile and CGB attributes at the explicit map base; DMG receives only the tile write. No VRAM access waits are inserted.
void __settileatcgb_unsafe(u16 base, u8 x, u8 y, u8 tile, u8 attr);

// Write attributes in bank one at the LCDC window map on CGB, then leave VBK zero; skip on DMG. Use the access-checked write path.
void __settilewinattr(u8 x, u8 y, u8 attr);
// Write attributes in bank one at the LCDC window map on CGB, then leave VBK zero; skip on DMG. No VRAM access waits are inserted.
void __settilewinattr_unsafe(u8 x, u8 y, u8 attr);
// Write tile and CGB attributes at the LCDC window map; DMG receives only the tile write. Use the access-checked write path.
void __settilewincgb(u8 x, u8 y, u8 tile, u8 attr);
// Write tile and CGB attributes at the LCDC window map; DMG receives only the tile write. No VRAM access waits are inserted.
void __settilewincgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

// Write attributes in bank one at the LCDC background map on CGB, then leave VBK zero; skip on DMG. Use the access-checked write path.
void __settilebgattr(u8 x, u8 y, u8 attr);
// Write attributes in bank one at the LCDC background map on CGB, then leave VBK zero; skip on DMG. No VRAM access waits are inserted.
void __settilebgattr_unsafe(u8 x, u8 y, u8 attr);
// Write tile and CGB attributes at the LCDC background map; DMG receives only the tile write. Use the access-checked write path.
void __settilebgcgb(u8 x, u8 y, u8 tile, u8 attr);
// Write tile and CGB attributes at the LCDC background map; DMG receives only the tile write. No VRAM access waits are inserted.
void __settilebgcgb_unsafe(u8 x, u8 y, u8 tile, u8 attr);

// Copy count attributes to dest in bank one on CGB, then leave VBK zero; skip on DMG. Transfers use access checks.
void __settileattr_bulk(u16 dest, const u8* src, u8 count);
// Copy count attributes to dest in bank one on CGB, then leave VBK zero; skip on DMG. The caller supplies writable VRAM timing.
void __settileattr_bulk_fast(u16 dest, const u8* src, u8 count);
// Copy tile and attribute streams to dest; DMG receives only tiles. Keep both sources readable. Transfers use access checks.
void __settilecgb_bulk(u16 dest, const u8* tile_src, const u8* attr_src, u8 count);
// Copy tile and attribute streams to dest; DMG receives only tiles. Keep both sources readable. The caller supplies writable VRAM timing.
void __settilecgb_bulk_fast(u16 dest, const u8* tile_src, const u8* attr_src, u8 count);
