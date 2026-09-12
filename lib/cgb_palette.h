#pragma once

// High-level CGB palette helpers for KITAQGB.
// These functions no-op on DMG and wrap the existing safe CGB register writers.

u8 __cgb_is_cgb();
void __cgb_safe_set_bcps(u8 value);
void __cgb_safe_set_bcpd(u8 value);
void __cgb_safe_set_ocps(u8 value);
void __cgb_safe_set_ocpd(u8 value);

#define CGB_RGB15(r5, g5, b5) ((u16)((u16)((r5) & 31) | ((u16)((g5) & 31) << 5) | ((u16)((b5) & 31) << 10)))

u16 cgb_rgb15(u8 r5, u8 g5, u8 b5);

void cgb_bg_color(u8 palette_index, u8 color_index, u16 rgb15);
void cgb_obj_color(u8 palette_index, u8 color_index, u16 rgb15);
void cgb_bg_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5);
void cgb_obj_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5);

void cgb_bg_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count);
void cgb_obj_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count);

void cgb_bg_palette(u8 palette_index, const u16* colors);
void cgb_obj_palette(u8 palette_index, const u16* colors);

void cgb_bg_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count);
void cgb_obj_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count);
