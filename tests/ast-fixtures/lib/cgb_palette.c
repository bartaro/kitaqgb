#include "cgb_palette.h"

u8 cgb__palette_color_slot(u8 palette_index, u8 color_index)
{
    return (u8)((u8)((palette_index & 7) << 2) + (u8)(color_index & 3));
}

u8 cgb__limit_color_count(u8 start_color_slot, u8 color_count)
{
    u8 remaining = (u8)(32 - start_color_slot);

    if (color_count > remaining) return remaining;
    return color_count;
}

void cgb__write_color_bg_raw(u8 color_slot, u16 rgb15)
{
    u8 byte_index;

    if (__cgb_is_cgb() == 0) return;

    byte_index = (u8)((color_slot & 31) << 1);
    __cgb_safe_set_bcps(byte_index);
    __cgb_safe_set_bcpd((u8)rgb15);
    __cgb_safe_set_bcpd((u8)(rgb15 >> 8));
}

void cgb__write_color_obj_raw(u8 color_slot, u16 rgb15)
{
    u8 byte_index;

    if (__cgb_is_cgb() == 0) return;

    byte_index = (u8)((color_slot & 31) << 1);
    __cgb_safe_set_ocps(byte_index);
    __cgb_safe_set_ocpd((u8)rgb15);
    __cgb_safe_set_ocpd((u8)(rgb15 >> 8));
}

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

u16 cgb_rgb15(u8 r5, u8 g5, u8 b5)
{
    return CGB_RGB15(r5, g5, b5);
}

void cgb_bg_color(u8 palette_index, u8 color_index, u16 rgb15)
{
    cgb__write_color_bg_raw(cgb__palette_color_slot(palette_index, color_index), rgb15);
}

void cgb_obj_color(u8 palette_index, u8 color_index, u16 rgb15)
{
    cgb__write_color_obj_raw(cgb__palette_color_slot(palette_index, color_index), rgb15);
}

void cgb_bg_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5)
{
    cgb_bg_color(palette_index, color_index, cgb_rgb15(r5, g5, b5));
}

void cgb_obj_rgb(u8 palette_index, u8 color_index, u8 r5, u8 g5, u8 b5)
{
    cgb_obj_color(palette_index, color_index, cgb_rgb15(r5, g5, b5));
}

void cgb_bg_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count)
{
    cgb__write_colors_bg_raw(cgb__palette_color_slot(palette_index, start_color_index), colors, color_count);
}

void cgb_obj_colors(u8 palette_index, u8 start_color_index, const u16* colors, u8 color_count)
{
    cgb__write_colors_obj_raw(cgb__palette_color_slot(palette_index, start_color_index), colors, color_count);
}

void cgb_bg_palette(u8 palette_index, const u16* colors)
{
    cgb_bg_colors(palette_index, 0, colors, 4);
}

void cgb_obj_palette(u8 palette_index, const u16* colors)
{
    cgb_obj_colors(palette_index, 0, colors, 4);
}

void cgb_bg_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    cgb__write_colors_bg_raw(start_color_slot, colors, color_count);
}

void cgb_obj_colors_raw(u8 start_color_slot, const u16* colors, u8 color_count)
{
    cgb__write_colors_obj_raw(start_color_slot, colors, color_count);
}
