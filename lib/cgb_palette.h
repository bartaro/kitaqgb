#pragma once

// High-level CGB palette helpers for KITAQGB.
// Palette writes no-op on DMG; RGB packing works on either hardware model.

// Safe here means CGB hardware gating. Palette data writes still require an accessible
// PPU phase (for example LCD off or a sufficiently short VBlank update).
u8 __cgb_is_cgb();
void __cgb_safe_set_bcps(u8 value);
void __cgb_safe_set_bcpd(u8 value);
void __cgb_safe_set_ocps(u8 value);
void __cgb_safe_set_ocpd(u8 value);

// Mask each component to five bits: red in 0..4, green in 5..9, blue in 10..14.
// These macros wrap oversized components rather than saturating them.
#define CGB_RGB15(r5, g5, b5) ((u16)((u16)((r5) & 31) | ((u16)((g5) & 31) << 5) | ((u16)((b5) & 31) << 10)))

// Pack the red, green and blue components using the public RGB15 macro.
u16 cgb_rgb15(u8 r5, u8 g5, u8 b5);

// Set one background color through the indexed palette wrapper.
void cgb_bg_color(u8 palette_index, u8 color_index, u16 rgb15);
// Set one object color through the indexed palette wrapper.
void cgb_obj_color(u8 palette_index, u8 color_index, u16 rgb15);
// Pack component values and write one background color.
void cgb_bg_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5);
// Pack component values and write one object color.
void cgb_obj_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5);

// Write background colors starting at the indexed slot. The range may cross
// palette boundaries and is clamped only at the end of all 32 slots.
void cgb_bg_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count);
// Write object colors from the indexed slot, potentially crossing into later palettes.
void cgb_obj_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count);

// Write four consecutive colors for one background palette.
void cgb_bg_palette(u8 palette_index, const u16* colors);
// Write four consecutive colors for one object palette.
void cgb_obj_palette(u8 palette_index, const u16* colors);

// Expose sequential background writes using an absolute color-slot index.
void cgb_bg_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count);
// Expose sequential object writes using an absolute color-slot index.
void cgb_obj_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count);
