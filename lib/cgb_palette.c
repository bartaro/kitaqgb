#include "cgb_palette.h"

// Mask palette/color indices to 3/2 bits and combine them into one of 32 color slots.
u8 cgb__palette_color_slot(u8 palette_index, u8 color_index)
{
    return (u8)((u8)((palette_index & 7) << 2) + (u8)(color_index & 3));
}

// Clamp a sequential write to the remaining color slots. The start slot must
// already be normalized to 0..31.
u8 cgb__limit_color_count(u8 start_color_slot, u8 color_count)
{
    u8 remaining = (u8)(32 - start_color_slot);

    if (color_count > remaining) return remaining;
    return color_count;
}

// On CGB, select the masked background slot and pass low/high color bytes to
// the safe palette-write intrinsics. DMG mode is a no-op.
void cgb__write_color_bg_raw(u8 color_slot, u16 rgb15)
{
    u8 byte_index;

    if (__cgb_is_cgb() == 0) return;

    // Advance from the low byte to the high byte instead of overwriting the same slot.
    byte_index = (u8)(((color_slot & 31) << 1) | 128);
    __cgb_safe_set_bcps(byte_index);
    __cgb_safe_set_bcpd((u8)rgb15);
    __cgb_safe_set_bcpd((u8)(rgb15 >> 8));
}

// On CGB, select the masked object slot and submit the two color bytes through
// the safe object-palette intrinsics. DMG mode is a no-op.
void cgb__write_color_obj_raw(u8 color_slot, u16 rgb15)
{
    u8 byte_index;

    if (__cgb_is_cgb() == 0) return;

    // Advance from the low byte to the high byte instead of overwriting the same slot.
    byte_index = (u8)(((color_slot & 31) << 1) | 128);
    __cgb_safe_set_ocps(byte_index);
    __cgb_safe_set_ocpd((u8)rgb15);
    __cgb_safe_set_ocpd((u8)(rgb15 >> 8));
}

// Write a sequential background-color range with index auto-increment enabled.
// Ignore null/empty input and clamp at slot 31 rather than wrapping to slot zero.
void cgb__write_colors_bg_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    u8 i;
    u16 rgb15;
    u8 byte_index;
    const u16* cur;

    if (__cgb_is_cgb() == 0) return;
    if (colors == 0 || color_count == 0) return;

    start_color_slot = (u8)(start_color_slot & 31);
    color_count = cgb__limit_color_count(start_color_slot, color_count);
    byte_index = (u8)(((u8)(start_color_slot << 1)) | 128);
    cur = colors;

    __cgb_safe_set_bcps(byte_index);
    for (i = 0; i < color_count; ++i)
    {
        rgb15 = *cur;
        cur = cur + 1;
        __cgb_safe_set_bcpd((u8)rgb15);
        __cgb_safe_set_bcpd((u8)(rgb15 >> 8));
    }
}

// Write a sequential object-color range with auto-increment, clamped to the
// 32-slot palette store. The source contains color words, not raw tile bytes.
void cgb__write_colors_obj_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    u8 i;
    u16 rgb15;
    u8 byte_index;
    const u16* cur;

    if (__cgb_is_cgb() == 0) return;
    if (colors == 0 || color_count == 0) return;

    start_color_slot = (u8)(start_color_slot & 31);
    color_count = cgb__limit_color_count(start_color_slot, color_count);
    byte_index = (u8)(((u8)(start_color_slot << 1)) | 128);
    cur = colors;

    __cgb_safe_set_ocps(byte_index);
    for (i = 0; i < color_count; ++i)
    {
        rgb15 = *cur;
        cur = cur + 1;
        __cgb_safe_set_ocpd((u8)rgb15);
        __cgb_safe_set_ocpd((u8)(rgb15 >> 8));
    }
}

// Pack the red, green and blue components using the public RGB15 macro.
u16 cgb_rgb15(u8 r5, u8 g5, u8 b5)
{
    return CGB_RGB15(r5, g5, b5);
}

// Set one background color through the indexed palette wrapper.
void cgb_bg_color(u8 palette_index, u8 color_index, u16 rgb15)
{
    cgb__write_color_bg_raw(cgb__palette_color_slot(palette_index, color_index), rgb15);
}

// Set one object color through the indexed palette wrapper.
void cgb_obj_color(u8 palette_index, u8 color_index, u16 rgb15)
{
    cgb__write_color_obj_raw(cgb__palette_color_slot(palette_index, color_index), rgb15);
}

// Pack component values and write one background color.
void cgb_bg_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5)
{
    cgb_bg_color(palette_index, color_index, cgb_rgb15(r5, g5, b5));
}

// Pack component values and write one object color.
void cgb_obj_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5)
{
    cgb_obj_color(palette_index, color_index, cgb_rgb15(r5, g5, b5));
}

// Write background colors starting at the indexed slot. The range may cross
// palette boundaries and is clamped only at the end of all 32 slots.
void cgb_bg_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count)
{
    cgb__write_colors_bg_raw(cgb__palette_color_slot(palette_index, start_color_index), colors, color_count);
}

// Write object colors from the indexed slot, potentially crossing into later palettes.
void cgb_obj_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count)
{
    cgb__write_colors_obj_raw(cgb__palette_color_slot(palette_index, start_color_index), colors, color_count);
}

// Write four consecutive colors for one background palette.
void cgb_bg_palette(u8 palette_index, const u16* colors)
{
    cgb_bg_colors(palette_index, 0, colors, 4);
}

// Write four consecutive colors for one object palette.
void cgb_obj_palette(u8 palette_index, const u16* colors)
{
    cgb_obj_colors(palette_index, 0, colors, 4);
}

// Expose sequential background writes using an absolute color-slot index.
void cgb_bg_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    cgb__write_colors_bg_raw(start_color_slot, colors, color_count);
}

// Expose sequential object writes using an absolute color-slot index.
void cgb_obj_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    cgb__write_colors_obj_raw(start_color_slot, colors, color_count);
}
