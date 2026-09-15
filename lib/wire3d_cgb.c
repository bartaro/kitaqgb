#include "wire3d_cgb.h"

// CGB-only renderer with shared non-reentrant state. Pixel, sparse, full-screen
// and fast-map modes reuse storage and require matching initialization/frame APIs.
// Do not call renderer routines concurrently from interrupt handlers.
#pragma rom_cgb cgb_only
#pragma bank 1

#define W3DCGB_TILE_W ((w3dcgb_u8)16)
#define W3DCGB_TILE_H ((w3dcgb_u8)12)
#define W3DCGB_ROW_BYTES ((w3dcgb_u8)0x60)
#define W3DCGB_STAGE_PLANE_STRIDE ((w3dcgb_u16)0x80)
#define W3DCGB_NEAR_Z ((w3dcgb_i16)8)
#define W3DCGB_FAR_Z ((w3dcgb_i16)255)
#define W3DCGB_CENTER_X ((w3dcgb_i16)64)
#define W3DCGB_CENTER_Y ((w3dcgb_i16)48)
#define W3DCGB_TRANSFORM_LIMIT ((w3dcgb_i16)220)
#define W3DCGB_PROJECT_LIMIT ((w3dcgb_i16)120)
#define W3DCGB_HUD_TILE_BASE ((w3dcgb_u8)0x80)
#define W3DCGB_HUD_TILE_BLANK ((w3dcgb_u8)0x8A)
#define W3DCGB_HUD_TILE_COUNT ((w3dcgb_u8)14)
#define W3DCGB_FAST_TILE_BASE ((w3dcgb_u8)0x40)
#define W3DCGB_FAST_DIAG_BASE ((w3dcgb_u8)0x70)
#define W3DCGB_FAST_STAMP_TILE_COUNT ((w3dcgb_u8)16)
#define W3DCGB_FAST_STAMP_SIZE ((w3dcgb_u8)4)
#define W3DCGB_FAST_STAMP_VARIANTS ((w3dcgb_u8)4)
#define W3DCGB_FAST_ATTR_OFFSET ((w3dcgb_u16)512)
#define W3DCGB_FAST_MAP_BYTES ((w3dcgb_u16)384)
#define W3DCGB_BG_QUEUE_LIMIT ((w3dcgb_u8)48)
#define W3DCGB_DIRTY_TILE_LIMIT ((w3dcgb_u8)192)

__location(0xFF40) w3dcgb_u8 w3dcgb_reg_lcdc;
__location(0xFF42) w3dcgb_u8 w3dcgb_reg_scy;
__location(0xFF43) w3dcgb_u8 w3dcgb_reg_scx;
__location(0xFF44) w3dcgb_u8 w3dcgb_reg_ly;
__location(0xFF47) w3dcgb_u8 w3dcgb_reg_bgp;
__location(0xFF4D) w3dcgb_u8 w3dcgb_reg_key1;
__location(0xFF4F) w3dcgb_u8 w3dcgb_reg_vbk;
__location(0xFF68) w3dcgb_u8 w3dcgb_reg_bcps;
__location(0xFF69) w3dcgb_u8 w3dcgb_reg_bcpd;

__location(0x8000) w3dcgb_u8 w3dcgb_vram_tiles[6144];
__location(0x9800) w3dcgb_u8 w3dcgb_bg_map_9800[1024];
/* The active 128x96 assembly path selects WRAM bank 2 around this window. */
#ifdef WIRE3DCGB_STAGE_BANK2_ALLOCATION
/* Bank 2's ASM stage starts at D300; simulation bank 1 remains available. */
__wramx_bank(2) w3dcgb_u8 w3dcgb_stage_bank2_prefix[0x300];
__wramx_bank(2) w3dcgb_u8 w3dcgb_stage[WIRE3DCGB_STAGE_BYTES];
#else
__location(WIRE3DCGB_STAGE_BASE) w3dcgb_u8 w3dcgb_stage[WIRE3DCGB_STAGE_BYTES];
#endif

// MSB-first pixel masks: local X zero addresses the leftmost bit of a tile row.
__prg_rom w3dcgb_u8 w3dcgb_bit_mask[8] = {
    0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01
};

// HUD glyphs 0123456789, space, S, L and V come from DAISUKE OBA
// original ASCII font under MIT, using the supplied GB tile conversion.
__prg_rom w3dcgb_u8 w3dcgb_hud_tiles[224] = {
    0x78, 0x78, 0xCC, 0xCC, 0xDC, 0xDC, 0xEC, 0xEC, 0xCC, 0xCC, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0x70, 0x70, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x78, 0x78, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0x0C, 0x0C, 0x18, 0x18, 0x30, 0x30, 0x60, 0x60, 0xFC, 0xFC, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0x0C, 0x0C, 0x38, 0x38, 0x0C, 0x0C, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0x18, 0x18, 0x38, 0x38, 0x78, 0x78, 0xD8, 0xD8, 0xFC, 0xFC, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00,
    0xFC, 0xFC, 0xC0, 0xC0, 0xC0, 0xC0, 0xF8, 0xF8, 0x0C, 0x0C, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0xC0, 0xC0, 0xF8, 0xF8, 0xCC, 0xCC, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0xFC, 0xFC, 0x0C, 0x0C, 0x0C, 0x0C, 0x18, 0x18, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0xCC, 0xCC, 0x78, 0x78, 0xCC, 0xCC, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0xCC, 0xCC, 0x7C, 0x7C, 0x0C, 0x0C, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x78, 0x78, 0xCC, 0xCC, 0xC0, 0xC0, 0x78, 0x78, 0x0C, 0x0C, 0xCC, 0xCC, 0x78, 0x78, 0x00, 0x00,
    0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xFC, 0xFC, 0x00, 0x00,
    0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x48, 0x48, 0x48, 0x48, 0x30, 0x30, 0x00, 0x00
};

#pragma bank 4
// Sixteen evenly spaced orientations, with signed Q6 coefficients (64 is unity).
__prg_rom w3dcgb_i8 w3dcgb_sin_q6[16] = {
     0,  24,  45,  59,  64,  59,  45,  24,
     0, -24, -45, -59, -64, -59, -45, -24
};

__prg_rom w3dcgb_i8 w3dcgb_cos_q6[16] = {
     64,  59,  45,  24,   0, -24, -45, -59,
    -64, -59, -45, -24,   0,  24,  45,  59
};

// Reciprocal-depth lookup: min(255, 1536/Z), with entry zero set to zero.
// The public 128x96 projector accepts only Z=8..255.
__prg_rom w3dcgb_u8 w3dcgb_inv_depth[256] = {
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xDB, 0xC0, 0xAA, 0x99, 0x8B, 0x80, 0x76, 0x6D, 0x66,
    0x60, 0x5A, 0x55, 0x50, 0x4C, 0x49, 0x45, 0x42, 0x40, 0x3D, 0x3B, 0x38, 0x36, 0x34, 0x33, 0x31,
    0x30, 0x2E, 0x2D, 0x2B, 0x2A, 0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x22, 0x21, 0x20,
    0x20, 0x1F, 0x1E, 0x1E, 0x1D, 0x1C, 0x1C, 0x1B, 0x1B, 0x1A, 0x1A, 0x1A, 0x19, 0x19, 0x18, 0x18,
    0x18, 0x17, 0x17, 0x16, 0x16, 0x16, 0x15, 0x15, 0x15, 0x15, 0x14, 0x14, 0x14, 0x13, 0x13, 0x13,
    0x13, 0x12, 0x12, 0x12, 0x12, 0x12, 0x11, 0x11, 0x11, 0x11, 0x11, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x10, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0E, 0x0E, 0x0E, 0x0E, 0x0E, 0x0E, 0x0E, 0x0D, 0x0D,
    0x0D, 0x0D, 0x0D, 0x0D, 0x0D, 0x0D, 0x0D, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C,
    0x0C, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0A, 0x0A, 0x0A, 0x0A,
    0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x08, 0x08, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
    0x08, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06
};

// Camera, line rasterization and projection scratch are shared across calls.
// Transforming another model replaces the cached projected vertices.
w3dcgb_i16 w3dcgb_cam_x;
w3dcgb_i16 w3dcgb_cam_y;
w3dcgb_i16 w3dcgb_cam_z;
w3dcgb_i8 w3dcgb_cam_pitch;
w3dcgb_i8 w3dcgb_cam_yaw;
w3dcgb_i8 w3dcgb_cam_roll;

w3dcgb_u8 w3dcgb_plot_x;
w3dcgb_u8 w3dcgb_plot_y;
w3dcgb_u8 w3dcgb_plot_tx;
w3dcgb_u8 w3dcgb_line_x0;
w3dcgb_u8 w3dcgb_line_y0;
w3dcgb_u8 w3dcgb_line_x1;
w3dcgb_u8 w3dcgb_line_y1;
w3dcgb_u8 w3dcgb_line_x;
w3dcgb_u8 w3dcgb_line_y;
w3dcgb_u8 w3dcgb_line_dx;
w3dcgb_u8 w3dcgb_line_dy;
w3dcgb_u8 w3dcgb_line_sx;
w3dcgb_u8 w3dcgb_line_sy;
w3dcgb_u8 w3dcgb_line_err;
w3dcgb_u8 w3dcgb_line_remaining;

w3dcgb_u8 w3dcgb_screen_x[WIRE3DCGB_MODEL_VERTEX_LIMIT];
w3dcgb_u8 w3dcgb_screen_y[WIRE3DCGB_MODEL_VERTEX_LIMIT];
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
w3dcgb_u8 w3dcgb_screen_visible[WIRE3DCGB_MODEL_VERTEX_LIMIT];
w3dcgb_u8 w3dcgb_face_visible[WIRE3DCGB_MODEL_FACE_LIMIT];
#endif
w3dcgb_u8 w3dcgb_occlusion_mask[1536];
w3dcgb_u8 w3dcgb_occlusion_active;
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
w3dcgb_u8 w3dcgb_scene_order[WIRE3DCGB_SCENE_OBJECT_LIMIT];
w3dcgb_i16 w3dcgb_scene_depth[WIRE3DCGB_SCENE_OBJECT_LIMIT];
#endif
w3dcgb_u8 w3dcgb_bgq_x[48];
w3dcgb_u8 w3dcgb_bgq_y[48];
w3dcgb_u8 w3dcgb_bgq_tile[48];
w3dcgb_u8 w3dcgb_bgq_count;
w3dcgb_u8 w3dcgb_bgq_addr_hi;
w3dcgb_u8 w3dcgb_bgq_addr_lo;
w3dcgb_u8 w3dcgb_bgq_value;
w3dcgb_u8 w3dcgb_line_color;
w3dcgb_u8 w3dcgb_occ_min_x;
w3dcgb_u8 w3dcgb_occ_min_y;
w3dcgb_u8 w3dcgb_occ_max_x;
w3dcgb_u8 w3dcgb_occ_max_y;
w3dcgb_u8 w3dcgb_mask_span_y;
w3dcgb_u8 w3dcgb_mask_span_min_x;
w3dcgb_u8 w3dcgb_mask_span_max_x;
w3dcgb_u8 w3dcgb_tri_x0;
w3dcgb_u8 w3dcgb_tri_y0;
w3dcgb_u8 w3dcgb_tri_x1;
w3dcgb_u8 w3dcgb_tri_y1;
w3dcgb_u8 w3dcgb_tri_x2;
w3dcgb_u8 w3dcgb_tri_y2;
w3dcgb_u8 w3dcgb_tri_long_x;
w3dcgb_u8 w3dcgb_tri_short_x;
w3dcgb_u8 w3dcgb_tri_long_dx;
w3dcgb_u8 w3dcgb_tri_short_dx;
w3dcgb_u8 w3dcgb_tri_long_dy;
w3dcgb_u8 w3dcgb_tri_short_dy;
w3dcgb_u8 w3dcgb_tri_long_err_lo;
w3dcgb_u8 w3dcgb_tri_long_err_hi;
w3dcgb_u8 w3dcgb_tri_short_err_lo;
w3dcgb_u8 w3dcgb_tri_short_err_hi;
w3dcgb_u8 w3dcgb_tri_long_step;
w3dcgb_u8 w3dcgb_tri_short_step;
w3dcgb_u8 w3dcgb_tri_y;
w3dcgb_u8 w3dcgb_full_frame_transfer;
w3dcgb_u8 w3dcgb_display_tile_bank;
w3dcgb_u8 w3dcgb_pending_tile_bank;
w3dcgb_u8 w3dcgb_dirty_flags[192];
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
w3dcgb_u8 w3dcgb_prev_dirty_flags[192];
#endif
w3dcgb_u8 w3dcgb_dirty_tiles[192];
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
w3dcgb_u8 w3dcgb_prev_dirty_tiles[192];
#endif
w3dcgb_u8 w3dcgb_dirty_count;
w3dcgb_u8 w3dcgb_prev_dirty_count;
w3dcgb_u8 w3dcgb_dirty_min_tile;
w3dcgb_u8 w3dcgb_dirty_max_tile;
w3dcgb_u8 w3dcgb_prev_dirty_min_tile;
w3dcgb_u8 w3dcgb_prev_dirty_max_tile;
w3dcgb_u8 w3dcgb_dirty_tile_tmp;
w3dcgb_u8 w3dcgb_dma_tile;
w3dcgb_u8 w3dcgb_dma_len;
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
w3dcgb_u8 w3dcgb_fast_map_ready;
w3dcgb_u8 w3dcgb_fast_prev_x[3];
w3dcgb_u8 w3dcgb_fast_prev_y[3];
w3dcgb_u8 w3dcgb_fast_prev_w[3];
w3dcgb_u8 w3dcgb_fast_prev_h[3];
w3dcgb_u8 w3dcgb_fast_prev_count;
w3dcgb_u8 w3dcgb_fm_x0;
w3dcgb_u8 w3dcgb_fm_y0;
w3dcgb_u8 w3dcgb_fm_x1;
w3dcgb_u8 w3dcgb_fm_y1;
w3dcgb_u8 w3dcgb_fm_count_x;
w3dcgb_u8 w3dcgb_fm_count_y;
w3dcgb_u8 w3dcgb_fm_tile_base;
w3dcgb_u8 w3dcgb_fm_sides;
w3dcgb_u8 w3dcgb_fm_stamp_x;
w3dcgb_u8 w3dcgb_fm_stamp_y;
w3dcgb_u8 w3dcgb_fm_stamp_tile;
w3dcgb_u8 w3dcgb_fm_stamp_attr;
#endif
w3dcgb_u8 w3dcgb_full_mode;
w3dcgb_u8 w3dcgb_full_tile_count;
w3dcgb_u8 w3dcgb_full_prev_tile_count;
w3dcgb_u8 w3dcgb_full_cache_tx;
w3dcgb_u8 w3dcgb_full_cache_ty;
w3dcgb_u8 w3dcgb_full_cache_slot;
w3dcgb_u8 w3dcgb_full_overflow;
w3dcgb_u8 w3dcgb_full_map_tile;

void w3dcgb_clear_vram_asm();
void w3dcgb_clear_sparse_stage_asm();
void w3dcgb_restore_hud_blank_tile_asm();
void w3dcgb_clear_frame_tiles_vram_asm();
void w3dcgb_fill_bg_map_asm();
void w3dcgb_fill_attr_map_asm();
void w3dcgb_clear_occlusion_mask_asm();
void w3dcgb_mask_set_span_asm();
void w3dcgb_stage_clear_span_asm();
void w3dcgb_mark_triangle_asm();
void w3dcgb_put_bg_tile_safe_asm();
void w3dcgb_cgb_speed_switch_asm();
void w3dcgb_fast_map_flush_gdma_asm();
void w3dcgb_fast_map_flush_tiles_gdma_asm();
void w3dcgb_fast_map_clear_rect_asm();
void w3dcgb_fast_map_rect_asm();
void w3dcgb_fast_map_clear_attr_rect_asm();
void w3dcgb_fast_map_stamp_4x4_asm();
void w3dcgb_erase_left_guard16_asm();
void w3dcgb_erase_left_guard24_asm();
void w3dcgb_draw_white_border_asm();
void w3dcgb_select_blank_render_bank_asm();
void w3dcgb_present_stage_asm();
void w3dcgb_full_clear_map_asm();
void w3dcgb_full_clear_spans_asm();
void w3dcgb_full_begin_asm();
void w3dcgb_full_span_insert_asm();
void w3dcgb_full_line_asm();
void w3dcgb_full_line_fast_asm();
void w3dcgb_full_transfer_asm();
void w3dcgb_full_present_asm();

static void w3dcgb_fast_map_prepare();

// Clamp a signed value to the inclusive interval; callers supply lo <= hi.
static w3dcgb_i16 w3dcgb_clamp_i16(w3dcgb_i16 v, w3dcgb_i16 lo, w3dcgb_i16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

// Clamp a signed screen coordinate to 0..max before narrowing it to a byte.
static w3dcgb_u8 w3dcgb_clamp_screen(w3dcgb_i16 v, w3dcgb_u8 max)
{
    if (v < 0) return 0;
    if (v > (w3dcgb_i16)max) return max;
    return (w3dcgb_u8)v;
}

// Negate a stored angle; rotation helpers subsequently wrap it to 16 steps.
static w3dcgb_i8 w3dcgb_neg_angle(w3dcgb_i8 v)
{
    return (w3dcgb_i8)(0 - v);
}

// Rotate X/Z using the low four angle bits and Q6 sine/cosine coefficients.
// Clamp both inputs to +/-220 before multiplication; valid distinct pointers
// receive the rotated components. This is a bounded fixed-point transform.
static void w3dcgb_rotate_y(w3dcgb_i16* px, w3dcgb_i16* pz, w3dcgb_i8 angle)
{
    w3dcgb_u8 ai;
    w3dcgb_i16 s;
    w3dcgb_i16 c;
    w3dcgb_i16 limit;
    w3dcgb_i16 in_x;
    w3dcgb_i16 in_z;
    w3dcgb_i16 out_x;
    w3dcgb_i16 out_z;

    ai = (w3dcgb_u8)angle;
    ai = (w3dcgb_u8)(ai & 15);
    s = w3dcgb_sin_q6[(__safe_index w3dcgb_u8)ai];
    c = w3dcgb_cos_q6[(__safe_index w3dcgb_u8)ai];
    limit = W3DCGB_TRANSFORM_LIMIT;
    in_x = w3dcgb_clamp_i16(*px, (w3dcgb_i16)(0 - limit), limit);
    in_z = w3dcgb_clamp_i16(*pz, (w3dcgb_i16)(0 - limit), limit);
    out_x = (w3dcgb_i16)(((in_x * c) + (in_z * s)) >> 6);
    out_z = (w3dcgb_i16)(((in_z * c) - (in_x * s)) >> 6);
    *px = out_x;
    *pz = out_z;
}

// Rotate Y/Z using 16 angles per revolution and Q6 coefficients. Clamp inputs
// to +/-220 and write through valid distinct pointers; other components are unchanged.
static void w3dcgb_rotate_x(w3dcgb_i16* py, w3dcgb_i16* pz, w3dcgb_i8 angle)
{
    w3dcgb_u8 ai;
    w3dcgb_i16 s;
    w3dcgb_i16 c;
    w3dcgb_i16 limit;
    w3dcgb_i16 in_y;
    w3dcgb_i16 in_z;
    w3dcgb_i16 out_y;
    w3dcgb_i16 out_z;

    ai = (w3dcgb_u8)angle;
    ai = (w3dcgb_u8)(ai & 15);
    s = w3dcgb_sin_q6[(__safe_index w3dcgb_u8)ai];
    c = w3dcgb_cos_q6[(__safe_index w3dcgb_u8)ai];
    limit = W3DCGB_TRANSFORM_LIMIT;
    in_y = w3dcgb_clamp_i16(*py, (w3dcgb_i16)(0 - limit), limit);
    in_z = w3dcgb_clamp_i16(*pz, (w3dcgb_i16)(0 - limit), limit);
    out_y = (w3dcgb_i16)(((in_y * c) - (in_z * s)) >> 6);
    out_z = (w3dcgb_i16)(((in_y * s) + (in_z * c)) >> 6);
    *py = out_y;
    *pz = out_z;
}

// Rotate X/Y using the low four angle bits and Q6 coefficients. Clamp inputs
// to +/-220 and update both pointed-to components; pointers must be valid and distinct.
static void w3dcgb_rotate_z(w3dcgb_i16* px, w3dcgb_i16* py, w3dcgb_i8 angle)
{
    w3dcgb_u8 ai;
    w3dcgb_i16 s;
    w3dcgb_i16 c;
    w3dcgb_i16 limit;
    w3dcgb_i16 in_x;
    w3dcgb_i16 in_y;
    w3dcgb_i16 out_x;
    w3dcgb_i16 out_y;

    ai = (w3dcgb_u8)angle;
    ai = (w3dcgb_u8)(ai & 15);
    s = w3dcgb_sin_q6[(__safe_index w3dcgb_u8)ai];
    c = w3dcgb_cos_q6[(__safe_index w3dcgb_u8)ai];
    limit = W3DCGB_TRANSFORM_LIMIT;
    in_x = w3dcgb_clamp_i16(*px, (w3dcgb_i16)(0 - limit), limit);
    in_y = w3dcgb_clamp_i16(*py, (w3dcgb_i16)(0 - limit), limit);
    out_x = (w3dcgb_i16)(((in_x * c) - (in_y * s)) >> 6);
    out_y = (w3dcgb_i16)(((in_x * s) + (in_y * c)) >> 6);
    *px = out_x;
    *py = out_y;
}

// Reject depth outside 8..255 without writing outputs. Otherwise clamp X/Y to
// +/-120, project with the reciprocal-depth table, and clamp pixels to 128x96.
// This pins off-screen points to the border rather than geometrically clipping
// their edges. Both output pointers must be writable; success returns one.
static w3dcgb_u8 w3dcgb_project_camera_space(w3dcgb_i16 vx, w3dcgb_i16 vy, w3dcgb_i16 vz, w3dcgb_u8* sx, w3dcgb_u8* sy)
{
    w3dcgb_u8 iz;
    w3dcgb_i16 limit;
    w3dcgb_i16 px;
    w3dcgb_i16 py;
    w3dcgb_i16 ox;
    w3dcgb_i16 oy;

    if (vz < W3DCGB_NEAR_Z) return 0;
    if (vz > W3DCGB_FAR_Z) return 0;

    limit = W3DCGB_PROJECT_LIMIT;
    px = w3dcgb_clamp_i16(vx, (w3dcgb_i16)(0 - limit), limit);
    py = w3dcgb_clamp_i16(vy, (w3dcgb_i16)(0 - limit), limit);
    iz = w3dcgb_inv_depth[(__safe_index w3dcgb_u8)((w3dcgb_u8)vz)];

    ox = (w3dcgb_i16)((px * (w3dcgb_i16)iz) >> 5);
    oy = (w3dcgb_i16)((py * (w3dcgb_i16)iz) >> 5);

    *sx = w3dcgb_clamp_screen((w3dcgb_i16)(W3DCGB_CENTER_X + ox), (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1));
    *sy = w3dcgb_clamp_screen((w3dcgb_i16)(W3DCGB_CENTER_Y - oy), (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1));
    return 1;
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Subtract camera position, then apply negative yaw, pitch and roll before
// projection. Keep differences representable as s16. Rotation input clamps
// apply at every axis; the projection result remains a 128x96 coordinate.
static w3dcgb_u8 w3dcgb_project_world(w3dcgb_i16 wx, w3dcgb_i16 wy, w3dcgb_i16 wz, w3dcgb_u8* sx, w3dcgb_u8* sy)
{
    w3dcgb_i16 vx;
    w3dcgb_i16 vy;
    w3dcgb_i16 vz;

    vx = (w3dcgb_i16)(wx - w3dcgb_cam_x);
    vy = (w3dcgb_i16)(wy - w3dcgb_cam_y);
    vz = (w3dcgb_i16)(wz - w3dcgb_cam_z);

    w3dcgb_rotate_y(&vx, &vz, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_yaw));
    w3dcgb_rotate_x(&vy, &vz, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_pitch));
    w3dcgb_rotate_z(&vx, &vy, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_roll));

    return w3dcgb_project_camera_space(vx, vy, vz, sx, sy);
}

// Project a world-space point through the current camera to the 128x96 viewport.
// Return zero for rejected depth and leave outputs unchanged; otherwise write
// clamped byte coordinates. Valid writable pointers are required. This is not
// the 160x144 full-screen projector. Absent from MINIMAL_RUNTIME builds.
w3dcgb_u8 Wire3DCGB_ProjectPoint(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_u8* sx, w3dcgb_u8* sy)
{
    return w3dcgb_project_world(x, y, z, sx, sy);
}

// Subtract camera position and project to 128x96 while ignoring all camera angles.
// Use when camera pitch, yaw and roll are zero. Reject invalid depth without
// changing output pointers; successful coordinates are clamped to the viewport.
// Absent from MINIMAL_RUNTIME builds.
w3dcgb_u8 Wire3DCGB_ProjectPointNoRotation(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z,
                                            w3dcgb_u8* sx, w3dcgb_u8* sy)
{
    return w3dcgb_project_camera_space((w3dcgb_i16)(x - w3dcgb_cam_x),
                                        (w3dcgb_i16)(y - w3dcgb_cam_y),
                                        (w3dcgb_i16)(z - w3dcgb_cam_z), sx, sy);
}
#endif

#pragma bank 2
#pragma bank 4
__prg_rom w3dcgb_u8 w3dcgb_fast_map_tile_patterns[768] = {
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00,
    0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00,
    0xFF, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00,
    0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xFF, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00,
    0xFF, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0xFF, 0x00,
    0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00,
    0xFF, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00,
    0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0xFF, 0x00,
    0xFF, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0xFF, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
    0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
    0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80,
    0x00, 0xFF, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80,
    0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xFF,
    0x00, 0xFF, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xFF,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
    0x00, 0xFF, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0xFF,
    0x00, 0xFF, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0xFF,
    0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81,
    0x00, 0xFF, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81,
    0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0xFF,
    0x00, 0xFF, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0x81, 0x00, 0xFF,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0xFF, 0xFF, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF,
    0xFF, 0xFF, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xFF, 0xFF,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0xFF, 0xFF, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0xFF, 0xFF,
    0xFF, 0xFF, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0xFF, 0xFF,
    0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81,
    0xFF, 0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81,
    0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF, 0xFF,
    0xFF, 0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF, 0xFF
};

// Select VRAM bank zero and copy the 768-byte precomputed tile table to 0x9400.
// The LCD must be off and the ROM table readable. VBK is left at zero; no STAT
// checks or previous-bank restoration occur in this startup-only path.
static void w3dcgb_load_fast_map_tiles_lite_asm()
{
    __asm {
        XOR_A
        LDH_MEM_A 79
        LD_DE_IMM w3dcgb_fast_map_tile_patterns
        LD_HL_IMM 0x9400
        LD_B_IMM 3
w3dfm_lite_page:
        LD_C_IMM 0
w3dfm_lite_loop:
        LD_A_DE
        LDI_HL_A
        INC_DE
        DEC_C
        JR_NZ w3dfm_lite_loop
        DEC_B
        JR_NZ w3dfm_lite_page
        RET
    }
}

#pragma bank 2
// Copy 14 two-bitplane HUD tiles to 0x8800 in the currently selected VRAM bank.
// Call during LCD-off initialization; this helper neither selects VBK nor polls STAT.
static void w3dcgb_load_hud_tiles()
{
    w3dcgb_u16 src;
    w3dcgb_u16 dst;

    src = 0;
    dst = 0x0800;
    while (src < (w3dcgb_u16)(W3DCGB_HUD_TILE_COUNT * 16))
    {
        w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)dst] = w3dcgb_hud_tiles[(__safe_index w3dcgb_u16)src];
        src = (w3dcgb_u16)(src + 1);
        dst = (w3dcgb_u16)(dst + 1);
    }
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Generate one row of an outline tile: mask bits 1/2/4/8 select top/bottom/left/right.
// Color bits select the two bitplanes. Call with color 1..3, mask 0..15 and
// row 0..7 while VRAM is writable; this stores directly in the 0x9400 tile region.
static void w3dcgb_load_fast_map_row(w3dcgb_u8 color, w3dcgb_u8 mask, w3dcgb_u8 row)
{
    w3dcgb_u16 dst;
    w3dcgb_u8 bits;
    w3dcgb_u8 lo;
    w3dcgb_u8 hi;

    bits = 0;
    if (((mask & 1) != 0) && (row == 0)) bits = (w3dcgb_u8)(bits | 0xFF);
    if (((mask & 2) != 0) && (row == 7)) bits = (w3dcgb_u8)(bits | 0xFF);
    if ((mask & 4) != 0) bits = (w3dcgb_u8)(bits | 0x80);
    if ((mask & 8) != 0) bits = (w3dcgb_u8)(bits | 0x01);

    lo = 0;
    hi = 0;
    if ((color & 1) != 0) lo = bits;
    if ((color & 2) != 0) hi = bits;

    dst = (w3dcgb_u16)(0x1000 + (((w3dcgb_u16)(W3DCGB_FAST_TILE_BASE + ((color - 1) << 4) + mask) << 4) + ((w3dcgb_u16)row << 1)));
    w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)dst] = lo;
    w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)(dst + 1)] = hi;
}

// Generate all eight rows for one border mask and color. Inherit the row
// helper contract: color 1..3, mask 0..15 and unrestricted VRAM access.
static void w3dcgb_load_fast_map_mask(w3dcgb_u8 color, w3dcgb_u8 mask)
{
    w3dcgb_load_fast_map_row(color, mask, 0);
    w3dcgb_load_fast_map_row(color, mask, 1);
    w3dcgb_load_fast_map_row(color, mask, 2);
    w3dcgb_load_fast_map_row(color, mask, 3);
    w3dcgb_load_fast_map_row(color, mask, 4);
    w3dcgb_load_fast_map_row(color, mask, 5);
    w3dcgb_load_fast_map_row(color, mask, 6);
    w3dcgb_load_fast_map_row(color, mask, 7);
}

// Generate a border row plus a rising two-pixel diagonal, then write its
// bitplanes directly. Supply row 0..7 and a four-bit border mask. Only color 1
// stays in tile VRAM: the current address formula places colors 2/3 in BG-map
// space. The normal tile loader does not use this helper.
static void w3dcgb_load_fast_diag_row(w3dcgb_u8 color, w3dcgb_u8 mask, w3dcgb_u8 row)
{
    w3dcgb_u16 dst;
    w3dcgb_u8 bits;
    w3dcgb_u8 lo;
    w3dcgb_u8 hi;

    bits = 0;
    if (((mask & 1) != 0) && (row == 0)) bits = (w3dcgb_u8)(bits | 0xFF);
    if (((mask & 2) != 0) && (row == 7)) bits = (w3dcgb_u8)(bits | 0xFF);
    if ((mask & 4) != 0) bits = (w3dcgb_u8)(bits | 0x80);
    if ((mask & 8) != 0) bits = (w3dcgb_u8)(bits | 0x01);

    if (row == 0) bits = (w3dcgb_u8)(bits | 0x03);
    else if (row == 1) bits = (w3dcgb_u8)(bits | 0x06);
    else if (row == 2) bits = (w3dcgb_u8)(bits | 0x0C);
    else if (row == 3) bits = (w3dcgb_u8)(bits | 0x18);
    else if (row == 4) bits = (w3dcgb_u8)(bits | 0x30);
    else if (row == 5) bits = (w3dcgb_u8)(bits | 0x60);
    else if (row == 6) bits = (w3dcgb_u8)(bits | 0xC0);
    else bits = (w3dcgb_u8)(bits | 0x80);

    lo = 0;
    hi = 0;
    if ((color & 1) != 0) lo = bits;
    if ((color & 2) != 0) hi = bits;

    dst = (w3dcgb_u16)(0x1000 + (((w3dcgb_u16)(W3DCGB_FAST_DIAG_BASE + ((color - 1) << 4) + (mask & 15)) << 4) + ((w3dcgb_u16)row << 1)));
    w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)dst] = lo;
    w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)(dst + 1)] = hi;
}

// Generate eight diagonal rows using the helper above. Only color 1 keeps
// its computed destination inside tile VRAM; this path is not used by normal setup.
static void w3dcgb_load_fast_diag(w3dcgb_u8 color, w3dcgb_u8 mask)
{
    w3dcgb_load_fast_diag_row(color, mask, 0);
    w3dcgb_load_fast_diag_row(color, mask, 1);
    w3dcgb_load_fast_diag_row(color, mask, 2);
    w3dcgb_load_fast_diag_row(color, mask, 3);
    w3dcgb_load_fast_diag_row(color, mask, 4);
    w3dcgb_load_fast_diag_row(color, mask, 5);
    w3dcgb_load_fast_diag_row(color, mask, 6);
    w3dcgb_load_fast_diag_row(color, mask, 7);
}

// Generate all 16 border masks for one color, with the LCD off and VBK already
// selected. The caller supplies color 1..3.
static void w3dcgb_load_fast_map_color(w3dcgb_u8 color)
{
    w3dcgb_u8 mask;

    mask = 0;
    while (mask < 16)
    {
        w3dcgb_load_fast_map_mask(color, mask);
        mask = (w3dcgb_u8)(mask + 1);
    }
}

// Generate 48 outline tiles for colors 1, 2 and 3. VRAM and its bank must
// be prepared by the caller; no diagonal or cube-stamp tiles are loaded here.
static void w3dcgb_load_fast_map_tiles()
{
    w3dcgb_load_fast_map_color(1);
    w3dcgb_load_fast_map_color(2);
    w3dcgb_load_fast_map_color(3);
}

#pragma bank 4
// Map four cube orientations, two visibility modes and cell 0..15 to signed
// BG tile IDs. Skip IDs 0x40..0x8F, reserving the outline and HUD tile ranges.
// Only the low two variant bits and low show_all bit participate.
static w3dcgb_u8 w3dcgb_fast_stamp_tile_id_init(w3dcgb_u8 variant, w3dcgb_u8 show_all, w3dcgb_u8 cell)
{
    w3dcgb_u8 index;

    index = (w3dcgb_u8)(((variant & 3) << 5) + ((show_all & 1) << 4) + cell);
    if (index < 64) return index;
    return (w3dcgb_u8)(0x90 + (index - 64));
}

// Convert an 8-bit signed-addressing BG tile ID to its offset from 0x8000.
// IDs below 0x80 address 0x9000 upward; IDs 0x80..0xFF address 0x8800..0x8FFF.
static w3dcgb_u16 w3dcgb_tile_pattern_offset(w3dcgb_u8 tile)
{
    if (tile < 0x80) return (w3dcgb_u16)(0x1000 + ((w3dcgb_u16)tile << 4));
    return (w3dcgb_u16)((w3dcgb_u16)tile << 4);
}

// OR a global stamp X coordinate into one tile row only when it lies within
// that tile. tile_x is a column of the 4x4 stamp, so it must be 0..3.
static w3dcgb_u8 w3dcgb_stamp_row_or_x(w3dcgb_u8 bits, w3dcgb_u8 tile_x, w3dcgb_u8 gx)
{
    w3dcgb_u8 base_x;

    base_x = (w3dcgb_u8)(tile_x << 3);
    if (gx < base_x) return bits;
    if (gx >= (w3dcgb_u8)(base_x + 8)) return bits;
    return (w3dcgb_u8)(bits | w3dcgb_bit_mask[(__safe_index w3dcgb_u8)(gx & 7)]);
}

// OR the inclusive global X range into the eight pixels of stamp column
// tile_x (0..3). Bits outside the range are preserved; inputs are not reordered.
static w3dcgb_u8 w3dcgb_stamp_row_or_range(w3dcgb_u8 bits, w3dcgb_u8 tile_x, w3dcgb_u8 min_x, w3dcgb_u8 max_x)
{
    w3dcgb_u8 base_x;
    w3dcgb_u8 local_x;
    w3dcgb_u8 gx;

    base_x = (w3dcgb_u8)(tile_x << 3);
    local_x = 0;
    while (local_x < 8)
    {
        gx = (w3dcgb_u8)(base_x + local_x);
        if ((gx >= min_x) && (gx <= max_x))
        {
            bits = (w3dcgb_u8)(bits | w3dcgb_bit_mask[(__safe_index w3dcgb_u8)local_x]);
        }
        local_x = (w3dcgb_u8)(local_x + 1);
    }
    return bits;
}

// Accumulate one scanline of a horizontal, vertical or 45-degree stamp edge.
// This specialized helper assumes diagonal |dx| equals |dy|; it is not a
// general line rasterizer. Stamp coordinates lie in 0..31 and tile_x in 0..3.
static w3dcgb_u8 w3dcgb_stamp_row_or_line(w3dcgb_u8 bits, w3dcgb_u8 tile_x, w3dcgb_u8 gy, w3dcgb_u8 x0, w3dcgb_u8 y0, w3dcgb_u8 x1, w3dcgb_u8 y1)
{
    w3dcgb_u8 min_v;
    w3dcgb_u8 max_v;
    w3dcgb_i16 d;
    w3dcgb_i16 gx;

    if (y0 == y1)
    {
        if (gy != y0) return bits;
        if (x0 < x1) return w3dcgb_stamp_row_or_range(bits, tile_x, x0, x1);
        return w3dcgb_stamp_row_or_range(bits, tile_x, x1, x0);
    }
    if (x0 == x1)
    {
        if (y0 < y1)
        {
            min_v = y0;
            max_v = y1;
        }
        else
        {
            min_v = y1;
            max_v = y0;
        }
        if (gy < min_v) return bits;
        if (gy > max_v) return bits;
        return w3dcgb_stamp_row_or_x(bits, tile_x, x0);
    }

    if (y0 < y1)
    {
        if (gy < y0) return bits;
        if (gy > y1) return bits;
        d = (w3dcgb_i16)(gy - y0);
    }
    else
    {
        if (gy < y1) return bits;
        if (gy > y0) return bits;
        d = (w3dcgb_i16)(y0 - gy);
    }

    if (x0 < x1) gx = (w3dcgb_i16)(x0 + d);
    else gx = (w3dcgb_i16)(x0 - d);
    if (gx < 0) return bits;
    if (gx > 31) return bits;
    return w3dcgb_stamp_row_or_x(bits, tile_x, (w3dcgb_u8)gx);
}

// Construct one row of a 32x32 cube stamp from front/back squares separated
// by six pixels. The low two variant bits choose the offset directions. A zero
// show_all omits selected rear edges and one connector; otherwise all are drawn.
// Supply tile_x/tile_y in 0..3 and local_y in 0..7.
static w3dcgb_u8 w3dcgb_fast_stamp_row_bits(w3dcgb_u8 tile_x, w3dcgb_u8 tile_y, w3dcgb_u8 local_y, w3dcgb_u8 variant, w3dcgb_u8 show_all)
{
    w3dcgb_u8 gy;
    w3dcgb_u8 bits;
    w3dcgb_u8 fx0;
    w3dcgb_u8 fy0;
    w3dcgb_u8 fx1;
    w3dcgb_u8 fy1;
    w3dcgb_u8 bx0;
    w3dcgb_u8 by0;
    w3dcgb_u8 bx1;
    w3dcgb_u8 by1;
    w3dcgb_u8 hidden_corner;

    gy = (w3dcgb_u8)((tile_y << 3) + local_y);
    bits = 0;

    if ((variant & 1) == 0)
    {
        fx0 = 0;
        bx0 = 6;
    }
    else
    {
        fx0 = 6;
        bx0 = 0;
    }
    if ((variant & 2) == 0)
    {
        fy0 = 6;
        by0 = 0;
    }
    else
    {
        fy0 = 0;
        by0 = 6;
    }

    fx1 = (w3dcgb_u8)(fx0 + 25);
    fy1 = (w3dcgb_u8)(fy0 + 25);
    bx1 = (w3dcgb_u8)(bx0 + 25);
    by1 = (w3dcgb_u8)(by0 + 25);

    bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx0, fy0, fx1, fy0);
    bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx0, fy1, fx1, fy1);
    bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx0, fy0, fx0, fy1);
    bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx1, fy0, fx1, fy1);

    if (show_all != 0)
    {
        bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by0, bx1, by0);
        bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by1, bx1, by1);
        bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by0, bx0, by1);
        bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx1, by0, bx1, by1);
    }
    else
    {
        if ((variant & 2) == 0) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by0, bx1, by0);
        else bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by1, bx1, by1);
        if ((variant & 1) == 0) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx1, by0, bx1, by1);
        else bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, bx0, by0, bx0, by1);
    }

    hidden_corner = 0;
    // The hidden connector is on the side opposite the back-square offset.
    if ((variant & 1) != 0) hidden_corner = (w3dcgb_u8)(hidden_corner | 1);
    if ((variant & 2) == 0) hidden_corner = (w3dcgb_u8)(hidden_corner | 2);
    if ((show_all != 0) || (hidden_corner != 0)) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx0, fy0, bx0, by0);
    if ((show_all != 0) || (hidden_corner != 1)) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx1, fy0, bx1, by0);
    if ((show_all != 0) || (hidden_corner != 2)) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx0, fy1, bx0, by1);
    if ((show_all != 0) || (hidden_corner != 3)) bits = w3dcgb_stamp_row_or_line(bits, tile_x, gy, fx1, fy1, bx1, by1);

    return bits;
}

// Generate eight rows of one cube-stamp tile and write pixels into its low
// bitplane, clearing the high bitplane. Supply consistent cell/tile coordinates
// and unrestricted VRAM access; palette attributes provide the displayed color.
static void w3dcgb_load_fast_stamp_tile(w3dcgb_u8 variant, w3dcgb_u8 show_all, w3dcgb_u8 cell, w3dcgb_u8 tile_x, w3dcgb_u8 tile_y)
{
    w3dcgb_u16 dst;
    w3dcgb_u8 local_y;
    w3dcgb_u8 bits;

    dst = w3dcgb_tile_pattern_offset(w3dcgb_fast_stamp_tile_id_init(variant, show_all, cell));

    local_y = 0;
    while (local_y < 8)
    {
        bits = w3dcgb_fast_stamp_row_bits(tile_x, tile_y, local_y, variant, show_all);
        w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)dst] = bits;
        w3dcgb_vram_tiles[(__safe_index w3dcgb_u16)(dst + 1)] = 0;
        dst = (w3dcgb_u16)(dst + 2);
        local_y = (w3dcgb_u8)(local_y + 1);
    }
}

// Generate 128 tiles: four orientations times two visibility modes times
// 16 cells. Use the current VRAM bank with the LCD off; reserved tile IDs
// are skipped by the ID mapping helper.
static void w3dcgb_load_fast_stamp_tiles()
{
    w3dcgb_u8 variant;
    w3dcgb_u8 show_all;
    w3dcgb_u8 cell;
    w3dcgb_u8 tile_x;
    w3dcgb_u8 tile_y;

    variant = 0;
    while (variant < W3DCGB_FAST_STAMP_VARIANTS)
    {
        show_all = 0;
        while (show_all <= 1)
        {
            tile_y = 0;
            cell = 0;
            while (tile_y < W3DCGB_FAST_STAMP_SIZE)
            {
                tile_x = 0;
                while (tile_x < W3DCGB_FAST_STAMP_SIZE)
                {
                    w3dcgb_load_fast_stamp_tile(variant, show_all, cell, tile_x, tile_y);
                    cell = (w3dcgb_u8)(cell + 1);
                    tile_x = (w3dcgb_u8)(tile_x + 1);
                }
                tile_y = (w3dcgb_u8)(tile_y + 1);
            }
            show_all = (w3dcgb_u8)(show_all + 1);
        }
        variant = (w3dcgb_u8)(variant + 1);
    }
}
#endif

#pragma bank 1
// Queue a tile-number write for a 32x32 BG map. Ignore invalid coordinates
// and submissions after 48 pending writes. EndFrame flushes this private queue;
// atomic-map mode mirrors writes to both BG maps. Attributes are not queued.
void Wire3DCGB_PutBgTile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 tile)
{
    if (x >= 32) return;
    if (y >= 32) return;
    if (w3dcgb_bgq_count >= W3DCGB_BG_QUEUE_LIMIT) return;
    w3dcgb_bgq_x[(__safe_index w3dcgb_u8)w3dcgb_bgq_count] = x;
    w3dcgb_bgq_y[(__safe_index w3dcgb_u8)w3dcgb_bgq_count] = y;
    w3dcgb_bgq_tile[(__safe_index w3dcgb_u8)w3dcgb_bgq_count] = tile;
    w3dcgb_bgq_count = (w3dcgb_u8)(w3dcgb_bgq_count + 1);
}

// On CGB, return if KEY1 already reports double speed; otherwise request
// the speed switch and call the STOP helper. This API assumes CGB hardware
// and does not provide a monochrome fallback.
void Wire3DCGB_EnableDoubleSpeed()
{
    if ((w3dcgb_reg_key1 & 0x80) != 0) return;
    w3dcgb_reg_key1 = 1;
    w3dcgb_cgb_speed_switch_asm();
}

#pragma fixed_bank 2
// Write SCX/SCY immediately, without waiting for VBlank or changing projected
// coordinates. The caller chooses when a visible scroll change is acceptable.
void Wire3DCGB_SetScreenOffset(w3dcgb_u8 scx, w3dcgb_u8 scy)
{
    w3dcgb_reg_scx = scx;
    w3dcgb_reg_scy = scy;
}
#pragma fixed_bank -1

// Write one RGB15 entry through the auto-increment BG palette port. The
// slot wraps to 0..31. The caller provides a valid palette-access window;
// there is no STAT polling and the RGB15 argument is not masked.
static void w3dcgb_write_bg_color(w3dcgb_u8 slot, w3dcgb_u16 rgb15)
{
    w3dcgb_reg_bcps = (w3dcgb_u8)(((slot & 31) << 1) | 0x80);
    w3dcgb_reg_bcpd = (w3dcgb_u8)rgb15;
    w3dcgb_reg_bcpd = (w3dcgb_u8)(rgb15 >> 8);
}

// Set BG palette zero to the four supplied RGB15 colors. Palettes 1..3
// share color0 and repeat their respective nonzero color in entries 1..3,
// allowing monochrome stamp tiles to be recolored by attributes. Supply a
// valid palette-access window, normally LCD-off setup or VBlank.
void Wire3DCGB_SetPaletteRGB15(w3dcgb_u16 color0, w3dcgb_u16 color1, w3dcgb_u16 color2, w3dcgb_u16 color3)
{
    w3dcgb_write_bg_color(0, color0);
    w3dcgb_write_bg_color(1, color1);
    w3dcgb_write_bg_color(2, color2);
    w3dcgb_write_bg_color(3, color3);

    w3dcgb_write_bg_color(4, color0);
    w3dcgb_write_bg_color(5, color1);
    w3dcgb_write_bg_color(6, color1);
    w3dcgb_write_bg_color(7, color1);

    w3dcgb_write_bg_color(8, color0);
    w3dcgb_write_bg_color(9, color2);
    w3dcgb_write_bg_color(10, color2);
    w3dcgb_write_bg_color(11, color2);

    w3dcgb_write_bg_color(12, color0);
    w3dcgb_write_bg_color(13, color3);
    w3dcgb_write_bg_color(14, color3);
    w3dcgb_write_bg_color(15, color3);
}

#pragma fixed_bank 4
// Store color & 3. Use the named line colors 1..3: normal lines OR their
// bitplanes, so intersecting colors combine; full-screen fast lines replace
// the pixel color. Line color zero is treated as 3, not as an eraser.
// Normal-mode point drawing separately supports color zero for erasure.
void Wire3DCGB_SetLineColor(w3dcgb_u8 color)
{
    w3dcgb_line_color = (w3dcgb_u8)(color & 3);
}
#pragma fixed_bank -1

// Return the currently stored two-bit line color without changing drawing state.
w3dcgb_u8 Wire3DCGB_GetLineColor()
{
    return w3dcgb_line_color;
}

w3dcgb_u8 w3dcgb_atomic_maps;

// Write queued tile numbers to 0x9800 through the STAT-safe byte helper.
// When atomic maps are enabled, mirror each write to 0x9C00. Clear the queue
// afterward. The caller selects tile-number VRAM bank zero before flushing.
static void w3dcgb_flush_bg_queue()
{
    w3dcgb_u8 i;

    i = 0;
    while (i < w3dcgb_bgq_count)
    {
        w3dcgb_u16 off;

        off = (w3dcgb_u16)(((w3dcgb_u16)w3dcgb_bgq_y[(__safe_index w3dcgb_u8)i] << 5) + (w3dcgb_u16)w3dcgb_bgq_x[(__safe_index w3dcgb_u8)i]);
        off = (w3dcgb_u16)(off + 0x9800);
        w3dcgb_bgq_addr_hi = (w3dcgb_u8)(off >> 8);
        w3dcgb_bgq_addr_lo = (w3dcgb_u8)off;
        w3dcgb_bgq_value = w3dcgb_bgq_tile[(__safe_index w3dcgb_u8)i];
        w3dcgb_put_bg_tile_safe_asm();
        if (w3dcgb_atomic_maps != 0)
        {
            w3dcgb_bgq_addr_hi = (w3dcgb_u8)(w3dcgb_bgq_addr_hi + 4);
            w3dcgb_put_bg_tile_safe_asm();
        }
        i = (w3dcgb_u8)(i + 1);
    }
    w3dcgb_bgq_count = 0;
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Transform only an object origin into camera space and return its Z for
// ordering. This estimate ignores the model extent and scale. obj must be valid.
static w3dcgb_i16 w3dcgb_scene_object_depth(const Wire3DCGB_Object* obj)
{
    w3dcgb_i16 vx;
    w3dcgb_i16 vy;
    w3dcgb_i16 vz;

    vx = (w3dcgb_i16)(obj->x - w3dcgb_cam_x);
    vy = (w3dcgb_i16)(obj->y - w3dcgb_cam_y);
    vz = (w3dcgb_i16)(obj->z - w3dcgb_cam_z);

    w3dcgb_rotate_y(&vx, &vz, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_yaw));
    w3dcgb_rotate_x(&vy, &vz, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_pitch));
    w3dcgb_rotate_z(&vx, &vy, (w3dcgb_i8)w3dcgb_neg_angle(w3dcgb_cam_roll));

    return vz;
}
#endif

// Return signed magnitude. The value -32768 has no positive s16 representation.
static w3dcgb_i16 w3dcgb_abs_i16(w3dcgb_i16 v)
{
    if (v < 0) return (w3dcgb_i16)(0 - v);
    return v;
}

// Clear the 1536-byte mask and reset its bounding rectangle to an empty
// range. Staged pixels and the occlusion-enabled flag are not changed here.
static void w3dcgb_clear_occlusion_mask()
{
    w3dcgb_clear_occlusion_mask_asm();
    w3dcgb_occ_min_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    w3dcgb_occ_min_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    w3dcgb_occ_max_x = 0;
    w3dcgb_occ_max_y = 0;
}

// Ignore indices outside the 192 column-major tiles. Sparse transfer tracks
// only min/max tile IDs; the full-transfer path records each ID once in a
// bounded flag/list pair. No pixels or VRAM are modified.
static void w3dcgb_mark_dirty_tile(w3dcgb_u8 tile)
{
    if (tile >= W3DCGB_DIRTY_TILE_LIMIT) return;
    if (w3dcgb_full_frame_transfer == 0)
    {
        if (tile < w3dcgb_dirty_min_tile) w3dcgb_dirty_min_tile = tile;
        if (tile > w3dcgb_dirty_max_tile) w3dcgb_dirty_max_tile = tile;
        return;
    }
    if (w3dcgb_dirty_flags[(__safe_index w3dcgb_u8)tile] != 0) return;
    if (w3dcgb_dirty_count >= W3DCGB_DIRTY_TILE_LIMIT) return;
    w3dcgb_dirty_flags[(__safe_index w3dcgb_u8)tile] = 1;
    w3dcgb_dirty_tiles[(__safe_index w3dcgb_u8)w3dcgb_dirty_count] = tile;
    w3dcgb_dirty_count = (w3dcgb_u8)(w3dcgb_dirty_count + 1);
}

// Clamp endpoints to 128x96 and mark their inclusive tile rectangle. Supply
// ordered minima/maxima. Sparse mode expands a single column-major ID range;
// the other path visits every tile in the rectangle.
static void w3dcgb_mark_dirty_rect(w3dcgb_u8 min_x, w3dcgb_u8 min_y, w3dcgb_u8 max_x, w3dcgb_u8 max_y)
{
    w3dcgb_u8 tx;
    w3dcgb_u8 ty;
    w3dcgb_u8 tx1;
    w3dcgb_u8 ty1;
    w3dcgb_u8 min_tile;
    w3dcgb_u8 max_tile;

    if (min_x >= WIRE3DCGB_SCREEN_W) min_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    if (max_x >= WIRE3DCGB_SCREEN_W) max_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    if (min_y >= WIRE3DCGB_SCREEN_H) min_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    if (max_y >= WIRE3DCGB_SCREEN_H) max_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);

    tx = (w3dcgb_u8)(min_x >> 3);
    tx1 = (w3dcgb_u8)(max_x >> 3);
    ty = (w3dcgb_u8)(min_y >> 3);
    ty1 = (w3dcgb_u8)(max_y >> 3);

    if (w3dcgb_full_frame_transfer == 0)
    {
        min_tile = (w3dcgb_u8)((tx * W3DCGB_TILE_H) + ty);
        max_tile = (w3dcgb_u8)((tx1 * W3DCGB_TILE_H) + ty1);
        if (min_tile < w3dcgb_dirty_min_tile) w3dcgb_dirty_min_tile = min_tile;
        if (max_tile > w3dcgb_dirty_max_tile) w3dcgb_dirty_max_tile = max_tile;
        return;
    }

    while (tx <= tx1)
    {
        ty = (w3dcgb_u8)(min_y >> 3);
        while (ty <= ty1)
        {
            w3dcgb_mark_dirty_tile((w3dcgb_u8)((tx * W3DCGB_TILE_H) + ty));
            if (ty == ty1) break;
            ty = (w3dcgb_u8)(ty + 1);
        }
        if (tx == tx1) break;
        tx = (w3dcgb_u8)(tx + 1);
    }
}

// Mark an inclusive pixel rectangle for later transfer without drawing it.
// Supply ordered endpoints; values beyond 127/95 are clamped to the border.
// This tracks 128x96 tiles, not the full-screen tile allocator.
void Wire3DCGB_MarkDirtyRect2D(w3dcgb_u8 min_x, w3dcgb_u8 min_y, w3dcgb_u8 max_x, w3dcgb_u8 max_y)
{
    w3dcgb_mark_dirty_rect(min_x, min_y, max_x, max_y);
}

// Order both axes, then mark the endpoint bounding rectangle. No extra
// tile-wide margin is added around a line.
static void w3dcgb_mark_dirty_line(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by)
{
    w3dcgb_u8 min_x;
    w3dcgb_u8 min_y;
    w3dcgb_u8 max_x;
    w3dcgb_u8 max_y;

    if (ax < bx)
    {
        min_x = ax;
        max_x = bx;
    }
    else
    {
        min_x = bx;
        max_x = ax;
    }

    if (ay < by)
    {
        min_y = ay;
        max_y = by;
    }
    else
    {
        min_y = by;
        max_y = ay;
    }

    /* Bresenham stays inside its endpoint rectangle; no tile-wide padding. */
    w3dcgb_mark_dirty_rect(min_x, min_y, max_x, max_y);
}

/* Sparse frames alternate VRAM banks. The back bank contains frame N-2. */
// Sparse uploads need current and two-frame-old bounds because the destination
// VRAM bank alternates. Previous-frame bounds rotate into this history after upload.
w3dcgb_u8 w3dcgb_older_dirty_min_tile;
w3dcgb_u8 w3dcgb_older_dirty_max_tile;

// Save SVBK and select WRAM bank 2. Transfer dma_len 16-byte tiles from
// D300 + tile*16 to 8900 + tile*16 in the caller-selected VRAM bank. Early
// VBlank uses GDMA for up to 96 blocks; other starts wait for mode 2 and use
// HBlank DMA. Wait for completion, then restore SVBK. Valid runs must fit
// 192 tiles and require an enabled LCD for the mode-2 path. The routine
// re-enables interrupts after starting DMA rather than preserving IME; handlers
// must keep SVBK/VBK stable until completion. Tile IDs >=192 return untouched.
void w3dcgb_transfer_dirty_tile_gdma_asm()
{
    __asm {
w3dgma_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112

        LD_A_MEM w3dcgb_dma_tile
        CP_IMM 0xC0
        JP_NC w3dgma_ret

w3dgma_safe:

        LD_A_MEM w3dcgb_dma_tile
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_C_A
        LDH_MEM_A 82
        LDH_MEM_A 84

        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_C_A
        ADD_A_IMM 0xD3
        LDH_MEM_A 81

        LD_A_C
        ADD_A_IMM 0x89
        LDH_MEM_A 83

        LD_A_MEM w3dcgb_dma_len
        OR_A
        JP_NZ w3dgma_len_ok
        LD_A_IMM 1
        LD_MEM_A w3dcgb_dma_len

w3dgma_len_ok:
        DI
        /* 96 blocks take 3072 PPU dots. Starting on LY 144/145 leaves
           at least a full scanline before the next visible frame. */
        LD_A_MEM w3dcgb_dma_len
        CP_IMM 97
        JR_NC w3dgma_wait_not_hblank
        LDH_A_MEM 68
        CP_IMM 144
        JR_C w3dgma_wait_not_hblank
        CP_IMM 146
        JR_NC w3dgma_wait_not_hblank
        LD_A_MEM w3dcgb_dma_len
        DEC_A
        LDH_MEM_A 85
        EI
        JP w3dgma_wait_done
        /* Never start HDMA in mode 0. Keep SVBK/VBK stable until complete. */
w3dgma_wait_not_hblank:
        LDH_A_MEM 65
        AND_IMM 3
        CP_IMM 2
        JR_NZ w3dgma_wait_not_hblank
w3dgma_start_safe:
        LD_A_MEM w3dcgb_dma_len
        DEC_A
        OR_IMM 0x80
        LDH_MEM_A 85
        EI

w3dgma_wait_done:
        LDH_A_MEM 85
        AND_IMM 0x80
        JP_Z w3dgma_wait_done

w3dgma_ret:
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

// Ignore invalid starting tile or zero length, cap length at 96, and invoke
// the DMA helper. The caller ensures tile + length <=192, an enabled LCD,
// a suitable destination VBK and handlers compatible with the DMA bank window.
static void w3dcgb_transfer_dirty_run(w3dcgb_u8 tile, w3dcgb_u8 len)
{
    if (tile >= W3DCGB_DIRTY_TILE_LIMIT) return;
    if (len == 0) return;
    if (len > 96) len = 96;
    w3dcgb_dma_tile = tile;
    w3dcgb_dma_len = len;
    w3dcgb_transfer_dirty_tile_gdma_asm();
}

// Upload to the opposite VRAM tile bank using the union of current and
// N-2 dirty ranges: the back bank still contains frame N-2. Split the inclusive
// range into runs of at most 96 tiles, advance history, clear the current range
// and leave VBK zero. Presentation is separate; this does not expose the bank.
static void w3dcgb_transfer_dirty_tiles()
{
    w3dcgb_u8 tile;
    w3dcgb_u8 run_len;
    w3dcgb_u8 min_tile;
    w3dcgb_u8 max_tile;

    /* Upload off-screen, then expose a complete image in one VBlank. */
    w3dcgb_pending_tile_bank = (w3dcgb_u8)(w3dcgb_display_tile_bank ^ 1);
    w3dcgb_reg_vbk = w3dcgb_pending_tile_bank;
    min_tile = w3dcgb_dirty_min_tile;
    max_tile = w3dcgb_dirty_max_tile;
    if (w3dcgb_older_dirty_min_tile < min_tile) min_tile = w3dcgb_older_dirty_min_tile;
    if (w3dcgb_older_dirty_max_tile > max_tile) max_tile = w3dcgb_older_dirty_max_tile;

    if (min_tile != 0xFF)
    {
        tile = min_tile;
        while (tile <= max_tile)
        {
            run_len = (w3dcgb_u8)(max_tile - tile + 1);
            if (run_len > 96) run_len = 96;
            w3dcgb_transfer_dirty_run(tile, run_len);
            if (run_len > (w3dcgb_u8)(max_tile - tile)) break;
            tile = (w3dcgb_u8)(tile + run_len);
        }
    }
    w3dcgb_older_dirty_min_tile = w3dcgb_prev_dirty_min_tile;
    w3dcgb_older_dirty_max_tile = w3dcgb_prev_dirty_max_tile;
    w3dcgb_prev_dirty_min_tile = w3dcgb_dirty_min_tile;
    w3dcgb_prev_dirty_max_tile = w3dcgb_dirty_max_tile;
    w3dcgb_dirty_min_tile = 0xFF;
    w3dcgb_dirty_max_tile = 0;
    w3dcgb_reg_vbk = 0;
}
#pragma bank 2

__prg_rom w3dcgb_u8 w3dcgb_span_start_mask[8] = {
    0xFF, 0x7F, 0x3F, 0x1F, 0x0F, 0x07, 0x03, 0x01
};

__prg_rom w3dcgb_u8 w3dcgb_span_end_mask[8] = {
    0x80, 0xC0, 0xE0, 0xF0, 0xF8, 0xFC, 0xFE, 0xFF
};

#pragma bank 4
// Map a validated 128x96 coordinate to its row-major 1bpp mask byte: y*16+x/8.
static w3dcgb_u16 w3dcgb_mask_offset(w3dcgb_u8 x, w3dcgb_u8 y)
{
    return (w3dcgb_u16)(((w3dcgb_u16)y << 4) + (w3dcgb_u16)(x >> 3));
}

// Return one for covered or out-of-bounds pixels, zero for an uncovered
// 128x96 pixel. This reads the mask only; it does not inspect staged color.
static w3dcgb_u8 w3dcgb_mask_get(w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u16 ofs;
    w3dcgb_u8 bit;

    if (x >= WIRE3DCGB_SCREEN_W) return 1;
    if (y >= WIRE3DCGB_SCREEN_H) return 1;

    ofs = w3dcgb_mask_offset(x, y);
    bit = w3dcgb_bit_mask[(__safe_index w3dcgb_u8)(x & 7)];
    if ((w3dcgb_occlusion_mask[(__safe_index w3dcgb_u16)ofs] & bit) != 0) return 1;
    return 0;
}

// Clear both bitplanes of one 128x96 staged pixel and mark its tile dirty.
// Invalid coordinates are ignored. This C path does not select SVBK; the
// caller or bank-aware allocation must provide the correct stage mapping.
static void w3dcgb_stage_clear_pixel(w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u16 ofs;
    w3dcgb_u8 bit;
    w3dcgb_u8 clear_mask;

    if (x >= WIRE3DCGB_SCREEN_W) return;
    if (y >= WIRE3DCGB_SCREEN_H) return;

    ofs = (w3dcgb_u16)(((((w3dcgb_u16)(x >> 3)) * W3DCGB_TILE_H + (w3dcgb_u16)(y >> 3)) << 4) + (((w3dcgb_u16)y & 7) << 1));
    w3dcgb_mark_dirty_tile((w3dcgb_u8)(((x >> 3) * W3DCGB_TILE_H) + (y >> 3)));
    bit = w3dcgb_bit_mask[(__safe_index w3dcgb_u8)(x & 7)];
    clear_mask = (w3dcgb_u8)(0xFF ^ bit);
    w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] & clear_mask);
    ofs = (w3dcgb_u16)(ofs + 1);
    w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] & clear_mask);
}

// Erase an inclusive staged span, using individual edge pixels and whole
// bitplane bytes in the middle. Supply ordered X endpoints in 0..127; only
// Y is checked here. Mark touched tiles dirty and retain the caller's WRAM mapping.
static void w3dcgb_stage_clear_span(w3dcgb_u8 y, w3dcgb_u8 min_x, w3dcgb_u8 max_x)
{
    w3dcgb_u8 x;
    w3dcgb_u16 ofs;

    if (y >= WIRE3DCGB_SCREEN_H) return;

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        w3dcgb_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3dcgb_u8)(x + 1);
    }

    while ((w3dcgb_u8)(x + 7) <= max_x)
    {
        ofs = (w3dcgb_u16)(((((w3dcgb_u16)(x >> 3)) * W3DCGB_TILE_H + (w3dcgb_u16)(y >> 3)) << 4) + (((w3dcgb_u16)y & 7) << 1));
        w3dcgb_mark_dirty_tile((w3dcgb_u8)(((x >> 3) * W3DCGB_TILE_H) + (y >> 3)));
        w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] = 0;
        w3dcgb_stage[(__safe_index w3dcgb_u16)(ofs + 1)] = 0;
        x = (w3dcgb_u8)(x + 8);
    }

    while (x <= max_x)
    {
        w3dcgb_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3dcgb_u8)(x + 1);
    }
}

// Replace one 128x96 staged pixel with the current two-bit color, so color
// zero erases it. Ignore invalid coordinates. This helper neither selects
// SVBK, marks tiles dirty nor tests occlusion; the caller handles those steps.
static void w3dcgb_plot_stage_pixel_c(w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u16 ofs;
    w3dcgb_u8 bit;
    w3dcgb_u8 clear_mask;

    if (x >= WIRE3DCGB_SCREEN_W) return;
    if (y >= WIRE3DCGB_SCREEN_H) return;

    ofs = (w3dcgb_u16)(((((w3dcgb_u16)(x >> 3)) * W3DCGB_TILE_H + (w3dcgb_u16)(y >> 3)) << 4) + (((w3dcgb_u16)y & 7) << 1));
    bit = w3dcgb_bit_mask[(__safe_index w3dcgb_u8)(x & 7)];
    clear_mask = (w3dcgb_u8)(0xFF ^ bit);

    w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] & clear_mask);
    w3dcgb_stage[(__safe_index w3dcgb_u16)(ofs + 1)] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)(ofs + 1)] & clear_mask);

    if ((w3dcgb_line_color & 1) != 0)
    {
        w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)ofs] | bit);
    }
    if ((w3dcgb_line_color & 2) != 0)
    {
        w3dcgb_stage[(__safe_index w3dcgb_u16)(ofs + 1)] = (w3dcgb_u8)(w3dcgb_stage[(__safe_index w3dcgb_u16)(ofs + 1)] | bit);
    }
}

// Pass a row and inclusive span through shared globals to the mask writer.
// Reject Y outside 0..95; callers must supply ordered X endpoints in 0..127.
// Pixels, dirty flags and the occlusion-enabled flag are not changed.
static void w3dcgb_mask_set_span(w3dcgb_u8 y, w3dcgb_u8 min_x, w3dcgb_u8 max_x)
{
    if (y >= WIRE3DCGB_SCREEN_H) return;
    w3dcgb_mask_span_y = y;
    w3dcgb_mask_span_min_x = min_x;
    w3dcgb_mask_span_max_x = max_x;
    w3dcgb_mask_set_span_asm();
}

// Widen byte coordinates before subtraction to retain a signed screen delta.
static w3dcgb_i16 w3dcgb_screen_delta(w3dcgb_u8 a, w3dcgb_u8 b)
{
    return (w3dcgb_i16)((w3dcgb_i16)a - (w3dcgb_i16)b);
}

// Return the signed 2D cross product. Keep inputs small enough that both
// products and their difference fit s16; this is not a general wide-coordinate test.
static w3dcgb_i16 w3dcgb_edge_area(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 px, w3dcgb_i16 py)
{
    return (w3dcgb_i16)(((bx - ax) * (py - ay)) - ((by - ay) * (px - ax)));
}

// Order two Q8 X intersections, round down to pixel columns, add a two-pixel
// guard on each side, clamp to 0..127 and mark the inclusive span. Invalid
// rows are ignored by the span wrapper. This helper is not the active ASM triangle path.
static void w3dcgb_mask_set_span_q8(w3dcgb_u8 y, w3dcgb_i16 left_q8, w3dcgb_i16 right_q8)
{
    w3dcgb_i16 span_min;
    w3dcgb_i16 span_max;

    if (left_q8 < right_q8)
    {
        span_min = (w3dcgb_i16)(left_q8 >> 8);
        span_max = (w3dcgb_i16)(right_q8 >> 8);
    }
    else
    {
        span_min = (w3dcgb_i16)(right_q8 >> 8);
        span_max = (w3dcgb_i16)(left_q8 >> 8);
    }

    span_min = (w3dcgb_i16)(span_min - 2);
    span_max = (w3dcgb_i16)(span_max + 2);
    if (span_min < 0) span_min = 0;
    if (span_max > (w3dcgb_i16)(WIRE3DCGB_SCREEN_W - 1)) span_max = (w3dcgb_i16)(WIRE3DCGB_SCREEN_W - 1);
    if (span_min <= span_max)
    {
        w3dcgb_mask_set_span(y, (w3dcgb_u8)span_min, (w3dcgb_u8)span_max);
    }
}

#pragma bank 2
// Copy vertices into shared scratch and invoke the active ASM scan converter.
// The disabled C reference below uses different guard widths and is not executed.
// The ASM path selects a 128x96 mask or full-screen span list by full_mode.
// Its one-pixel X guard stops at the selected viewport edge. Supply vertices
// inside that viewport; this wrapper does not clip arbitrary triangles.
static void w3dcgb_mark_triangle(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy)
{
    w3dcgb_tri_x0 = ax;
    w3dcgb_tri_y0 = ay;
    w3dcgb_tri_x1 = bx;
    w3dcgb_tri_y1 = by;
    w3dcgb_tri_x2 = cx;
    w3dcgb_tri_y2 = cy;
    w3dcgb_mark_triangle_asm();
#if 0
    w3dcgb_u8 x0;
    w3dcgb_u8 y0;
    w3dcgb_u8 x1;
    w3dcgb_u8 y1;
    w3dcgb_u8 x2;
    w3dcgb_u8 y2;
    w3dcgb_u8 min_x;
    w3dcgb_u8 max_x;
    w3dcgb_u8 long_x;
    w3dcgb_u8 short_x;
    w3dcgb_u8 long_dx;
    w3dcgb_u8 short_dx;
    w3dcgb_u8 long_dy;
    w3dcgb_u8 short_dy;
    w3dcgb_u8 long_err;
    w3dcgb_u8 short_err;
    w3dcgb_u8 long_right;
    w3dcgb_u8 short_right;
    w3dcgb_u8 span_min;
    w3dcgb_u8 span_max;
    w3dcgb_u8 y;

    x0 = ax;
    y0 = ay;
    x1 = bx;
    y1 = by;
    x2 = cx;
    y2 = cy;

    if (y0 > y1)
    {
        w3dcgb_u8 tx;
        w3dcgb_u8 ty;
        tx = x0; x0 = x1; x1 = tx;
        ty = y0; y0 = y1; y1 = ty;
    }
    if (y1 > y2)
    {
        w3dcgb_u8 tx;
        w3dcgb_u8 ty;
        tx = x1; x1 = x2; x2 = tx;
        ty = y1; y1 = y2; y2 = ty;
    }
    if (y0 > y1)
    {
        w3dcgb_u8 tx;
        w3dcgb_u8 ty;
        tx = x0; x0 = x1; x1 = tx;
        ty = y0; y0 = y1; y1 = ty;
    }

    min_x = x0;
    if (x1 < min_x) min_x = x1;
    if (x2 < min_x) min_x = x2;
    max_x = x0;
    if (x1 > max_x) max_x = x1;
    if (x2 > max_x) max_x = x2;
    if (min_x > 0) min_x = (w3dcgb_u8)(min_x - 1);
    if (max_x < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1)) max_x = (w3dcgb_u8)(max_x + 1);

    if (w3dcgb_occ_min_x > w3dcgb_occ_max_x)
    {
        w3dcgb_occ_min_x = min_x;
        w3dcgb_occ_min_y = y0;
        w3dcgb_occ_max_x = max_x;
        w3dcgb_occ_max_y = y2;
    }
    else
    {
        if (min_x < w3dcgb_occ_min_x) w3dcgb_occ_min_x = min_x;
        if (y0 < w3dcgb_occ_min_y) w3dcgb_occ_min_y = y0;
        if (max_x > w3dcgb_occ_max_x) w3dcgb_occ_max_x = max_x;
        if (y2 > w3dcgb_occ_max_y) w3dcgb_occ_max_y = y2;
    }

    if (y0 == y2)
    {
        w3dcgb_mask_set_span(y0, min_x, max_x);
        return;
    }

    long_x = x0;
    long_dy = (w3dcgb_u8)(y2 - y0);
    if (x2 >= x0)
    {
        long_dx = (w3dcgb_u8)(x2 - x0);
        long_right = 1;
    }
    else
    {
        long_dx = (w3dcgb_u8)(x0 - x2);
        long_right = 0;
    }
    long_err = 0;

    if (y1 > y0)
    {
        short_x = x0;
        short_dy = (w3dcgb_u8)(y1 - y0);
        if (x1 >= x0)
        {
            short_dx = (w3dcgb_u8)(x1 - x0);
            short_right = 1;
        }
        else
        {
            short_dx = (w3dcgb_u8)(x0 - x1);
            short_right = 0;
        }
        short_err = 0;
        y = y0;
        while (y < y1)
        {
            if (long_x < short_x) { span_min = long_x; span_max = short_x; }
            else { span_min = short_x; span_max = long_x; }
            if (span_min > 1) span_min = (w3dcgb_u8)(span_min - 2); else span_min = 0;
            if (span_max < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 2)) span_max = (w3dcgb_u8)(span_max + 2);
            else span_max = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
            w3dcgb_mask_set_span(y, span_min, span_max);

            long_err = (w3dcgb_u8)(long_err + long_dx);
            while (long_err >= long_dy)
            {
                if (long_right != 0) long_x = (w3dcgb_u8)(long_x + 1);
                else long_x = (w3dcgb_u8)(long_x - 1);
                long_err = (w3dcgb_u8)(long_err - long_dy);
            }
            short_err = (w3dcgb_u8)(short_err + short_dx);
            while (short_err >= short_dy)
            {
                if (short_right != 0) short_x = (w3dcgb_u8)(short_x + 1);
                else short_x = (w3dcgb_u8)(short_x - 1);
                short_err = (w3dcgb_u8)(short_err - short_dy);
            }
            y = (w3dcgb_u8)(y + 1);
        }
    }

    if (y2 > y1)
    {
        short_x = x1;
        short_dy = (w3dcgb_u8)(y2 - y1);
        if (x2 >= x1)
        {
            short_dx = (w3dcgb_u8)(x2 - x1);
            short_right = 1;
        }
        else
        {
            short_dx = (w3dcgb_u8)(x1 - x2);
            short_right = 0;
        }
        short_err = 0;
        y = y1;
        while (y <= y2)
        {
            if (long_x < short_x) { span_min = long_x; span_max = short_x; }
            else { span_min = short_x; span_max = long_x; }
            if (span_min > 1) span_min = (w3dcgb_u8)(span_min - 2); else span_min = 0;
            if (span_max < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 2)) span_max = (w3dcgb_u8)(span_max + 2);
            else span_max = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
            w3dcgb_mask_set_span(y, span_min, span_max);
            if (y == y2) break;

            long_err = (w3dcgb_u8)(long_err + long_dx);
            while (long_err >= long_dy)
            {
                if (long_right != 0) long_x = (w3dcgb_u8)(long_x + 1);
                else long_x = (w3dcgb_u8)(long_x - 1);
                long_err = (w3dcgb_u8)(long_err - long_dy);
            }
            short_err = (w3dcgb_u8)(short_err + short_dx);
            while (short_err >= short_dy)
            {
                if (short_right != 0) short_x = (w3dcgb_u8)(short_x + 1);
                else short_x = (w3dcgb_u8)(short_x - 1);
                short_err = (w3dcgb_u8)(short_err - short_dy);
            }
            y = (w3dcgb_u8)(y + 1);
        }
    }
    else
    {
        if (long_x < x1) { span_min = long_x; span_max = x1; }
        else { span_min = x1; span_max = long_x; }
        if (span_min > 1) span_min = (w3dcgb_u8)(span_min - 2); else span_min = 0;
        if (span_max < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 2)) span_max = (w3dcgb_u8)(span_max + 2);
        else span_max = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
        w3dcgb_mask_set_span(y2, span_min, span_max);
    }
#endif
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Clear full-screen span history when full_mode is set; otherwise clear the
// 128x96 bitmap mask and its bounds. This does not erase displayed/staged
// pixels or change whether occlusion testing is enabled.
void Wire3DCGB_ClearOcclusionMask()
{
    if (w3dcgb_full_mode != 0) w3dcgb_full_clear_spans_asm();
    else w3dcgb_clear_occlusion_mask();
}

// Normalize active to zero or one and store it as the occlusion switch.
// No mask clearing or geometry marking occurs.
void Wire3DCGB_SetOcclusionActive(w3dcgb_u8 active)
{
    if (active == 0) w3dcgb_occlusion_active = 0;
    else w3dcgb_occlusion_active = 1;
}

// Mark a screen-space triangle without drawing pixels or enabling occlusion.
// Supply vertices inside the selected viewport. The active ASM adds a one-pixel
// X guard, limited to 0..127 in 128x96 mode or 0..159 in 160x144 mode.
// This function does not clip arbitrary out-of-bounds triangles.
void Wire3DCGB_MarkTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy)
{
    w3dcgb_mark_triangle(ax, ay, bx, by, cx, cy);
}
#endif

// Mark up to 29 silhouette rows in the 128x96 bitmap mask. Each packed byte
// uses its high nibble for center offset (nibble-7), low nibble for half-width;
// 0xFF skips a row. Place rows at y+y0+i, expand spans by two pixels, and clip
// to the viewport. profile must contain count readable bytes; null is ignored.
// This does not update full-screen spans or enable occlusion.
void Wire3DCGB_MarkPackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                     w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u8 i;
    w3dcgb_u8 packed;
    w3dcgb_u8 half_width;
    w3dcgb_i16 center;
    w3dcgb_i16 left;
    w3dcgb_i16 right;
    w3dcgb_i16 row;

    if (profile == 0) return;
    if (count > 29) count = 29;
    i = 0;
    while (i < count)
    {
        packed = profile[(__safe_index w3dcgb_u8)i];
        if (packed != 0xFF)
        {
            row = (w3dcgb_i16)((w3dcgb_i16)y + (w3dcgb_i16)y0 + (w3dcgb_i16)i);
            if ((row >= 0) && (row < WIRE3DCGB_SCREEN_H))
            {
                center = (w3dcgb_i16)((w3dcgb_i16)x + (w3dcgb_i16)(packed >> 4) - 7);
                half_width = (w3dcgb_u8)(packed & 15);
                /* Two-pixel guard covers rasterized endpoints at 22.5-degree views. */
                left = (w3dcgb_i16)(center - (w3dcgb_i16)half_width - 2);
                right = (w3dcgb_i16)(center + (w3dcgb_i16)half_width + 2);
                if ((right >= 0) && (left < WIRE3DCGB_SCREEN_W))
                {
                    if (left < 0) left = 0;
                    if (right >= WIRE3DCGB_SCREEN_W) right = WIRE3DCGB_SCREEN_W - 1;
                    w3dcgb_mask_span_y = (w3dcgb_u8)row;
                    w3dcgb_mask_span_min_x = (w3dcgb_u8)left;
                    w3dcgb_mask_span_max_x = (w3dcgb_u8)right;
                    w3dcgb_mask_set_span_asm();
                }
            }
        }
        i = (w3dcgb_u8)(i + 1);
    }
}

// Erase up to 29 packed silhouette rows from both staged bitplanes. Format,
// placement, two-pixel guard and clipping match MarkPackedSilhouette2D; null
// is ignored. The ASM span writer selects/restores WRAM bank 2 but does not
// mark dirty tiles. In sparse mode the caller must ensure erased tiles are
// included in the upload range. This API targets the 128x96 stage.
void Wire3DCGB_ErasePackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                      w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u8 i;
    w3dcgb_u8 packed;
    w3dcgb_u8 half_width;
    w3dcgb_i16 center;
    w3dcgb_i16 left;
    w3dcgb_i16 right;
    w3dcgb_i16 row;

    if (profile == 0) return;
    if (count > 29) count = 29;
    i = 0;
    while (i < count)
    {
        packed = profile[(__safe_index w3dcgb_u8)i];
        if (packed != 0xFF)
        {
            row = (w3dcgb_i16)((w3dcgb_i16)y + (w3dcgb_i16)y0 + (w3dcgb_i16)i);
            if ((row >= 0) && (row < WIRE3DCGB_SCREEN_H))
            {
                center = (w3dcgb_i16)((w3dcgb_i16)x + (w3dcgb_i16)(packed >> 4) - 7);
                half_width = (w3dcgb_u8)(packed & 15);
                left = (w3dcgb_i16)(center - (w3dcgb_i16)half_width - 2);
                right = (w3dcgb_i16)(center + (w3dcgb_i16)half_width + 2);
                if ((right >= 0) && (left < WIRE3DCGB_SCREEN_W))
                {
                    if (left < 0) left = 0;
                    if (right >= WIRE3DCGB_SCREEN_W) right = WIRE3DCGB_SCREEN_W - 1;
                    w3dcgb_mask_span_y = (w3dcgb_u8)row;
                    w3dcgb_mask_span_min_x = (w3dcgb_u8)left;
                    w3dcgb_mask_span_max_x = (w3dcgb_u8)right;
                    w3dcgb_stage_clear_span_asm();
                }
            }
        }
        i = (w3dcgb_u8)(i + 1);
    }
}

#pragma bank 1
// Erase integer-intersection spans of a nondegenerate triangle, ignoring
// horizontal edges while finding intersections. X spans are clamped to 0..127
// and out-of-range rows are ignored by the stage helper. Touched tiles become
// dirty; caller/allocation must provide the correct WRAM mapping for this C path.
static void w3dcgb_clear_triangle(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy)
{
    w3dcgb_u8 min_x;
    w3dcgb_u8 max_x;
    w3dcgb_u8 min_y;
    w3dcgb_u8 max_y;
    w3dcgb_u8 y;

    min_x = ax;
    if (bx < min_x) min_x = bx;
    if (cx < min_x) min_x = cx;
    max_x = ax;
    if (bx > max_x) max_x = bx;
    if (cx > max_x) max_x = cx;

    min_y = ay;
    if (by < min_y) min_y = by;
    if (cy < min_y) min_y = cy;
    max_y = ay;
    if (by > max_y) max_y = by;
    if (cy > max_y) max_y = cy;

    if (w3dcgb_edge_area((w3dcgb_i16)ax, (w3dcgb_i16)ay, (w3dcgb_i16)bx, (w3dcgb_i16)by, (w3dcgb_i16)cx, (w3dcgb_i16)cy) == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        w3dcgb_u8 hits;
        w3dcgb_i16 span_min;
        w3dcgb_i16 span_max;
        w3dcgb_i16 dx;
        w3dcgb_i16 dy;
        w3dcgb_i16 ix;

        hits = 0;
        span_min = 127;
        span_max = 0;

        if (ay != by)
        {
            if (((y >= ay) && (y <= by)) || ((y >= by) && (y <= ay)))
            {
                dx = (w3dcgb_i16)((w3dcgb_i16)bx - (w3dcgb_i16)ax);
                dy = (w3dcgb_i16)((w3dcgb_i16)by - (w3dcgb_i16)ay);
                ix = (w3dcgb_i16)((w3dcgb_i16)ax + ((((w3dcgb_i16)y - (w3dcgb_i16)ay) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3dcgb_u8)(hits + 1);
            }
        }

        if (by != cy)
        {
            if (((y >= by) && (y <= cy)) || ((y >= cy) && (y <= by)))
            {
                dx = (w3dcgb_i16)((w3dcgb_i16)cx - (w3dcgb_i16)bx);
                dy = (w3dcgb_i16)((w3dcgb_i16)cy - (w3dcgb_i16)by);
                ix = (w3dcgb_i16)((w3dcgb_i16)bx + ((((w3dcgb_i16)y - (w3dcgb_i16)by) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3dcgb_u8)(hits + 1);
            }
        }

        if (cy != ay)
        {
            if (((y >= cy) && (y <= ay)) || ((y >= ay) && (y <= cy)))
            {
                dx = (w3dcgb_i16)((w3dcgb_i16)ax - (w3dcgb_i16)cx);
                dy = (w3dcgb_i16)((w3dcgb_i16)ay - (w3dcgb_i16)cy);
                ix = (w3dcgb_i16)((w3dcgb_i16)cx + ((((w3dcgb_i16)y - (w3dcgb_i16)cy) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3dcgb_u8)(hits + 1);
            }
        }

        if (hits >= 2)
        {
            if (span_min < (w3dcgb_i16)min_x) span_min = (w3dcgb_i16)min_x;
            if (span_max > (w3dcgb_i16)max_x) span_max = (w3dcgb_i16)max_x;
            if (span_min < 0) span_min = 0;
            if (span_max > (w3dcgb_i16)(WIRE3DCGB_SCREEN_W - 1)) span_max = (w3dcgb_i16)(WIRE3DCGB_SCREEN_W - 1);
            if (span_min <= span_max)
            {
                w3dcgb_stage_clear_span(y, (w3dcgb_u8)span_min, (w3dcgb_u8)span_max);
            }
        }

        if (y == max_y) break;
        y = (w3dcgb_u8)(y + 1);
    }
}

#pragma bank 4
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Build front-face flags from positive projected signed area. Faces with
// invalid vertex IDs or a depth-rejected vertex remain invisible. Supply a
// valid face array and counts within 24 projected vertices and 16 faces.
static void w3dcgb_build_face_visibility(const Wire3DCGB_Model* model, w3dcgb_u8 count, w3dcgb_u8 face_count)
{
    w3dcgb_u8 i;

    i = 0;
    while (i < face_count)
    {
        const Wire3DCGB_Face* f;
        w3dcgb_i16 abx;
        w3dcgb_i16 aby;
        w3dcgb_i16 acx;
        w3dcgb_i16 acy;
        w3dcgb_i16 area;

        w3dcgb_face_visible[(__safe_index w3dcgb_u8)i] = 0;
        f = &model->faces[(__safe_index w3dcgb_u8)i];

        if ((f->a < count) && (f->b < count) && (f->c < count))
        {
            if (w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->a] &&
                w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->b] &&
                w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->c])
            {
                abx = w3dcgb_screen_delta(w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->b], w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->a]);
                aby = w3dcgb_screen_delta(w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->b], w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->a]);
                acx = w3dcgb_screen_delta(w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->c], w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->a]);
                acy = w3dcgb_screen_delta(w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->c], w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->a]);
                area = (w3dcgb_i16)((abx * acy) - (aby * acx));
                if (area > 0)
                {
                    w3dcgb_face_visible[(__safe_index w3dcgb_u8)i] = 1;
                }
            }
        }
        i = (w3dcgb_u8)(i + 1);
    }
}

// Keep an edge when hidden-line filtering is off or either adjacent face is
// visible. Missing adjacency, absent faces, no adjacent face IDs, and IDs beyond
// face_count fail open. A valid model and edge adjacency entry are required.
static w3dcgb_u8 w3dcgb_is_edge_visible(const Wire3DCGB_Model* model, w3dcgb_u8 edge_index, w3dcgb_u8 face_count)
{
    const Wire3DCGB_EdgeFaces* ef;
    w3dcgb_u8 any_face;

    if ((model->flags & WIRE3DCGB_MODEL_HIDDEN_LINES) == 0) return 1;
    if (model->faces == 0) return 1;
    if (model->edge_faces == 0) return 1;

    ef = &model->edge_faces[(__safe_index w3dcgb_u8)edge_index];
    any_face = 0;

    if (ef->f0 != WIRE3DCGB_FACE_NONE)
    {
        any_face = 1;
        if (ef->f0 >= face_count) return 1;
        if (w3dcgb_face_visible[(__safe_index w3dcgb_u8)ef->f0]) return 1;
    }

    if (ef->f1 != WIRE3DCGB_FACE_NONE)
    {
        any_face = 1;
        if (ef->f1 >= face_count) return 1;
        if (w3dcgb_face_visible[(__safe_index w3dcgb_u8)ef->f1]) return 1;
    }

    if (any_face == 0) return 1;
    return 0;
}

// Use the most recent projection cache to mark faces of a hidden-line model.
// At most 24 vertices and 16 faces are considered. This marks every face whose
// vertices passed depth, including back faces; it does not consult the rebuilt
// front-face flags. Face indices must already be valid: this loop indexes
// projection arrays without repeating the visibility builder's bounds checks.
static void w3dcgb_mark_model_occluder(const Wire3DCGB_Model* model)
{
    w3dcgb_u8 count;
    w3dcgb_u8 face_count;
    w3dcgb_u8 i;

    if (model == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3DCGB_MODEL_HIDDEN_LINES) == 0) return;

    count = model->vertex_count;
    if (count > WIRE3DCGB_MODEL_VERTEX_LIMIT) count = WIRE3DCGB_MODEL_VERTEX_LIMIT;
    face_count = model->face_count;
    if (face_count > WIRE3DCGB_MODEL_FACE_LIMIT) face_count = WIRE3DCGB_MODEL_FACE_LIMIT;

    w3dcgb_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3DCGB_Face* f;
        f = &model->faces[(__safe_index w3dcgb_u8)i];
        if (w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->a] &&
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->b] &&
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->c])
        {
            w3dcgb_mark_triangle(
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->a],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->a],
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->b],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->b],
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->c],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->c]);
        }
        i = (w3dcgb_u8)(i + 1);
    }
}
#endif

#pragma bank 1
// Return if the LCD is off; otherwise wait out any current VBlank and then
// wait for the next one. This is a busy wait and does not change interrupt state.
void w3dcgb_wait_vblank_start()
{
    if ((w3dcgb_reg_lcdc & 0x80) == 0) return;
    while (w3dcgb_reg_ly >= 144) { }
    while (w3dcgb_reg_ly < 144) { }
}

#pragma fixed_bank 2
#pragma fixed_order 92
// Zero 0x8000..0x97FF in both VRAM banks and leave VBK zero. The LCD
// must be off; no STAT checks or interrupt protection occur.
void w3dcgb_clear_vram_asm()
{
    __asm {
w3dcv_enter:
        XOR_A
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_vram_tiles
        XOR_A
        LD_D_IMM 24

w3dcv_page:
        LD_C_IMM 0

w3dcv_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3dcv_loop
        DEC_D
        JR_NZ w3dcv_page
        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_vram_tiles
        XOR_A
        LD_D_IMM 24

w3dcv_page_b1:
        LD_C_IMM 0

w3dcv_loop_b1:
        LDI_HL_A
        DEC_C
        JR_NZ w3dcv_loop_b1
        DEC_D
        JR_NZ w3dcv_page_b1
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 93
// Clear the 3072 frame-tile bytes at 0x8900..0x94FF in both VRAM banks.
// Require the LCD off and leave VBK zero; tile data outside this range is retained.
void w3dcgb_clear_frame_tiles_vram_asm()
{
    __asm {
w3dcf_enter:
        XOR_A
        LDH_MEM_A 79
        LD_HL_IMM 0x8900
        XOR_A
        LD_D_IMM 12

w3dcf_page:
        LD_C_IMM 0

w3dcf_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3dcf_loop
        DEC_D
        JR_NZ w3dcf_page
        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM 0x8900
        XOR_A
        LD_D_IMM 12

w3dcf_page_b1:
        LD_C_IMM 0

w3dcf_loop_b1:
        LDI_HL_A
        DEC_C
        JR_NZ w3dcf_loop_b1
        DEC_D
        JR_NZ w3dcf_page_b1
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 94
// Fill 0x9800..0x9BFF with signed HUD blank tile ID 0x8A. The caller
// selects VBK zero and disables the LCD before entering this unchecked writer.
void w3dcgb_fill_bg_map_asm()
{
    __asm {
w3dfb_enter:
        LD_HL_IMM w3dcgb_bg_map_9800
        LD_A_IMM 0x8A
        LD_D_IMM 4

w3dfb_page:
        LD_C_IMM 0

w3dfb_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3dfb_loop
        DEC_D
        JR_NZ w3dfb_page
        RET
    }
}

#pragma fixed_order 95
// Select VBK one, clear 1024 attributes for BG map 0x9800, and leave VBK
// zero. The LCD must be off; the other BG map is not changed.
void w3dcgb_fill_attr_map_asm()
{
    __asm {
w3dfa_enter:
        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_bg_map_9800
        XOR_A
        LD_D_IMM 4

w3dfa_page:
        LD_C_IMM 0

w3dfa_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3dfa_loop
        DEC_D
        JR_NZ w3dfa_page

        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 96
// Zero the 1536-byte bitmap mask in 96 groups of 16 stores. No bounding
// metadata, pixel stage or occlusion-enabled flag is changed here.
void w3dcgb_clear_occlusion_mask_asm()
{
    __asm {
w3dco_enter:
        LD_HL_IMM w3dcgb_occlusion_mask
        XOR_A
        LD_B_IMM 96

w3dco_loop:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        DEC_B
        JR_NZ w3dco_loop
        RET
    }
}

#pragma fixed_order 97
// OR an inclusive row span into the 1536-byte row-major mask. Preserve bits
// outside the first/last partial bytes and fill complete middle bytes with FF.
// Shared endpoints must be ordered in 0..127 and Y in 0..95; this low-level
// writer performs no bounds checks and does not update mask bounding metadata.
void w3dcgb_mask_set_span_asm()
{
    __asm {
w3dmss_enter:
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A

        LD_A_MEM w3dcgb_mask_span_max_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_C_A

        LD_A_MEM w3dcgb_mask_span_y
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_E_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_D_A
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE

        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE

        LD_A_MEM w3dcgb_mask_span_max_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        CP_B
        JP_Z w3dmss_single

        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_min_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_start_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        LD_A_HL
        OR_B
        LD_HL_A
        INC_HL

        LD_A_MEM w3dcgb_mask_span_max_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_C_A
        LD_A_B
        SUB_C
        DEC_A
        LD_B_A
        OR_A
        JP_Z w3dmss_last

// Interior mask bytes are wholly covered; preserve only the partial boundary bytes.
w3dmss_middle:
        LD_A_IMM 0xFF
        LDI_HL_A
        DEC_B
        JP_NZ w3dmss_middle

w3dmss_last:
        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_max_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_end_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        LD_A_HL
        OR_B
        LD_HL_A
        RET

w3dmss_single:
        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_min_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_start_mask
        ADD_HL_DE
        LD_A_HL
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_max_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_end_mask
        ADD_HL_DE
        LD_A_HL
        AND_C
        LD_B_A
        POP_HL
        LD_A_HL
        OR_B
        LD_HL_A
        RET
    }
}

// Save SVBK, select bank 2, and clear an inclusive span in both bitplanes of
// the D300 column-major stage. Adjacent tile columns are 0xC0 bytes apart.
// Restore SVBK on return. Shared inputs must be ordered within 128x96; no
// clipping, dirty marking, mask update or interrupt protection occurs here.
void w3dcgb_stage_clear_span_asm()
{
    __asm {
w3dscs_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112

        /* HL = stage byte for min-x tile and requested scanline. */
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_C
        ADD_B
        LD_C_A

        LD_A_MEM w3dcgb_mask_span_y
        LD_B_A
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_C
        LD_C_A

        LD_A_C
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A
        LD_A_C
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        AND_IMM 7
        ADD_A
        ADD_C
        LD_L_A

        LD_A_MEM w3dcgb_mask_span_max_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        CP_B
        JP_Z w3dscs_single

        /* Clear the first partial tile in both bitplanes. */
        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_min_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_start_mask
        ADD_HL_DE
        LD_A_HL
        CPL
        LD_B_A
        POP_HL
        LD_A_HL
        AND_B
        LD_HL_A
        INC_L
        LD_A_HL
        AND_B
        LD_HL_A
        DEC_L

        LD_DE_IMM 0x00C0
        ADD_HL_DE
        LD_A_MEM w3dcgb_mask_span_max_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_C_A
        LD_A_B
        SUB_C
        DEC_A
        LD_B_A
        OR_A
        JP_Z w3dscs_last

w3dscs_middle:
        XOR_A
        LD_HL_A
        INC_L
        LD_HL_A
        DEC_L
        ADD_HL_DE
        DEC_B
        JP_NZ w3dscs_middle

w3dscs_last:
        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_max_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_end_mask
        ADD_HL_DE
        LD_A_HL
        CPL
        LD_B_A
        POP_HL
        LD_A_HL
        AND_B
        LD_HL_A
        INC_L
        LD_A_HL
        AND_B
        LD_HL_A
        JP w3dscs_ret

w3dscs_single:
        PUSH_HL
        LD_A_MEM w3dcgb_mask_span_min_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_start_mask
        ADD_HL_DE
        LD_A_HL
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_max_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_span_end_mask
        ADD_HL_DE
        LD_A_HL
        AND_C
        CPL
        LD_B_A
        POP_HL
        LD_A_HL
        AND_B
        LD_HL_A
        INC_L
        LD_A_HL
        AND_B
        LD_HL_A

w3dscs_ret:
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_order 98
// Sort the shared vertices by Y, walk the long edge and each short edge with
// 16-bit error accumulators, and emit inclusive scanline spans with a one-pixel
// X guard. Flat triangles emit one span. Select the bitmap-mask or full-screen
// span writer through full_mode; all scratch is shared. Keep vertices within
// the selected viewport. Padding stops at X=127 or X=159 respectively, but
// general triangle clipping is not performed.
void w3dcgb_mark_triangle_asm()
{
    __asm {
w3dmt_enter:
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        CP_B
        JP_NC w3dmt_sort12
        CALL w3dmt_swap01

w3dmt_sort12:
        LD_A_MEM w3dcgb_tri_y1
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        CP_B
        JP_NC w3dmt_sort01_again
        CALL w3dmt_swap12

w3dmt_sort01_again:
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        CP_B
        JP_NC w3dmt_sorted
        CALL w3dmt_swap01

w3dmt_sorted:
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        CP_B
        JP_NZ w3dmt_setup_long

        LD_A_MEM w3dcgb_tri_x0
        LD_MEM_A w3dcgb_tri_long_x
        LD_MEM_A w3dcgb_tri_short_x
        LD_A_MEM w3dcgb_tri_x1
        LD_B_A
        LD_A_MEM w3dcgb_tri_long_x
        CP_B
        JP_C w3dmt_flat_min1_done
        LD_A_B
        LD_MEM_A w3dcgb_tri_long_x
w3dmt_flat_min1_done:
        LD_A_MEM w3dcgb_tri_x1
        LD_B_A
        LD_A_MEM w3dcgb_tri_short_x
        CP_B
        JP_NC w3dmt_flat_max1_done
        LD_A_B
        LD_MEM_A w3dcgb_tri_short_x
w3dmt_flat_max1_done:
        LD_A_MEM w3dcgb_tri_x2
        LD_B_A
        LD_A_MEM w3dcgb_tri_long_x
        CP_B
        JP_C w3dmt_flat_min2_done
        LD_A_B
        LD_MEM_A w3dcgb_tri_long_x
w3dmt_flat_min2_done:
        LD_A_MEM w3dcgb_tri_x2
        LD_B_A
        LD_A_MEM w3dcgb_tri_short_x
        CP_B
        JP_NC w3dmt_flat_max2_done
        LD_A_B
        LD_MEM_A w3dcgb_tri_short_x
w3dmt_flat_max2_done:
        LD_A_MEM w3dcgb_tri_y0
        LD_MEM_A w3dcgb_tri_y
        CALL w3dmt_emit_span
        RET

// Keep the top-to-bottom edge across both halves; replace only the short edge
// when reaching the middle vertex.
w3dmt_setup_long:
        LD_A_MEM w3dcgb_tri_x0
        LD_MEM_A w3dcgb_tri_long_x
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        SUB_B
        LD_MEM_A w3dcgb_tri_long_dy

        LD_A_MEM w3dcgb_tri_x0
        LD_B_A
        LD_A_MEM w3dcgb_tri_x2
        CP_B
        JP_C w3dmt_long_left
        SUB_B
        LD_MEM_A w3dcgb_tri_long_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_tri_long_step
        JP w3dmt_long_ready
w3dmt_long_left:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_tri_long_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_tri_long_step
w3dmt_long_ready:
        XOR_A
        LD_MEM_A w3dcgb_tri_long_err_lo
        LD_MEM_A w3dcgb_tri_long_err_hi

        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        CP_B
        JP_Z w3dmt_setup_lower

        LD_A_MEM w3dcgb_tri_x0
        LD_MEM_A w3dcgb_tri_short_x
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        SUB_B
        LD_MEM_A w3dcgb_tri_short_dy
        LD_A_MEM w3dcgb_tri_x0
        LD_B_A
        LD_A_MEM w3dcgb_tri_x1
        CP_B
        JP_C w3dmt_upper_left
        SUB_B
        LD_MEM_A w3dcgb_tri_short_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_tri_short_step
        JP w3dmt_upper_ready
w3dmt_upper_left:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_tri_short_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_tri_short_step
w3dmt_upper_ready:
        XOR_A
        LD_MEM_A w3dcgb_tri_short_err_lo
        LD_MEM_A w3dcgb_tri_short_err_hi
        LD_A_MEM w3dcgb_tri_y0
        LD_MEM_A w3dcgb_tri_y

w3dmt_upper_loop:
        LD_A_MEM w3dcgb_tri_y
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        CP_B
        JP_Z w3dmt_setup_lower
        CALL w3dmt_emit_span
        CALL w3dmt_step_long
        CALL w3dmt_step_short
        LD_A_MEM w3dcgb_tri_y
        INC_A
        LD_MEM_A w3dcgb_tri_y
        JP w3dmt_upper_loop

w3dmt_setup_lower:
        LD_A_MEM w3dcgb_tri_x1
        LD_MEM_A w3dcgb_tri_short_x
        LD_A_MEM w3dcgb_tri_y1
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        SUB_B
        LD_MEM_A w3dcgb_tri_short_dy
        LD_A_MEM w3dcgb_tri_x1
        LD_B_A
        LD_A_MEM w3dcgb_tri_x2
        CP_B
        JP_C w3dmt_lower_left
        SUB_B
        LD_MEM_A w3dcgb_tri_short_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_tri_short_step
        JP w3dmt_lower_ready
w3dmt_lower_left:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_tri_short_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_tri_short_step
w3dmt_lower_ready:
        XOR_A
        LD_MEM_A w3dcgb_tri_short_err_lo
        LD_MEM_A w3dcgb_tri_short_err_hi
        LD_A_MEM w3dcgb_tri_y1
        LD_MEM_A w3dcgb_tri_y

w3dmt_lower_loop:
        CALL w3dmt_emit_span
        LD_A_MEM w3dcgb_tri_y
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        CP_B
        JP_Z w3dmt_done
        CALL w3dmt_step_long
        CALL w3dmt_step_short
        LD_A_MEM w3dcgb_tri_y
        INC_A
        LD_MEM_A w3dcgb_tri_y
        JP w3dmt_lower_loop

w3dmt_done:
        RET

// Accumulate horizontal distance in a two-byte error so wide shallow edges
// do not overflow an eight-bit accumulator before the next scanline.
w3dmt_step_long:
        LD_A_MEM w3dcgb_tri_long_dx
        LD_C_A
        LD_A_MEM w3dcgb_tri_long_err_lo
        ADD_C
        LD_MEM_A w3dcgb_tri_long_err_lo
        JP_NC w3dmt_step_long_test
        LD_A_MEM w3dcgb_tri_long_err_hi
        INC_A
        LD_MEM_A w3dcgb_tri_long_err_hi
w3dmt_step_long_test:
        LD_A_MEM w3dcgb_tri_long_err_hi
        OR_A
        JP_NZ w3dmt_step_long_once
        LD_A_MEM w3dcgb_tri_long_dy
        LD_C_A
        LD_A_MEM w3dcgb_tri_long_err_lo
        CP_C
        RET_C
w3dmt_step_long_once:
        LD_A_MEM w3dcgb_tri_long_dy
        LD_C_A
        LD_A_MEM w3dcgb_tri_long_err_lo
        SUB_C
        LD_MEM_A w3dcgb_tri_long_err_lo
        JP_NC w3dmt_step_long_x
        LD_A_MEM w3dcgb_tri_long_err_hi
        DEC_A
        LD_MEM_A w3dcgb_tri_long_err_hi
w3dmt_step_long_x:
        LD_A_MEM w3dcgb_tri_long_x
        LD_B_A
        LD_A_MEM w3dcgb_tri_long_step
        ADD_B
        LD_MEM_A w3dcgb_tri_long_x
        JP w3dmt_step_long_test

w3dmt_step_short:
        LD_A_MEM w3dcgb_tri_short_dx
        LD_C_A
        LD_A_MEM w3dcgb_tri_short_err_lo
        ADD_C
        LD_MEM_A w3dcgb_tri_short_err_lo
        JP_NC w3dmt_step_short_test
        LD_A_MEM w3dcgb_tri_short_err_hi
        INC_A
        LD_MEM_A w3dcgb_tri_short_err_hi
w3dmt_step_short_test:
        LD_A_MEM w3dcgb_tri_short_err_hi
        OR_A
        JP_NZ w3dmt_step_short_once
        LD_A_MEM w3dcgb_tri_short_dy
        LD_C_A
        LD_A_MEM w3dcgb_tri_short_err_lo
        CP_C
        RET_C
w3dmt_step_short_once:
        LD_A_MEM w3dcgb_tri_short_dy
        LD_C_A
        LD_A_MEM w3dcgb_tri_short_err_lo
        SUB_C
        LD_MEM_A w3dcgb_tri_short_err_lo
        JP_NC w3dmt_step_short_x
        LD_A_MEM w3dcgb_tri_short_err_hi
        DEC_A
        LD_MEM_A w3dcgb_tri_short_err_hi
w3dmt_step_short_x:
        LD_A_MEM w3dcgb_tri_short_x
        LD_B_A
        LD_A_MEM w3dcgb_tri_short_step
        ADD_B
        LD_MEM_A w3dcgb_tri_short_x
        JP w3dmt_step_short_test

// Order the two intersections and add the active ASM path's one-pixel guard.
// Each mode limits right padding before dispatching to its own span writer.
w3dmt_emit_span:
        LD_A_MEM w3dcgb_tri_long_x
        LD_B_A
        LD_A_MEM w3dcgb_tri_short_x
        CP_B
        JP_C w3dmt_span_short_left
        LD_A_B
        LD_MEM_A w3dcgb_mask_span_min_x
        LD_A_MEM w3dcgb_tri_short_x
        LD_MEM_A w3dcgb_mask_span_max_x
        JP w3dmt_span_pad
w3dmt_span_short_left:
        LD_MEM_A w3dcgb_mask_span_min_x
        LD_A_B
        LD_MEM_A w3dcgb_mask_span_max_x
w3dmt_span_pad:
        LD_A_MEM w3dcgb_mask_span_min_x
        OR_A
        JP_Z w3dmt_span_min_done
        DEC_A
        LD_MEM_A w3dcgb_mask_span_min_x
w3dmt_span_min_done:
        LD_A_MEM w3dcgb_tri_y
        LD_MEM_A w3dcgb_mask_span_y
        LD_A_MEM w3dcgb_full_mode
        OR_A
        JP_NZ w3dmt_emit_full_span
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_IMM 127
        JP_NC w3dmt_span_max_done
        INC_A
        LD_MEM_A w3dcgb_mask_span_max_x
w3dmt_span_max_done:
        CALL w3dcgb_mask_set_span_asm
        RET

w3dmt_emit_full_span:
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_IMM 159
        JP_NC w3dmt_full_span_max_done
        INC_A
        LD_MEM_A w3dcgb_mask_span_max_x
w3dmt_full_span_max_done:
        CALL w3dcgb_full_span_insert_asm
        RET

w3dmt_swap01:
        LD_A_MEM w3dcgb_tri_x0
        LD_B_A
        LD_A_MEM w3dcgb_tri_x1
        LD_MEM_A w3dcgb_tri_x0
        LD_A_B
        LD_MEM_A w3dcgb_tri_x1
        LD_A_MEM w3dcgb_tri_y0
        LD_B_A
        LD_A_MEM w3dcgb_tri_y1
        LD_MEM_A w3dcgb_tri_y0
        LD_A_B
        LD_MEM_A w3dcgb_tri_y1
        RET

w3dmt_swap12:
        LD_A_MEM w3dcgb_tri_x1
        LD_B_A
        LD_A_MEM w3dcgb_tri_x2
        LD_MEM_A w3dcgb_tri_x1
        LD_A_B
        LD_MEM_A w3dcgb_tri_x2
        LD_A_MEM w3dcgb_tri_y1
        LD_B_A
        LD_A_MEM w3dcgb_tri_y2
        LD_MEM_A w3dcgb_tri_y1
        LD_A_B
        LD_MEM_A w3dcgb_tri_y2
        RET
    }
}

#pragma fixed_order 99
// Wait until STAT mode bit 1 is clear, then write the shared address/value
// once. The caller selects VBK and prevents interrupts from disrupting the
// access window; neither VBK nor interrupt state is managed here.
void w3dcgb_put_bg_tile_safe_asm()
{
    __asm {
w3dbg_enter:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ w3dbg_enter
        LD_A_MEM w3dcgb_bgq_addr_hi
        LD_H_A
        LD_A_MEM w3dcgb_bgq_addr_lo
        LD_L_A
        LD_A_MEM w3dcgb_bgq_value
        LD_HL_A
        RET
    }
}

#pragma fixed_order 100
// Save SVBK, select WRAM bank 2, clear all 192 two-bitplane tiles (3072
// bytes) starting at stage, and restore SVBK. No dirty flags or history are
// updated. Interrupt handlers must not change the active stage mapping.
void w3dcgb_clear_stage_asm()
{
    __asm {
w3dcs_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112
        LD_HL_IMM w3dcgb_stage
        XOR_A
        LD_B_IMM 192

w3dcs_inner:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        DEC_B
        JR_NZ w3dcs_inner
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_order 101
// Save/select/restore WRAM bank 2 while clearing dma_len consecutive tiles
// starting at D300 + dma_tile*16. Zero length is a no-op. The caller must
// validate the entire range; no dirty flags or bounds are checked.
void w3dcgb_clear_sparse_stage_asm()
{
    __asm {
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112

        LD_A_MEM w3dcgb_dma_len
        OR_A
        JP_Z w3dcss_ret
        LD_B_A

        LD_A_MEM w3dcgb_dma_tile
        LD_C_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_L_A

        LD_A_C
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A
        XOR_A

w3dcss_tile:
        LD_C_IMM 16
w3dcss_byte:
        LDI_HL_A
        DEC_C
        JR_NZ w3dcss_byte
        DEC_B
        JR_NZ w3dcss_tile
w3dcss_ret:
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_order 102
// Wait for current-or-next VBlank when the LCD is on, then clear the blank
// HUD tile at 0x88A0 in VRAM bank zero. Leave VBK zero; this does not wait
// for a fresh VBlank when already inside one.
void w3dcgb_restore_hud_blank_tile_asm()
{
    __asm {
        LDH_A_MEM 64
        AND_IMM 0x80
        JR_Z w3drhb_write
w3drhb_wait_vblank:
        LDH_A_MEM 68
        CP_IMM 0x90
        JR_C w3drhb_wait_vblank
w3drhb_write:
        XOR_A
        LDH_MEM_A 79
        LD_HL_IMM 0x88A0
        LD_C_IMM 16
w3drhb_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3drhb_loop
        RET
    }
}

#pragma fixed_order 104
// Save SVBK, select bank 2, clear 384 bytes for the leftmost two tile
// columns and restore SVBK. No dirty flags are updated.
void w3dcgb_erase_left_guard16_asm()
{
    __asm {
w3deg_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112
        LD_HL_IMM w3dcgb_stage
        XOR_A
        LD_B_IMM 0
        LD_C_IMM 0x80

w3deg_loop0:
        LDI_HL_A
        DEC_C
        JR_NZ w3deg_loop0

        LD_C_IMM 0x80
w3deg_loop1:
        LDI_HL_A
        DEC_C
        JR_NZ w3deg_loop1

        LD_C_IMM 0x80
w3deg_loop2:
        LDI_HL_A
        DEC_C
        JR_NZ w3deg_loop2
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_order 106
// Save SVBK, select bank 2, clear 576 bytes for the leftmost three tile
// columns in unrolled groups and restore SVBK. No dirty flags are updated.
void w3dcgb_erase_left_guard24_asm()
{
    __asm {
w3deh_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112
        LD_HL_IMM w3dcgb_stage
        XOR_A
        LD_B_IMM 36

w3deh_loop:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        DEC_B
        JR_NZ w3deh_loop
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_bank 1
#pragma fixed_order 110
// Save/select/restore WRAM bank 2 while replacing one validated 128x96
// pixel in both bitplanes. Active mask bits suppress writes; color zero
// erases. Sparse mode sets the tile flag, but this helper does not expand
// the sparse min/max range; the public point wrapper handles that range.
void w3dcgb_plot_stage_asm()
{
    __asm {
w3dps_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112

        LD_A_MEM w3dcgb_occlusion_active
        OR_A
        JP_Z w3dps_bounds

        LD_A_MEM w3dcgb_plot_x
        CP_IMM 0x80
        JP_NC w3dps_ret
        LD_A_MEM w3dcgb_plot_y
        CP_IMM 0x60
        JP_NC w3dps_ret

        LD_A_MEM w3dcgb_plot_y
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_E_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_D_A
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE

        LD_A_MEM w3dcgb_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE

        PUSH_HL
        LD_A_MEM w3dcgb_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        LD_A_HL
        AND_B
        JP_NZ w3dps_ret
        JP w3dps_inside

w3dps_bounds:
        LD_A_MEM w3dcgb_plot_x
        CP_IMM 0x80
        JP_NC w3dps_ret

        LD_A_MEM w3dcgb_plot_y
        CP_IMM 0x60
        JP_NC w3dps_ret

w3dps_inside:
        LD_A_MEM w3dcgb_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_C
        ADD_B
        LD_C_A

        LD_A_MEM w3dcgb_plot_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_C
        ADD_B
        LD_MEM_A w3dcgb_dirty_tile_tmp
        LD_B_A

        LD_A_MEM w3dcgb_full_frame_transfer
        OR_A
        JP_NZ w3dps_dirty_ready
        LD_C_A
        LD_A_B
        LD_C_A
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_dirty_flags
        ADD_HL_DE
        LD_A_HL
        OR_A
        JP_NZ w3dps_dirty_ready
        LD_A_IMM 1
        LD_HL_A
w3dps_dirty_ready:
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A

        LD_A_B
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_B_A

        LD_A_MEM w3dcgb_plot_y
        AND_IMM 7
        ADD_A
        ADD_B
        LD_L_A

        LD_A_MEM w3dcgb_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

        LD_A_B
        CPL
        LD_C_A

        LD_A_HL
        AND_C
        LD_HL_A

        INC_L
        LD_A_HL
        AND_C
        LD_HL_A

        LD_A_MEM w3dcgb_line_color
        AND_IMM 1
        JP_Z w3dps_skip_low

        DEC_L
        LD_A_HL
        OR_B
        LD_HL_A
        INC_L

w3dps_skip_low:
        LD_A_MEM w3dcgb_line_color
        AND_IMM 2
        JP_Z w3dps_ret

        LD_A_HL
        OR_B
        LD_HL_A

w3dps_ret:
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_bank -1
#pragma bank 4
#pragma fixed_order 120
// Draw inclusive byte endpoints through the normal 128x96 stage in WRAM
// bank 2, restoring SVBK afterward. The active fast path has no coordinate
// checks or dirty-range updates; callers must provide in-bounds geometry
// and arrange sparse uploads. It ORs color 1/2 into their respective planes;
// other values, including zero, set both planes. Overlapping colors combine.
// Masked and unmasked shallow/steep loops share address and bit-mask stepping.
void w3dcgb_line_stage_asm()
{
    __asm {
w3dls_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112
w3dls_masked_generic:
        LD_A_MEM w3dcgb_line_x0
        LD_B_A
        LD_A_MEM w3dcgb_line_x1
        CP_B
        JP_C w3dls_x_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sx
        JP w3dls_y_start

w3dls_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sx

w3dls_y_start:
        LD_A_MEM w3dcgb_line_y0
        LD_B_A
        LD_A_MEM w3dcgb_line_y1
        CP_B
        JP_C w3dls_y_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sy
        JP w3dls_branch

w3dls_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sy

w3dls_branch:
        JP w3dls_fast_start

// Maintain HL at the stage byte, DE at its row-major mask byte and B as
// the current pixel bit; stepping avoids recomputing addresses per pixel.
w3dls_fast_start:
        LD_A_MEM w3dcgb_line_x0
        LD_MEM_A w3dcgb_line_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_C
        ADD_B
        LD_C_A

        LD_A_MEM w3dcgb_line_y0
        LD_MEM_A w3dcgb_line_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_C
        ADD_B
        LD_B_A

        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A

        LD_A_B
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_MEM w3dcgb_line_y0
        AND_IMM 7
        ADD_A
        ADD_C
        LD_L_A

        PUSH_HL
        LD_A_MEM w3dcgb_line_y0
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_E_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_D_A
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE
        LD_A_MEM w3dcgb_line_x0
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_H
        LD_E_L
        POP_HL

        PUSH_DE
        LD_A_MEM w3dcgb_line_x0
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        POP_DE

        LD_A_MEM w3dcgb_occlusion_active
        OR_A
        JP_Z w3dls_fast_unmasked_branch

        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_dy
        CP_C
        JP_C w3dls_fast_shallow
        JP_Z w3dls_fast_shallow
        JP w3dls_fast_steep

w3dls_fast_shallow:
        LD_A_MEM w3dcgb_line_dx
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_shallow_loop:
        CALL w3dls_fast_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_shallow_no_y
        CALL w3dls_fast_step_y
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_shallow_no_y:
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_x
        JP w3dls_fast_shallow_loop

w3dls_fast_steep:
        LD_A_MEM w3dcgb_line_dy
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_steep_loop:
        CALL w3dls_fast_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_steep_no_x
        CALL w3dls_fast_step_x
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_steep_no_x:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_y
        JP w3dls_fast_steep_loop

// Skip mask reads when occlusion is disabled; color 3 also bypasses
// per-pixel color selection in its dedicated loop.
w3dls_fast_unmasked_branch:
        LD_A_MEM w3dcgb_line_color
        CP_IMM 3
        JP_Z w3dls_fast_unmasked_both_branch

        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_dy
        CP_C
        JP_C w3dls_fast_unmasked_shallow
        JP_Z w3dls_fast_unmasked_shallow
        JP w3dls_fast_unmasked_steep

w3dls_fast_unmasked_shallow:
        LD_A_MEM w3dcgb_line_dx
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_shallow_loop:
        CALL w3dls_plot_hl
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_unmasked_shallow_no_y
        CALL w3dls_fast_step_y
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_shallow_no_y:
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_x
        JP w3dls_fast_unmasked_shallow_loop

w3dls_fast_unmasked_steep:
        LD_A_MEM w3dcgb_line_dy
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_steep_loop:
        CALL w3dls_plot_hl
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_unmasked_steep_no_x
        CALL w3dls_fast_step_x
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_steep_no_x:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_y
        JP w3dls_fast_unmasked_steep_loop

w3dls_fast_unmasked_both_branch:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_dy
        CP_C
        JP_C w3dls_fast_unmasked_both_shallow
        JP_Z w3dls_fast_unmasked_both_shallow
        JP w3dls_fast_unmasked_both_steep

w3dls_fast_unmasked_both_shallow:
        LD_A_MEM w3dcgb_line_dx
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_both_shallow_loop:
        CALL w3dls_plot_both
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_unmasked_both_shallow_no_y
        CALL w3dls_fast_step_y
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_both_shallow_no_y:
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_x
        JP w3dls_fast_unmasked_both_shallow_loop

w3dls_fast_unmasked_both_steep:
        LD_A_MEM w3dcgb_line_dy
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_both_steep_loop:
        CALL w3dls_plot_both
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        JP_Z w3dls_done
        DEC_A
        LD_MEM_A w3dcgb_line_remaining

        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dls_fast_unmasked_both_steep_no_x
        CALL w3dls_fast_step_x
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err

w3dls_fast_unmasked_both_steep_no_x:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dls_fast_step_y
        JP w3dls_fast_unmasked_both_steep_loop

w3dls_fast_step_y:
        LD_A_MEM w3dcgb_line_sy
        CP_IMM 1
        JP_Z w3dls_fast_step_y_down
        DEC_HL
        DEC_HL
        LD_A_E
        SUB_IMM 0x10
        LD_E_A
        JP_NC w3dls_fast_step_y_done
        DEC_D
        RET
w3dls_fast_step_y_done:
        RET
w3dls_fast_step_y_down:
        INC_HL
        INC_HL
        LD_A_E
        ADD_A_IMM 0x10
        LD_E_A
        JP_NC w3dls_fast_step_y_done
        INC_D
        RET

w3dls_fast_step_x:
        LD_A_MEM w3dcgb_line_sx
        CP_IMM 1
        JP_Z w3dls_fast_step_x_right

        LD_A_B
        ADD_A
        LD_B_A
        OR_A
        JP_NZ w3dls_fast_step_x_done
        LD_B_IMM 1
        PUSH_DE
        LD_DE_IMM 0xFF40
        ADD_HL_DE
        POP_DE
        DEC_DE
        RET

w3dls_fast_step_x_right:
        LD_A_B
        OR_A
        RRA
        LD_B_A
        OR_A
        JP_NZ w3dls_fast_step_x_done
        LD_B_IMM 0x80
        PUSH_DE
        LD_DE_IMM 0x00C0
        ADD_HL_DE
        POP_DE
        INC_DE
w3dls_fast_step_x_done:
        RET

w3dls_fast_plot:
        LD_A_MEM w3dcgb_occlusion_active
        OR_A
        JP_Z w3dls_plot_hl
        LD_A_DE
        AND_B
        JP_Z w3dls_plot_hl
        RET

// These retained generic/axis loops are not selected by the active entry,
// which jumps directly to w3dls_fast_start. They are not the active bounds checks.
w3dls_shallow:
        LD_A_MEM w3dcgb_line_x0
        LD_MEM_A w3dcgb_line_x
        LD_A_MEM w3dcgb_line_y0
        LD_MEM_A w3dcgb_line_y
        LD_A_MEM w3dcgb_line_dx
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_shallow_loop:
        LD_A_MEM w3dcgb_line_x
        LD_MEM_A w3dcgb_plot_x
        LD_A_MEM w3dcgb_line_y
        LD_MEM_A w3dcgb_plot_y
        CALL w3dcgb_plot_stage_asm

        LD_A_MEM w3dcgb_line_x
        LD_B_A
        LD_A_MEM w3dcgb_line_x1
        CP_B
        JP_Z w3dls_done

        LD_A_MEM w3dcgb_line_dy
        LD_B_A
        LD_A_MEM w3dcgb_line_err
        CP_B
        JP_NC w3dls_shallow_skip_bridge

        LD_A_MEM w3dcgb_line_y
        LD_B_A
        LD_A_MEM w3dcgb_line_sy
        ADD_B
        LD_MEM_A w3dcgb_line_y

        LD_A_MEM w3dcgb_line_x
        LD_MEM_A w3dcgb_plot_x
        LD_A_MEM w3dcgb_line_y
        LD_MEM_A w3dcgb_plot_y
        CALL w3dcgb_plot_stage_asm

        LD_A_MEM w3dcgb_line_dx
        LD_B_A
        LD_A_MEM w3dcgb_line_err
        ADD_B
        LD_MEM_A w3dcgb_line_err

w3dls_shallow_skip_bridge:
        LD_A_MEM w3dcgb_line_err
        LD_B_A
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_err

        LD_A_MEM w3dcgb_line_x
        LD_B_A
        LD_A_MEM w3dcgb_line_sx
        ADD_B
        LD_MEM_A w3dcgb_line_x
        JP w3dls_shallow_loop

w3dls_steep:
        LD_A_MEM w3dcgb_line_x0
        LD_MEM_A w3dcgb_line_x
        LD_A_MEM w3dcgb_line_y0
        LD_MEM_A w3dcgb_line_y
        LD_A_MEM w3dcgb_line_dy
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err

w3dls_steep_loop:
        LD_A_MEM w3dcgb_line_x
        LD_MEM_A w3dcgb_plot_x
        LD_A_MEM w3dcgb_line_y
        LD_MEM_A w3dcgb_plot_y
        CALL w3dcgb_plot_stage_asm

        LD_A_MEM w3dcgb_line_y
        LD_B_A
        LD_A_MEM w3dcgb_line_y1
        CP_B
        JP_Z w3dls_done

        LD_A_MEM w3dcgb_line_dx
        LD_B_A
        LD_A_MEM w3dcgb_line_err
        CP_B
        JP_NC w3dls_steep_skip_bridge

        LD_A_MEM w3dcgb_line_x
        LD_B_A
        LD_A_MEM w3dcgb_line_sx
        ADD_B
        LD_MEM_A w3dcgb_line_x

        LD_A_MEM w3dcgb_line_x
        LD_MEM_A w3dcgb_plot_x
        LD_A_MEM w3dcgb_line_y
        LD_MEM_A w3dcgb_plot_y
        CALL w3dcgb_plot_stage_asm

        LD_A_MEM w3dcgb_line_dy
        LD_B_A
        LD_A_MEM w3dcgb_line_err
        ADD_B
        LD_MEM_A w3dcgb_line_err

w3dls_steep_skip_bridge:
        LD_A_MEM w3dcgb_line_err
        LD_B_A
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_err

        LD_A_MEM w3dcgb_line_y
        LD_B_A
        LD_A_MEM w3dcgb_line_sy
        ADD_B
        LD_MEM_A w3dcgb_line_y
        JP w3dls_steep_loop

w3dls_horizontal:
        LD_A_MEM w3dcgb_line_x0
        LD_B_A
        LD_A_MEM w3dcgb_line_x1
        CP_B
        JP_C w3dls_horizontal_reverse
        LD_A_B
        LD_MEM_A w3dcgb_line_x
        JP w3dls_horizontal_addr

w3dls_horizontal_reverse:
        LD_MEM_A w3dcgb_line_x
        LD_A_B
        LD_MEM_A w3dcgb_line_x1

w3dls_horizontal_addr:
        LD_A_MEM w3dcgb_line_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_C
        ADD_B
        LD_C_A

        LD_A_MEM w3dcgb_line_y0
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_C
        ADD_B
        LD_B_A

        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A

        LD_A_B
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_B_A

        LD_A_MEM w3dcgb_line_y0
        AND_IMM 7
        ADD_A
        ADD_B
        LD_L_A

        LD_A_MEM w3dcgb_line_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

w3dls_horizontal_loop:
        CALL w3dls_plot_hl
        LD_A_MEM w3dcgb_line_x
        LD_C_A
        LD_A_MEM w3dcgb_line_x1
        CP_C
        JP_Z w3dls_done
        LD_A_C
        ADD_A_IMM 1
        LD_MEM_A w3dcgb_line_x
        LD_A_B
        CP_IMM 1
        JP_Z w3dls_horizontal_next_tile
        OR_A
        RRA
        LD_B_A
        JP w3dls_horizontal_loop

w3dls_horizontal_next_tile:
        LD_B_IMM 0x80
        LD_DE_IMM 0x00C0
        ADD_HL_DE
        JP w3dls_horizontal_loop

w3dls_vertical:
        LD_A_MEM w3dcgb_line_y0
        LD_B_A
        LD_A_MEM w3dcgb_line_y1
        CP_B
        JP_C w3dls_vertical_reverse
        LD_A_B
        LD_MEM_A w3dcgb_line_y
        JP w3dls_vertical_addr

w3dls_vertical_reverse:
        LD_MEM_A w3dcgb_line_y
        LD_A_B
        LD_MEM_A w3dcgb_line_y1

w3dls_vertical_addr:
        LD_A_MEM w3dcgb_line_x0
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_B
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_C
        ADD_B
        LD_C_A

        LD_A_MEM w3dcgb_line_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_C
        ADD_B
        LD_B_A

        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD3
        LD_H_A

        LD_A_B
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_B_A

        LD_A_MEM w3dcgb_line_y
        AND_IMM 7
        ADD_A
        ADD_B
        LD_L_A

        LD_A_MEM w3dcgb_line_x0
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

w3dls_vertical_loop:
        CALL w3dls_plot_hl
        LD_A_MEM w3dcgb_line_y
        LD_C_A
        LD_A_MEM w3dcgb_line_y1
        CP_C
        JP_Z w3dls_done
        LD_A_C
        ADD_A_IMM 1
        LD_MEM_A w3dcgb_line_y
        INC_HL
        INC_HL
        JP w3dls_vertical_loop

w3dls_done:
        POP_AF
        LDH_MEM_A 112
        RET

// The normal line path accumulates coverage with OR. It does not clear the
// other color plane, unlike the point and full-screen fast plotters.
w3dls_plot_hl:
        LD_A_MEM w3dcgb_line_color
        CP_IMM 1
        JP_Z w3dls_plot_low_only
        CP_IMM 2
        JP_Z w3dls_plot_high_only

        LD_A_HL
        OR_B
        LD_HL_A
        INC_L
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_L
        RET

w3dls_plot_low_only:
        LD_A_HL
        OR_B
        LD_HL_A
        RET

w3dls_plot_high_only:
        INC_L
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_L
        RET

w3dls_plot_both:
        LD_A_HL
        OR_B
        LD_HL_A
        INC_L
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_L
        RET
    }
}

#pragma fixed_bank 2
#pragma bank 2
#pragma fixed_order 129
// During LCD-off setup, set the 16x12 viewport attributes to tile bank 1,
// palette zero. Leave other attributes unchanged and VBK zero on return.
void w3dcgb_select_blank_render_bank_asm()
{
    __asm {
w3dsb_enter:
        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_bg_map_9800
        LD_DE_IMM 16
        LD_B_IMM 12
        LD_A_IMM 8

w3dsb_row:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        ADD_HL_DE
        DEC_B
        JR_NZ w3dsb_row

        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 130
// Copy the complete 3072-byte bank-2 stage into the opposite VRAM tile bank.
// Wait for a fresh VBlank, then arm HBlank DMA in mode 2 for 128 blocks
// D300->8900 and 64 blocks DB00->9100. Wait for both transfers. Restore
// SVBK and leave VBK zero; pixels are not consumed. The LCD must remain
// enabled. Each DMA start executes EI, so entry IME is not preserved and
// handlers must retain the source/destination bank mapping during DMA.
void w3dcgb_transfer_stage_asm()
{
    __asm {
w3dtf_enter:
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112

        LD_A_MEM w3dcgb_display_tile_bank
        XOR_IMM 1
        LD_MEM_A w3dcgb_pending_tile_bank
        LDH_MEM_A 79

        /* Full frames use HBlank HDMA so LCD fetches are never starved. */
w3dtf_wait_start_vblank_end:
        LDH_A_MEM 68
        CP_IMM 144
        JP_NC w3dtf_wait_start_vblank_end

w3dtf_wait_start_vblank:
        LDH_A_MEM 68
        CP_IMM 144
        JP_C w3dtf_wait_start_vblank

        LD_A_IMM 0xD3
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_IMM 0x89
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        DI
w3dtf_arm_first:
        LDH_A_MEM 65
        AND_IMM 3
        CP_IMM 2
        JR_NZ w3dtf_arm_first
        LD_A_IMM 0xFF
        LDH_MEM_A 85
        EI

w3dtf_wait_first:
        LDH_A_MEM 85
        AND_IMM 0x80
        JP_Z w3dtf_wait_first

        LD_A_IMM 0xDB
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_IMM 0x91
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        DI
w3dtf_arm_second:
        LDH_A_MEM 65
        AND_IMM 3
        CP_IMM 2
        JR_NZ w3dtf_arm_second
        LD_A_IMM 0xBF
        LDH_MEM_A 85
        EI

w3dtf_wait_second:
        LDH_A_MEM 85
        AND_IMM 0x80
        JP_Z w3dtf_wait_second

        XOR_A
        LDH_MEM_A 79
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

#pragma fixed_order 131
// Expose pending tile data during VBlank. Atomic-map mode can use the
// current VBlank and flips LCDC map selection; normal mode waits for a fresh
// VBlank and rewrites all 16x12 attributes to the pending bank with palette
// zero. Record the displayed bank and leave VBK zero. The LCD must be enabled.
void w3dcgb_present_stage_asm()
{
    __asm {
        LD_A_MEM w3dcgb_atomic_maps
        OR_A
        JP_Z w3dpr_wait_vblank_end
w3dpr_atomic_wait:
        LDH_A_MEM 68
        CP_IMM 144
        JR_C w3dpr_atomic_wait
        LD_A_MEM w3dcgb_pending_tile_bank
        LD_MEM_A w3dcgb_display_tile_bank
        OR_A
        LDH_A_MEM 64
        JR_Z w3dpr_atomic_zero
        OR_IMM 8
        JR w3dpr_atomic_show
w3dpr_atomic_zero:
        AND_IMM 0xF7
w3dpr_atomic_show:
        LDH_MEM_A 64
        XOR_A
        LDH_MEM_A 79
        RET
w3dpr_wait_vblank_end:
        LDH_A_MEM 68
        CP_IMM 144
        JP_NC w3dpr_wait_vblank_end

w3dpr_wait_vblank_start:
        LDH_A_MEM 68
        CP_IMM 144
        JP_C w3dpr_wait_vblank_start

        LD_A_IMM 1
        LDH_MEM_A 79
        LD_A_MEM w3dcgb_pending_tile_bank
        OR_A
        JP_Z w3dpr_attr_value_ready
        LD_A_IMM 8

w3dpr_attr_value_ready:
        LD_HL_IMM w3dcgb_bg_map_9800
        LD_DE_IMM 16
        LD_B_IMM 12

w3dpr_attr_row:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        ADD_HL_DE
        DEC_B
        JR_NZ w3dpr_attr_row

        LD_A_MEM w3dcgb_pending_tile_bank
        LD_MEM_A w3dcgb_display_tile_bank
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 132
// Execute STOP after the caller has prepared KEY1, then return at the new
// CGB speed. This helper does not validate hardware or configure interrupts.
void w3dcgb_cgb_speed_switch_asm()
{
    __asm {
        STOP 0
        RET
    }
}

#pragma fixed_order 133
// Clear tile numbers and attributes for map 0x9800, using both VRAM banks.
// Require the LCD off and leave VBK zero; map 0x9C00 is untouched.
void w3dcgb_full_clear_map_asm()
{
    __asm {
        XOR_A
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_bg_map_9800
        LD_D_IMM 4
w3dfcm_page0:
        LD_C_IMM 0
w3dfcm_loop0:
        LDI_HL_A
        DEC_C
        JR_NZ w3dfcm_loop0
        DEC_D
        JR_NZ w3dfcm_page0

        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM w3dcgb_bg_map_9800
        XOR_A
        LD_D_IMM 4
w3dfcm_page1:
        LD_C_IMM 0
w3dfcm_loop1:
        LDI_HL_A
        DEC_C
        JR_NZ w3dfcm_loop1
        DEC_D
        JR_NZ w3dfcm_page1
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 134
// Mark both full-screen span slots empty on each of 144 rows by setting
// their minimum-X arrays to FF. Maximum-X arrays are retained but ignored
// while the corresponding minimum is FF. Other mask storage is untouched.
void w3dcgb_full_clear_spans_asm()
{
    __asm {
        LD_HL_IMM w3dcgb_occlusion_mask
        LD_A_IMM 0xFF
        LD_B_IMM 144
w3dfcs_min0:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfcs_min0

        LD_HL_IMM w3dcgb_occlusion_mask+288
        LD_B_IMM 144
w3dfcs_min1:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfcs_min1
        RET
    }
}

#pragma fixed_order 135
// Reset the 360-byte full-screen tile lookup to FF, empty both row-span
// lists, zero allocated-tile count/overflow/occlusion, and invalidate the
// last-tile cache. Pixel payload is not bulk-cleared here. This path does
// not select SVBK; retain a consistent WRAM mapping across full-screen calls.
void w3dcgb_full_begin_asm()
{
    __asm {
        LD_HL_IMM w3dcgb_stage
        LD_A_IMM 0xFF
        LD_B_IMM 22
w3dfb_lookup16:
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        DEC_B
        JR_NZ w3dfb_lookup16
        LD_B_IMM 8
w3dfb_lookup8:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfb_lookup8

        CALL w3dcgb_full_clear_spans_asm
        XOR_A
        LD_MEM_A w3dcgb_full_tile_count
        LD_MEM_A w3dcgb_full_overflow
        LD_MEM_A w3dcgb_occlusion_active
        LD_A_IMM 0xFF
        LD_MEM_A w3dcgb_full_cache_tx
        LD_MEM_A w3dcgb_full_cache_ty
        RET
    }
}

#pragma fixed_order 136
// Insert an ordered inclusive span for Y=0..143 and X=0..159. Empty minima
// are FF. Merge touching/overlapping coverage into slot zero; otherwise use
// slot one, expanding that second slot even for disjoint later spans. This
// conservative two-span representation can cover gaps. No clipping, WRAM
// bank selection, mask clearing or occlusion-enable update occurs here.
void w3dcgb_full_span_insert_asm()
{
    __asm {
        LD_A_MEM w3dcgb_mask_span_y
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dfsi_store0
        LD_B_A

        LD_HL_IMM w3dcgb_occlusion_mask+144
        ADD_HL_DE
        LD_A_HL
        LD_C_A
        CP_IMM 159
        JP_NC w3dfsi_right0_ready
        INC_A
w3dfsi_right0_ready:
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_min_x
        CP_C
        JP_C w3dfsi_left0_test
        JP_Z w3dfsi_left0_test
        JP w3dfsi_try1

w3dfsi_left0_test:
        LD_A_B
        OR_A
        JP_Z w3dfsi_merge0
        DEC_A
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_C
        JP_C w3dfsi_try1

w3dfsi_merge0:
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        CP_B
        JP_NC w3dfsi_merge0_max
        LD_HL_A
w3dfsi_merge0_max:
        LD_HL_IMM w3dcgb_occlusion_mask+144
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_B
        RET_C
        RET_Z
        LD_HL_A
        RET

w3dfsi_store0:
        LD_A_MEM w3dcgb_mask_span_min_x
        LD_HL_A
        LD_HL_IMM w3dcgb_occlusion_mask+144
        ADD_HL_DE
        LD_A_MEM w3dcgb_mask_span_max_x
        LD_HL_A
        RET

// A row has only two span slots. Later disjoint coverage expands slot one
// to its bounding interval, intentionally accepting conservative over-occlusion.
w3dfsi_try1:
        LD_HL_IMM w3dcgb_occlusion_mask+288
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dfsi_store1
        LD_B_A
        LD_HL_IMM w3dcgb_occlusion_mask+432
        ADD_HL_DE
        LD_A_HL
        LD_C_A
        CP_IMM 159
        JP_NC w3dfsi_right1_ready
        INC_A
w3dfsi_right1_ready:
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_min_x
        CP_C
        JP_C w3dfsi_left1_test
        JP_Z w3dfsi_left1_test
        JP w3dfsi_merge1

w3dfsi_left1_test:
        LD_A_B
        OR_A
        JP_Z w3dfsi_merge1
        DEC_A
        LD_C_A
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_C
        JP_C w3dfsi_merge1

w3dfsi_merge1:
        LD_HL_IMM w3dcgb_occlusion_mask+288
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_min_x
        CP_B
        JP_NC w3dfsi_merge1_max
        LD_HL_A
w3dfsi_merge1_max:
        LD_HL_IMM w3dcgb_occlusion_mask+432
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_mask_span_max_x
        CP_B
        RET_C
        RET_Z
        LD_HL_A
        RET

w3dfsi_store1:
        LD_A_MEM w3dcgb_mask_span_min_x
        LD_HL_A
        LD_HL_IMM w3dcgb_occlusion_mask+432
        ADD_HL_DE
        LD_A_MEM w3dcgb_mask_span_max_x
        LD_HL_A
        RET
    }
}

#pragma fixed_order 137
// Retained scalar 160x144 rasterizer: test pixel bounds/occlusion, cache
// a tile lookup, allocate and clear payload on first use, then replace both
// color planes. It uses the caller WRAM mapping. Keep allocation within
// 127 tiles: a failed allocation updates cache coordinates without replacing
// the cached slot. Public full-screen line drawing uses the fast variant.
void w3dcgb_full_line_asm()
{
    __asm {
w3dfl_enter:
        LD_A_MEM w3dcgb_line_x0
        LD_MEM_A w3dcgb_line_x
        LD_B_A
        LD_A_MEM w3dcgb_line_x1
        CP_B
        JP_C w3dfl_x_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sx
        JP w3dfl_y_setup
w3dfl_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sx

w3dfl_y_setup:
        LD_A_MEM w3dcgb_line_y0
        LD_MEM_A w3dcgb_line_y
        LD_B_A
        LD_A_MEM w3dcgb_line_y1
        CP_B
        JP_C w3dfl_y_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sy
        JP w3dfl_choose
w3dfl_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sy

w3dfl_choose:
        LD_A_MEM w3dcgb_line_dx
        LD_B_A
        LD_A_MEM w3dcgb_line_dy
        CP_B
        JP_C w3dfl_shallow
        JP_Z w3dfl_shallow
        JP w3dfl_steep

w3dfl_shallow:
        LD_A_MEM w3dcgb_line_dx
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err
w3dfl_shallow_loop:
        CALL w3dfl_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        RET_Z
        DEC_A
        LD_MEM_A w3dcgb_line_remaining
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dfl_shallow_no_y
        LD_A_MEM w3dcgb_line_sy
        LD_B_A
        LD_A_MEM w3dcgb_line_y
        ADD_B
        LD_MEM_A w3dcgb_line_y
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err
w3dfl_shallow_no_y:
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        LD_A_MEM w3dcgb_line_sx
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        ADD_B
        LD_MEM_A w3dcgb_line_x
        JP w3dfl_shallow_loop

w3dfl_steep:
        LD_A_MEM w3dcgb_line_dy
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err
w3dfl_steep_loop:
        CALL w3dfl_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        RET_Z
        DEC_A
        LD_MEM_A w3dcgb_line_remaining
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dfl_steep_no_x
        LD_A_MEM w3dcgb_line_sx
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        ADD_B
        LD_MEM_A w3dcgb_line_x
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err
w3dfl_steep_no_x:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        LD_A_MEM w3dcgb_line_sy
        LD_B_A
        LD_A_MEM w3dcgb_line_y
        ADD_B
        LD_MEM_A w3dcgb_line_y
        JP w3dfl_steep_loop

w3dfl_plot:
        LD_A_MEM w3dcgb_line_x
        CP_IMM 160
        RET_NC
        LD_A_MEM w3dcgb_line_y
        CP_IMM 144
        RET_NC

        LD_A_MEM w3dcgb_occlusion_active
        OR_A
        JP_Z w3dfl_tile
        LD_A_MEM w3dcgb_line_y
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dfl_mask1
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dfl_mask1
        LD_HL_IMM w3dcgb_occlusion_mask+144
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        RET_C
        RET_Z
w3dfl_mask1:
        LD_HL_IMM w3dcgb_occlusion_mask+288
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dfl_tile
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dfl_tile
        LD_HL_IMM w3dcgb_occlusion_mask+432
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        RET_C
        RET_Z

w3dfl_tile:
        LD_A_MEM w3dcgb_line_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_B_A
        LD_A_MEM w3dcgb_full_cache_tx
        CP_B
        JP_NZ w3dfl_cache_miss
        LD_A_MEM w3dcgb_line_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_C_A
        LD_A_MEM w3dcgb_full_cache_ty
        CP_C
        JP_Z w3dfl_have_slot

w3dfl_cache_miss:
        LD_A_MEM w3dcgb_line_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3dcgb_full_cache_tx
        LD_B_A
        LD_A_MEM w3dcgb_line_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3dcgb_full_cache_ty
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        LD_D_H
        LD_E_L
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_DE
        LD_A_B
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_DE_IMM w3dcgb_stage
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_NZ w3dfl_cache_slot

        LD_A_MEM w3dcgb_full_tile_count
        CP_IMM 127
        JP_C w3dfl_allocate
        LD_A_IMM 1
        LD_MEM_A w3dcgb_full_overflow
        RET

w3dfl_allocate:
        LD_HL_A
        LD_MEM_A w3dcgb_full_cache_slot

        LD_A_MEM w3dcgb_full_cache_ty
        AND_IMM 7
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_MEM w3dcgb_full_cache_tx
        ADD_B
        LD_C_A
        LD_A_MEM w3dcgb_full_cache_ty
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0x98
        LD_B_A
        LD_A_MEM w3dcgb_full_cache_slot
        ADD_A
        LD_L_A
        LD_H_IMM 0xDD
        LD_A_C
        LDI_HL_A
        LD_A_B
        LD_HL_A

        LD_A_MEM w3dcgb_full_cache_slot
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_L_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD5
        LD_H_A
        XOR_A
        LD_B_IMM 16
w3dfl_zero_tile:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfl_zero_tile
        LD_A_MEM w3dcgb_full_tile_count
        INC_A
        LD_MEM_A w3dcgb_full_tile_count
        JP w3dfl_have_slot

w3dfl_cache_slot:
        LD_MEM_A w3dcgb_full_cache_slot

w3dfl_have_slot:
        LD_A_MEM w3dcgb_full_cache_slot
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_MEM w3dcgb_line_y
        AND_IMM 7
        ADD_A
        ADD_C
        LD_L_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD5
        LD_H_A

        PUSH_HL
        LD_A_MEM w3dcgb_line_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        CPL
        LD_C_A
        LD_A_HL
        AND_C
        LD_HL_A
        INC_L
        LD_A_HL
        AND_C
        LD_HL_A

        LD_A_MEM w3dcgb_line_color
        AND_IMM 1
        JP_Z w3dfl_skip_low
        DEC_L
        LD_A_HL
        OR_B
        LD_HL_A
        INC_L
w3dfl_skip_low:
        LD_A_MEM w3dcgb_line_color
        AND_IMM 2
        RET_Z
        LD_A_HL
        OR_B
        LD_HL_A
        RET
    }
}

#pragma fixed_order 138
// Transfer allocated full-screen tiles from D500 to the alternate tile-ID
// range in VRAM bank zero: 8010 or 8800. pending_tile_bank selects these
// address ranges, not VBK. Empty frames skip DMA. Otherwise wait for a fresh
// VBlank and finish HBlank DMA with stable WRAM mapping and an enabled LCD.
// The allocator must keep the count within 127; SVBK/IME are not managed.
void w3dcgb_full_transfer_asm()
{
    /* Pending selects the two tile-address ranges, not the CGB VBK register. */
    __asm {
        LD_A_MEM w3dcgb_display_tile_bank
        XOR_IMM 1
        LD_MEM_A w3dcgb_pending_tile_bank
        XOR_A
        LDH_MEM_A 79

        LD_A_MEM w3dcgb_full_tile_count
        OR_A
        JP_Z w3dft_done

w3dft_wait_vblank_end:
        LDH_A_MEM 68
        CP_IMM 144
        JP_NC w3dft_wait_vblank_end
w3dft_wait_vblank_start:
        LDH_A_MEM 68
        CP_IMM 144
        JP_C w3dft_wait_vblank_start

        LD_A_IMM 0xD5
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_MEM w3dcgb_pending_tile_bank
        OR_A
        JP_Z w3dft_low_tiles
        LD_A_IMM 0x88
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        JP w3dft_dest_ready
w3dft_low_tiles:
        LD_A_IMM 0x80
        LDH_MEM_A 83
        LD_A_IMM 0x10
        LDH_MEM_A 84
w3dft_dest_ready:
        LD_A_MEM w3dcgb_full_tile_count
        DEC_A
        OR_IMM 0x80
        LDH_MEM_A 85
w3dft_wait_dma:
        LDH_A_MEM 85
        AND_IMM 0x80
        JP_Z w3dft_wait_dma

w3dft_done:
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

#pragma fixed_order 139
// At a fresh VBlank, clear previous map positions from DE00, then write
// current positions from DD00 with sequential IDs starting at 1 or 128.
// Update the displayed range and copy current addresses into the previous
// list. Require a stable WRAM mapping, enabled LCD and counts <=127.
// Direct map stores rely on the VBlank window; VBK is left zero.
void w3dcgb_full_present_asm()
{
    __asm {
w3dfp_wait_vblank_end:
        LDH_A_MEM 68
        CP_IMM 144
        JP_NC w3dfp_wait_vblank_end
w3dfp_wait_vblank_start:
        LDH_A_MEM 68
        CP_IMM 144
        JP_C w3dfp_wait_vblank_start

        XOR_A
        LDH_MEM_A 79
        LD_A_MEM w3dcgb_full_prev_tile_count
        OR_A
        JP_Z w3dfp_write_tiles
        LD_B_A
        LD_HL_IMM 0xDE00
w3dfp_clear_old:
        LDI_A_HL
        LD_E_A
        LDI_A_HL
        LD_D_A
// Long old/new map lists can exceed VBlank. Wait at each write so mode 3
// cannot discard map updates; A is reloaded after checking STAT.
w3dfp_clear_wait:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3dfp_clear_wait
        XOR_A
        LD_DE_A
        DEC_B
        JR_NZ w3dfp_clear_old

w3dfp_write_tiles:
        LD_A_MEM w3dcgb_full_tile_count
        OR_A
        JP_Z w3dfp_finish_map
        LD_B_A
        LD_A_MEM w3dcgb_pending_tile_bank
        OR_A
        JP_Z w3dfp_low_tile_ids
        LD_A_IMM 128
        JP w3dfp_tile_ids_ready
w3dfp_low_tile_ids:
        LD_A_IMM 1
w3dfp_tile_ids_ready:
        LD_MEM_A w3dcgb_full_map_tile
        LD_HL_IMM 0xDD00
w3dfp_tile_loop:
        LDI_A_HL
        LD_E_A
        LDI_A_HL
        LD_D_A
w3dfp_tile_wait:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3dfp_tile_wait
        LD_A_MEM w3dcgb_full_map_tile
        LD_DE_A
        INC_A
        LD_MEM_A w3dcgb_full_map_tile
        DEC_B
        JR_NZ w3dfp_tile_loop

w3dfp_finish_map:
        LD_A_MEM w3dcgb_pending_tile_bank
        LD_MEM_A w3dcgb_display_tile_bank
        XOR_A
        LDH_MEM_A 79

        LD_A_MEM w3dcgb_full_tile_count
        LD_MEM_A w3dcgb_full_prev_tile_count
        OR_A
        RET_Z
        LD_B_A
        LD_HL_IMM 0xDD00
        LD_DE_IMM 0xDE00
w3dfp_copy_list:
        LD_A_HL
        INC_HL
        LD_DE_A
        INC_DE
        LD_A_HL
        INC_HL
        LD_DE_A
        INC_DE
        DEC_B
        JR_NZ w3dfp_copy_list
        RET
    }
}

#pragma fixed_order 140
// Walk a full-screen line with incremental pixel pointers and bit masks,
// reselecting/allocating tiles at boundaries. Tile lookup is D300, payload
// D500 and map-address list DD00 in the caller WRAM mapping. Allocate at
// most 127 tiles, including tiles reached before an occlusion test. Invalid
// coordinates or allocation exhaustion set overflow, suppressing subsequent
// plots until frame reset. Colors 1/2 replace both planes; other values,
// including zero, draw color 3. No ordinary sparse dirty bookkeeping is used.
void w3dcgb_full_line_fast_asm()
{
    __asm {
w3dff_enter:
        LD_A_MEM w3dcgb_line_x0
        LD_MEM_A w3dcgb_line_x
        LD_B_A
        LD_A_MEM w3dcgb_line_x1
        CP_B
        JP_C w3dff_x_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sx
        JP w3dff_y_setup
w3dff_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dx
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sx

w3dff_y_setup:
        LD_A_MEM w3dcgb_line_y0
        LD_MEM_A w3dcgb_line_y
        LD_B_A
        LD_A_MEM w3dcgb_line_y1
        CP_B
        JP_C w3dff_y_reverse
        SUB_B
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 1
        LD_MEM_A w3dcgb_line_sy
        JP w3dff_ready
w3dff_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3dcgb_line_dy
        LD_A_IMM 255
        LD_MEM_A w3dcgb_line_sy

w3dff_ready:
        CALL w3dff_select_pixel
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_dy
        CP_C
        JP_C w3dff_shallow
        JP_Z w3dff_shallow
        JP w3dff_steep

w3dff_shallow:
        LD_A_MEM w3dcgb_line_dx
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err
w3dff_shallow_loop:
        CALL w3dff_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        RET_Z
        DEC_A
        LD_MEM_A w3dcgb_line_remaining
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dff_shallow_no_y
        CALL w3dff_step_y
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err
w3dff_shallow_no_y:
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dff_step_x
        JP w3dff_shallow_loop

w3dff_steep:
        LD_A_MEM w3dcgb_line_dy
        LD_MEM_A w3dcgb_line_remaining
        OR_A
        RRA
        LD_MEM_A w3dcgb_line_err
w3dff_steep_loop:
        CALL w3dff_plot
        LD_A_MEM w3dcgb_line_remaining
        OR_A
        RET_Z
        DEC_A
        LD_MEM_A w3dcgb_line_remaining
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        CP_C
        JP_NC w3dff_steep_no_x
        CALL w3dff_step_x
        LD_A_MEM w3dcgb_line_dy
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        ADD_C
        LD_MEM_A w3dcgb_line_err
w3dff_steep_no_x:
        LD_A_MEM w3dcgb_line_dx
        LD_C_A
        LD_A_MEM w3dcgb_line_err
        SUB_C
        LD_MEM_A w3dcgb_line_err
        CALL w3dff_step_y
        JP w3dff_steep_loop

w3dff_step_x:
        LD_A_MEM w3dcgb_line_sx
        CP_IMM 1
        JP_Z w3dff_step_x_right
        LD_A_MEM w3dcgb_line_x
        DEC_A
        LD_MEM_A w3dcgb_line_x
        LD_A_B
        ADD_A
        LD_B_A
        RET_NZ
        JP w3dff_select_pixel
w3dff_step_x_right:
        LD_A_MEM w3dcgb_line_x
        INC_A
        LD_MEM_A w3dcgb_line_x
        LD_A_B
        OR_A
        RRA
        LD_B_A
        // RRA always clears Z; test the shifted mask before returning.
        OR_A
        RET_NZ
        JP w3dff_select_pixel

w3dff_step_y:
        LD_A_MEM w3dcgb_line_sy
        CP_IMM 1
        JP_Z w3dff_step_y_down
        LD_A_MEM w3dcgb_line_y
        LD_C_A
        DEC_A
        LD_MEM_A w3dcgb_line_y
        LD_A_C
        AND_IMM 7
        JP_Z w3dff_select_pixel
        DEC_HL
        DEC_HL
        RET
w3dff_step_y_down:
        LD_A_MEM w3dcgb_line_y
        LD_C_A
        INC_A
        LD_MEM_A w3dcgb_line_y
        LD_A_C
        AND_IMM 7
        CP_IMM 7
        JP_Z w3dff_select_pixel
        INC_HL
        INC_HL
        RET

w3dff_plot:
        LD_A_MEM w3dcgb_full_overflow
        OR_A
        RET_NZ
        LD_A_MEM w3dcgb_occlusion_active
        OR_A
        JP_Z w3dff_draw
        PUSH_HL
        PUSH_BC
        LD_A_MEM w3dcgb_line_y
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_occlusion_mask
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dff_mask1
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dff_mask1
        LD_HL_IMM w3dcgb_occlusion_mask+144
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dff_hidden
        JP_Z w3dff_hidden
w3dff_mask1:
        LD_HL_IMM w3dcgb_occlusion_mask+288
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_Z w3dff_visible
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dff_visible
        LD_HL_IMM w3dcgb_occlusion_mask+432
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        LD_A_MEM w3dcgb_line_x
        CP_B
        JP_C w3dff_hidden
        JP_Z w3dff_hidden
w3dff_visible:
        POP_BC
        POP_HL
        JP w3dff_draw
w3dff_hidden:
        POP_BC
        POP_HL
        RET

w3dff_draw:
        LD_A_MEM w3dcgb_line_color
        CP_IMM 1
        JP_Z w3dff_draw_low
        CP_IMM 2
        JP_Z w3dff_draw_main

        LD_A_HL
        OR_B
        LD_HL_A
        INC_L
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_L
        RET

w3dff_draw_low:
        LD_A_HL
        OR_B
        LD_HL_A
        LD_A_B
        CPL
        LD_C_A
        INC_L
        LD_A_HL
        AND_C
        LD_HL_A
        DEC_L
        RET

w3dff_draw_main:
        LD_A_B
        CPL
        LD_C_A
        LD_A_HL
        AND_C
        LD_HL_A
        INC_L
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_L
        RET

// Resolve the 20-column tile lookup only at a tile boundary; allocate and
// zero a tile before testing visibility of its individual pixels.
w3dff_select_pixel:
        LD_A_MEM w3dcgb_line_x
        CP_IMM 160
        JP_NC w3dff_select_overflow
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3dcgb_full_cache_tx
        LD_B_A
        LD_A_MEM w3dcgb_line_y
        CP_IMM 144
        JP_NC w3dff_select_overflow
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3dcgb_full_cache_ty
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        LD_D_H
        LD_E_L
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_DE
        LD_A_B
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_DE_IMM w3dcgb_stage
        ADD_HL_DE
        LD_A_HL
        CP_IMM 0xFF
        JP_NZ w3dff_select_slot

        LD_A_MEM w3dcgb_full_tile_count
        CP_IMM 127
        JP_NC w3dff_select_overflow
        LD_HL_A
        LD_MEM_A w3dcgb_full_cache_slot
        LD_A_MEM w3dcgb_full_cache_ty
        AND_IMM 7
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_B_A
        LD_A_MEM w3dcgb_full_cache_tx
        ADD_B
        LD_C_A
        LD_A_MEM w3dcgb_full_cache_ty
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0x98
        LD_B_A
        LD_A_MEM w3dcgb_full_cache_slot
        ADD_A
        LD_L_A
        LD_H_IMM 0xDD
        LD_A_C
        LDI_HL_A
        LD_A_B
        LD_HL_A

        LD_A_MEM w3dcgb_full_cache_slot
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_L_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD5
        LD_H_A
        XOR_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LD_HL_A
        LD_A_MEM w3dcgb_full_tile_count
        INC_A
        LD_MEM_A w3dcgb_full_tile_count
        JP w3dff_select_address

w3dff_select_slot:
        LD_MEM_A w3dcgb_full_cache_slot
w3dff_select_address:
        LD_A_MEM w3dcgb_full_cache_slot
        LD_B_A
        AND_IMM 0x0F
        ADD_A
        ADD_A
        ADD_A
        ADD_A
        LD_C_A
        LD_A_MEM w3dcgb_line_y
        AND_IMM 7
        ADD_A
        ADD_C
        LD_L_A
        LD_A_B
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD5
        LD_H_A
        PUSH_HL
        LD_A_MEM w3dcgb_line_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3dcgb_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL
        RET

// Latch the frame fault. The plot helper suppresses writes after this point;
// BeginFrame clears the flag for the next frame.
w3dff_select_overflow:
        LD_A_IMM 1
        LD_MEM_A w3dcgb_full_overflow
        RET
    }
}
#pragma fixed_order -1
#pragma fixed_bank -1
#pragma bank 1

// Enable map flipping once during 128x96 startup; ignore full-screen mode
// or an already-enabled setting. With LCD on, wait for VBlank, disable it,
// copy both banks of 9800 to 9C00, and assign viewport attributes to tile
// banks 0/1 respectively. Restore LCDC with map 9C00 selected and VBK zero.
// Both BG maps become reserved. Queued HUD tile numbers are mirrored.
void Wire3DCGB_EnableAtomicMaps()
{
    if ((w3dcgb_full_mode != 0) || (w3dcgb_atomic_maps != 0)) return;
    __asm {
w3dam_wait:
        LDH_A_MEM 68
        CP_IMM 144
        JR_C w3dam_wait
        LDH_A_MEM 64
        PUSH_AF
        XOR_A
        LDH_MEM_A 64
        LDH_MEM_A 79
        LD_HL_IMM 0x9800
        LD_DE_IMM 0x9C00
        LD_B_IMM 4
w3dam_copy_page:
        LD_C_IMM 0
w3dam_copy_byte:
        LD_A_HL
        INC_HL
        LD_DE_A
        INC_DE
        DEC_C
        JR_NZ w3dam_copy_byte
        DEC_B
        JR_NZ w3dam_copy_page
        LDH_A_MEM 79
        AND_IMM 1
        JR_NZ w3dam_attrs
        LD_A_IMM 1
        LDH_MEM_A 79
        LD_HL_IMM 0x9800
        LD_DE_IMM 0x9C00
        LD_B_IMM 4
        JR w3dam_copy_page
w3dam_attrs:
        LD_HL_IMM 0x9800
        LD_DE_IMM 0x9C00
        LD_B_IMM 12
w3dam_row:
        LD_C_IMM 16
w3dam_cell:
        XOR_A
        LDI_HL_A
        LD_A_IMM 8
        LD_DE_A
        INC_DE
        DEC_C
        JR_NZ w3dam_cell
        LD_A_L
        ADD_A_IMM 16
        LD_L_A
        JR_NC w3dam_hl_ready
        INC_H
w3dam_hl_ready:
        LD_A_E
        ADD_A_IMM 16
        LD_E_A
        JR_NC w3dam_de_ready
        INC_D
w3dam_de_ready:
        DEC_B
        JR_NZ w3dam_row
        LD_A_IMM 1
        LD_MEM_A w3dcgb_atomic_maps
        XOR_A
        LDH_MEM_A 79
        POP_AF
        OR_IMM 8
        LDH_MEM_A 64
        RET
    }
}

// Initialize the CGB-only 128x96 renderer: enable double speed, turn the
// LCD off at VBlank, clear both tile banks, load HUD/generated tiles, and
// install a 16x12 column-major viewport. Start with blank tile bank 1, scroll
// zero, default BG palettes, color 2 and a zero camera. Reset queues and
// clear the bank-2 stage. Call BeginFrame before drawing to initialize
// occlusion state. Initialization replaces LCD/tile/map state globally.
void Wire3DCGB_Init()
{
    w3dcgb_u8 row;
    w3dcgb_u8 col;
    w3dcgb_u8 tid;
    w3dcgb_u8 tile_index;
    w3dcgb_u16 off;

    w3dcgb_full_mode = 0;
    w3dcgb_atomic_maps = 0;
    Wire3DCGB_EnableDoubleSpeed();
    w3dcgb_wait_vblank_start();
    w3dcgb_reg_lcdc = 0;

    w3dcgb_clear_vram_asm();
    w3dcgb_load_hud_tiles();
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
    w3dcgb_load_fast_map_tiles();
    w3dcgb_load_fast_stamp_tiles();
#endif
    w3dcgb_fill_bg_map_asm();
    w3dcgb_fill_attr_map_asm();

    row = 0;
    while (row < W3DCGB_TILE_H)
    {
        off = (w3dcgb_u16)((w3dcgb_u16)row << 5);
        tile_index = row;
        col = 0;
        while (col < W3DCGB_TILE_W)
        {
            if (tile_index < 112) tid = (w3dcgb_u8)(0x90 + tile_index);
            else tid = (w3dcgb_u8)(tile_index - 112);
            w3dcgb_bg_map_9800[(w3dcgb_u16)(off + (w3dcgb_u16)col)] = tid;
            tile_index = (w3dcgb_u8)(tile_index + W3DCGB_TILE_H);
            col = (w3dcgb_u8)(col + 1);
        }
        row = (w3dcgb_u8)(row + 1);
    }

    w3dcgb_select_blank_render_bank_asm();

    w3dcgb_reg_scx = 0;
    w3dcgb_reg_scy = 0;
    w3dcgb_reg_bgp = 0xB4;
    Wire3DCGB_SetPaletteRGB15(
        WIRE3DCGB_RGB15(0, 0, 2),
        WIRE3DCGB_RGB15(10, 20, 31),
        WIRE3DCGB_RGB15(31, 31, 31),
        WIRE3DCGB_RGB15(31, 14, 4));
    w3dcgb_line_color = WIRE3DCGB_COLOR_DEFAULT;
    w3dcgb_reg_lcdc = 0x81;
    w3dcgb_bgq_count = 0;
    w3dcgb_dirty_count = 0;
    w3dcgb_prev_dirty_count = 0;
    w3dcgb_dirty_min_tile = 0xFF;
    w3dcgb_dirty_max_tile = 0;
    w3dcgb_prev_dirty_min_tile = 0xFF;
    w3dcgb_prev_dirty_max_tile = 0;
    w3dcgb_display_tile_bank = 1;
    w3dcgb_pending_tile_bank = 1;

    Wire3DCGB_SetCamera((w3dcgb_i16)0, (w3dcgb_i16)0, (w3dcgb_i16)0, 0, 0, 0);
    w3dcgb_clear_stage_asm();
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Initialize the CGB-only 160x144 tile allocator. Clear both tile banks
// and map 9800, set scroll/palettes/color/camera, reset tile counts and
// lookup/span state, then enable unsigned BG tile addressing. HUD tiles
// are not loaded. Reserve its fixed WRAM lookup, payload and address lists
// and keep the same WRAM bank mapped through full-screen rendering.
void Wire3DCGB_InitFullScreen()
{
    w3dcgb_atomic_maps = 0;
    Wire3DCGB_EnableDoubleSpeed();
    w3dcgb_wait_vblank_start();
    w3dcgb_reg_lcdc = 0;

    w3dcgb_clear_vram_asm();
    w3dcgb_full_clear_map_asm();
    w3dcgb_reg_scx = 0;
    w3dcgb_reg_scy = 0;
    w3dcgb_reg_bgp = 0xB4;
    Wire3DCGB_SetPaletteRGB15(
        WIRE3DCGB_RGB15(0, 0, 2),
        WIRE3DCGB_RGB15(10, 20, 31),
        WIRE3DCGB_RGB15(31, 31, 31),
        WIRE3DCGB_RGB15(31, 14, 4));
    w3dcgb_line_color = WIRE3DCGB_COLOR_DEFAULT;
    w3dcgb_bgq_count = 0;
    w3dcgb_dirty_count = 0;
    w3dcgb_prev_dirty_count = 0;
    w3dcgb_display_tile_bank = 0;
    w3dcgb_pending_tile_bank = 0;
    w3dcgb_full_tile_count = 0;
    w3dcgb_full_prev_tile_count = 0;
    w3dcgb_full_overflow = 0;
    w3dcgb_full_mode = 1;

    Wire3DCGB_SetCamera((w3dcgb_i16)0, (w3dcgb_i16)0, (w3dcgb_i16)0, 0, 0, 0);
    w3dcgb_full_begin_asm();
    w3dcgb_reg_lcdc = 0x91;
}

// Initialize a CGB tile-map renderer with HUD and 48 precomputed outline
// tiles. Clear tile VRAM/maps, reset scroll/palettes/camera and select normal
// mode. This omits normal pixel-viewport map layout and generated cube stamps;
// use the FastMap APIs for this mode. Clear the bank-2 stage and mark fast
// map preparation pending. Full-runtime build required.
void Wire3DCGB_InitFastMapLite()
{
    w3dcgb_atomic_maps = 0;
    w3dcgb_full_mode = 0;
    Wire3DCGB_EnableDoubleSpeed();
    w3dcgb_wait_vblank_start();
    w3dcgb_reg_lcdc = 0;

    w3dcgb_clear_vram_asm();
    w3dcgb_load_hud_tiles();
    w3dcgb_load_fast_map_tiles_lite_asm();
    w3dcgb_fill_bg_map_asm();
    w3dcgb_fill_attr_map_asm();

    w3dcgb_reg_scx = 0;
    w3dcgb_reg_scy = 0;
    w3dcgb_reg_bgp = 0xB4;
    Wire3DCGB_SetPaletteRGB15(
        WIRE3DCGB_RGB15(0, 0, 2),
        WIRE3DCGB_RGB15(10, 20, 31),
        WIRE3DCGB_RGB15(31, 31, 31),
        WIRE3DCGB_RGB15(31, 14, 4));
    w3dcgb_line_color = WIRE3DCGB_COLOR_DEFAULT;
    w3dcgb_reg_lcdc = 0x81;
    w3dcgb_bgq_count = 0;
    w3dcgb_dirty_count = 0;
    w3dcgb_prev_dirty_count = 0;
    w3dcgb_dirty_min_tile = 0xFF;
    w3dcgb_dirty_max_tile = 0;
    w3dcgb_prev_dirty_min_tile = 0xFF;
    w3dcgb_prev_dirty_max_tile = 0;
    w3dcgb_fast_map_ready = 0;
    w3dcgb_display_tile_bank = 0;
    w3dcgb_pending_tile_bank = 0;

    Wire3DCGB_SetCamera((w3dcgb_i16)0, (w3dcgb_i16)0, (w3dcgb_i16)0, 0, 0, 0);
    w3dcgb_clear_stage_asm();
}
#endif

#pragma bank 2
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// In normal mode, select full-stage transfer, clear all staged pixels and
// the bitmap mask, reset mask bounds, and disable occlusion. In full-screen
// mode, reset that mode's allocator/spans instead. Pending BG writes remain queued.
// Full-runtime build required.
void Wire3DCGB_BeginFrame()
{
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_begin_asm();
        return;
    }
    w3dcgb_full_frame_transfer = 1;
    w3dcgb_clear_stage_asm();
    w3dcgb_clear_occlusion_mask_asm();
    w3dcgb_occ_min_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    w3dcgb_occ_min_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    w3dcgb_occ_max_x = 0;
    w3dcgb_occ_max_y = 0;
    w3dcgb_occlusion_active = 0;
}
#endif

// Perform the same normal/full-screen frame reset as BeginFrame, while
// first marking both previous sparse-history ranges as the entire 192-tile
// viewport. This prevents a subsequent sparse upload from trusting stale banks.
void Wire3DCGB_BeginFrameFast()
{
    w3dcgb_older_dirty_min_tile = 0;
    w3dcgb_older_dirty_max_tile = 191;
    w3dcgb_prev_dirty_min_tile = 0;
    w3dcgb_prev_dirty_max_tile = 191;
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_begin_asm();
        return;
    }
    w3dcgb_full_frame_transfer = 1;
    w3dcgb_clear_stage_asm();
    w3dcgb_clear_occlusion_mask_asm();
    w3dcgb_occ_min_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    w3dcgb_occ_min_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    w3dcgb_occ_max_x = 0;
    w3dcgb_occ_max_y = 0;
    w3dcgb_occlusion_active = 0;
}

// Select sparse transfer and clear the entire 128x96 stage, avoiding stale
// pixels when the dirty range expands. Reset mask bounds and disable
// occlusion; PAINTER_ONLY builds skip clearing the unused bitmap mask.
// Full-screen mode delegates to its own allocator reset.
void Wire3DCGB_BeginFrameSparse()
{
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_begin_asm();
        return;
    }
    w3dcgb_full_frame_transfer = 0;
    w3dcgb_dirty_count = 0;
    /* One ASM pass is faster than repeated range-growth clears and guarantees
       that newly exposed dirty tiles can never copy stale stage pixels. */
    w3dcgb_clear_stage_asm();
#ifndef WIRE3DCGB_PAINTER_ONLY
    /* A Z-sorted painter erases silhouettes directly in the stage and never
       reads the separate front-to-back coverage mask. */
    w3dcgb_clear_occlusion_mask_asm();
#endif
    w3dcgb_occ_min_x = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    w3dcgb_occ_min_y = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    w3dcgb_occ_max_x = 0;
    w3dcgb_occ_max_y = 0;
    w3dcgb_occlusion_active = 0;
}

// Clear the entire bank-2 stage and mark the full 128x96 viewport dirty.
// Despite its name this is not a partial clear and does not preserve staged
// pixels. It neither clears occlusion nor selects a transfer mode.
void Wire3DCGB_ClearSparseStageFast()
{
    w3dcgb_clear_stage_asm();
    w3dcgb_mark_dirty_rect(0, 0,
                           (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1),
                           (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1));
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// For the 128x96 renderer, wait for VBlank, disable the LCD, clear the stage
// and frame tile ranges in both banks, reset map attributes, then enable LCDC
// 0x81. Clear dirty flags/counts but not sparse min/max history. This does
// not reset full_mode and is not a full-screen reinitializer.
void Wire3DCGB_ClearFrameTiles()
{
    w3dcgb_u8 i;

    w3dcgb_wait_vblank_start();
    w3dcgb_reg_lcdc = 0;
    w3dcgb_clear_stage_asm();
    w3dcgb_clear_frame_tiles_vram_asm();
    w3dcgb_fill_attr_map_asm();
    w3dcgb_reg_lcdc = 0x81;

    i = 0;
    while (i < W3DCGB_DIRTY_TILE_LIMIT)
    {
        w3dcgb_dirty_flags[(__safe_index w3dcgb_u8)i] = 0;
        w3dcgb_prev_dirty_flags[(__safe_index w3dcgb_u8)i] = 0;
        i = (w3dcgb_u8)(i + 1);
    }
    w3dcgb_dirty_count = 0;
    w3dcgb_prev_dirty_count = 0;
}
#endif

// Store camera position and angles without changing existing pixels. Rotation
// helpers use the low four angle bits (16 steps per turn). Screen offsets are
// separate LCD registers and are not changed by this call.
void Wire3DCGB_SetCamera(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 pitch, w3dcgb_i8 yaw, w3dcgb_i8 roll)
{
    w3dcgb_cam_x = x;
    w3dcgb_cam_y = y;
    w3dcgb_cam_z = z;
    w3dcgb_cam_pitch = pitch;
    w3dcgb_cam_yaw = yaw;
    w3dcgb_cam_roll = roll;
}

#pragma bank 4
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// In normal mode, reject coordinates outside 128x96, mark sparse bounds
// and replace the staged pixel using the current color, including zero.
// Full-screen mode draws a coincident-endpoint fast line instead, so zero
// means color 3 and invalid coordinates latch that frame's overflow flag.
// Display transfer occurs later.
void Wire3DCGB_DrawPoint2D(w3dcgb_u8 x, w3dcgb_u8 y)
{
    w3dcgb_u8 tile;
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_line_x0 = x;
        w3dcgb_line_y0 = y;
        w3dcgb_line_x1 = x;
        w3dcgb_line_y1 = y;
        w3dcgb_full_line_fast_asm();
        return;
    }
    if (x >= WIRE3DCGB_SCREEN_W) return;
    if (y >= WIRE3DCGB_SCREEN_H) return;
    if (w3dcgb_full_frame_transfer == 0)
    {
        tile = (w3dcgb_u8)(((x >> 3) * W3DCGB_TILE_H) + (y >> 3));
        if (tile < w3dcgb_dirty_min_tile) w3dcgb_dirty_min_tile = tile;
        if (tile > w3dcgb_dirty_max_tile) w3dcgb_dirty_max_tile = tile;
    }
    w3dcgb_plot_x = x;
    w3dcgb_plot_y = y;
    w3dcgb_plot_stage_asm();
}
#endif

// Draw a six-pixel marker through the 128x96 plotter while preserving line
// color. Centers outside X=3..124 or Y=3..92 are ignored. The low five turn
// bits select left/straight/right nose and tail offsets; this is not a general
// 3D transform and does not dispatch to the full-screen renderer.
void Wire3DCGB_DrawTinyModel2D(w3dcgb_u8 x, w3dcgb_u8 y,
                               w3dcgb_u8 turn, w3dcgb_u8 color)
{
    w3dcgb_u8 nose_x;
    w3dcgb_u8 tail_x;
    w3dcgb_u8 old_color;
    if ((x < 3) || (x > 124) || (y < 3) || (y > 92)) return;

    nose_x = x;
    tail_x = x;
    turn = (w3dcgb_u8)(turn & 31);
    if (turn < 12)
    {
        nose_x = (w3dcgb_u8)(x + 1);
        tail_x = (w3dcgb_u8)(x - 1);
    }
    else if (turn >= 20)
    {
        nose_x = (w3dcgb_u8)(x - 1);
        tail_x = (w3dcgb_u8)(x + 1);
    }

    old_color = w3dcgb_line_color;
    w3dcgb_line_color = (w3dcgb_u8)(color & 3);
    w3dcgb_plot_x = nose_x;
    w3dcgb_plot_y = (w3dcgb_u8)(y - 2);
    w3dcgb_plot_stage_asm();
    w3dcgb_plot_x = x;
    w3dcgb_plot_y = (w3dcgb_u8)(y - 1);
    w3dcgb_plot_stage_asm();
    w3dcgb_plot_x = (w3dcgb_u8)(x - 2);
    w3dcgb_plot_y = (w3dcgb_u8)(y + 1);
    w3dcgb_plot_stage_asm();
    w3dcgb_plot_x = (w3dcgb_u8)(x + 2);
    w3dcgb_plot_stage_asm();
    w3dcgb_plot_x = x;
    w3dcgb_plot_y = y;
    w3dcgb_plot_stage_asm();
    w3dcgb_plot_x = tail_x;
    w3dcgb_plot_y = (w3dcgb_u8)(y + 2);
    w3dcgb_plot_stage_asm();
    w3dcgb_line_color = old_color;
}

// Store byte endpoints and invoke the normal or full-screen line rasterizer.
// Provide coordinates inside the selected viewport; use DrawLineClipped2D
// for signed/off-screen geometry. Normal lines OR color planes and do not
// mark sparse bounds; use the clipped API or InvalidateFrameHistory before
// a sparse upload. Full-screen lines replace colors and use tile allocation.
// Both active line paths treat color zero as 3, not as an eraser.
void Wire3DCGB_DrawLine2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by)
{
    w3dcgb_line_x0 = ax;
    w3dcgb_line_y0 = ay;
    w3dcgb_line_x1 = bx;
    w3dcgb_line_y1 = by;
    if (w3dcgb_full_mode != 0) w3dcgb_full_line_fast_asm();
    else w3dcgb_line_stage_asm();
}

// Set color & 3 around DrawLine2D and restore the previous color. Follow
// that function's coordinate, color-combination and sparse-range contracts;
// zero does not erase a line in either active renderer.
void Wire3DCGB_DrawLine2DColor(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 color)
{
    w3dcgb_u8 old_color;
    old_color = w3dcgb_line_color;
    Wire3DCGB_SetLineColor(color);
    Wire3DCGB_DrawLine2D(ax, ay, bx, by);
    w3dcgb_line_color = old_color;
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Draw enabled model edges using signed-byte preprojected offsets around
// (x,y). Mask bits are least-significant-first, eight edges per byte. All
// pointers and vertex indices must be valid; edge_mask[0] is read even for
// zero edges. No counts are capped. Keep translated endpoints within the
// viewport before their byte conversion; negative values would wrap.
// Preserve the previous line color and dispatch by the active mode.
void Wire3DCGB_DrawMaskedModel2D(const Wire3DCGB_Model* model,
                                 const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                                 const w3dcgb_u8* edge_mask,
                                 w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color)
{
    w3dcgb_u8 i;
    w3dcgb_u8 bit;
    w3dcgb_u8 mask_index;
    w3dcgb_u8 mask;
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 old_color;

    old_color = w3dcgb_line_color;
    w3dcgb_line_color = (w3dcgb_u8)(color & 3);
    i = 0;
    bit = 1;
    mask_index = 0;
    mask = edge_mask[0];
    while (i < model->edge_count)
    {
        if ((mask & bit) != 0)
        {
            a = model->edges[(__safe_index w3dcgb_u8)i].a;
            b = model->edges[(__safe_index w3dcgb_u8)i].b;
            w3dcgb_line_x0 = (w3dcgb_u8)((w3dcgb_i16)x + (w3dcgb_i16)vertex_x[(__safe_index w3dcgb_u8)a]);
            w3dcgb_line_y0 = (w3dcgb_u8)((w3dcgb_i16)y + (w3dcgb_i16)vertex_y[(__safe_index w3dcgb_u8)a]);
            w3dcgb_line_x1 = (w3dcgb_u8)((w3dcgb_i16)x + (w3dcgb_i16)vertex_x[(__safe_index w3dcgb_u8)b]);
            w3dcgb_line_y1 = (w3dcgb_u8)((w3dcgb_i16)y + (w3dcgb_i16)vertex_y[(__safe_index w3dcgb_u8)b]);
            if (w3dcgb_full_mode != 0) w3dcgb_full_line_fast_asm();
            else w3dcgb_line_stage_asm();
        }
        i = (w3dcgb_u8)(i + 1);
        bit = (w3dcgb_u8)(bit << 1);
        if ((bit == 0) && (i < model->edge_count))
        {
            bit = 1;
            mask_index = (w3dcgb_u8)(mask_index + 1);
            mask = edge_mask[(__safe_index w3dcgb_u8)mask_index];
        }
    }
    w3dcgb_line_color = old_color;
}
#endif

const Wire3DCGB_Edge* w3dcgb_edge_list_ptr;
const w3dcgb_i8* w3dcgb_edge_x_ptr;
const w3dcgb_i8* w3dcgb_edge_y_ptr;
w3dcgb_u8 w3dcgb_edge_remaining;
w3dcgb_u8 w3dcgb_edge_center_x;
w3dcgb_u8 w3dcgb_edge_center_y;
w3dcgb_u8 w3dcgb_edge_second;

// Consume the shared edge pointer/count, add byte centers to indexed byte
// offsets modulo 256, and call the normal rasterizer. Zero count is a no-op.
// The caller supplies readable arrays and valid indices/translated endpoints;
// this loop does not validate bounds or manage source-bank mapping.
void w3dcgb_edge_list_asm()
{
    __asm {
        LD_A_MEM w3dcgb_edge_remaining
        OR_A
        RET_Z
w3dedge_next:
        LD_A_MEM w3dcgb_edge_list_ptr
        LD_L_A
        LD_A_MEM w3dcgb_edge_list_ptr+1
        LD_H_A
        LDI_A_HL
        LD_E_A
        LDI_A_HL
        LD_MEM_A w3dcgb_edge_second
        LD_A_L
        LD_MEM_A w3dcgb_edge_list_ptr
        LD_A_H
        LD_MEM_A w3dcgb_edge_list_ptr+1
        LD_D_IMM 0
        LD_A_MEM w3dcgb_edge_x_ptr
        LD_L_A
        LD_A_MEM w3dcgb_edge_x_ptr+1
        LD_H_A
        ADD_HL_DE
        LD_A_MEM w3dcgb_edge_center_x
        LD_B_A
        LD_A_HL
        ADD_B
        LD_MEM_A w3dcgb_line_x0
        LD_A_MEM w3dcgb_edge_y_ptr
        LD_L_A
        LD_A_MEM w3dcgb_edge_y_ptr+1
        LD_H_A
        ADD_HL_DE
        LD_A_MEM w3dcgb_edge_center_y
        LD_B_A
        LD_A_HL
        ADD_B
        LD_MEM_A w3dcgb_line_y0
        LD_A_MEM w3dcgb_edge_second
        LD_E_A
        LD_A_MEM w3dcgb_edge_x_ptr
        LD_L_A
        LD_A_MEM w3dcgb_edge_x_ptr+1
        LD_H_A
        ADD_HL_DE
        LD_A_MEM w3dcgb_edge_center_x
        LD_B_A
        LD_A_HL
        ADD_B
        LD_MEM_A w3dcgb_line_x1
        LD_A_MEM w3dcgb_edge_y_ptr
        LD_L_A
        LD_A_MEM w3dcgb_edge_y_ptr+1
        LD_H_A
        ADD_HL_DE
        LD_A_MEM w3dcgb_edge_center_y
        LD_B_A
        LD_A_HL
        ADD_B
        LD_MEM_A w3dcgb_line_y1
        CALL w3dcgb_line_stage_asm
        LD_A_MEM w3dcgb_edge_remaining
        DEC_A
        LD_MEM_A w3dcgb_edge_remaining
        JP_NZ w3dedge_next
        RET
    }
}

// Draw up to 64 indexed edges from signed-byte offsets around (cx,cy).
// Null arrays are ignored; indices must address readable vertices. Keep the
// translated endpoints in the viewport before byte conversion, which wraps.
// Dispatch by mode and preserve the old color; no vertex-count check occurs.
void Wire3DCGB_DrawEdgeList2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
                             const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                             w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color)
{
    w3dcgb_u8 i;
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 old_color;

    if (edges == 0) return;
    if (vertex_x == 0) return;
    if (vertex_y == 0) return;
    if (edge_count > WIRE3DCGB_MODEL_EDGE_LIMIT) edge_count = WIRE3DCGB_MODEL_EDGE_LIMIT;

    old_color = w3dcgb_line_color;
    w3dcgb_line_color = (w3dcgb_u8)(color & 3);
    if (w3dcgb_full_mode == 0)
    {
        w3dcgb_edge_list_ptr = edges;
        w3dcgb_edge_x_ptr = vertex_x;
        w3dcgb_edge_y_ptr = vertex_y;
        w3dcgb_edge_remaining = edge_count;
        w3dcgb_edge_center_x = x;
        w3dcgb_edge_center_y = y;
        w3dcgb_edge_list_asm();
        w3dcgb_line_color = old_color;
        return;
    }
    i = 0;
    while (i < edge_count)
    {
        a = edges[(__safe_index w3dcgb_u8)i].a;
        b = edges[(__safe_index w3dcgb_u8)i].b;
        w3dcgb_line_x0 = (w3dcgb_u8)((w3dcgb_i16)x + (w3dcgb_i16)vertex_x[(__safe_index w3dcgb_u8)a]);
        w3dcgb_line_y0 = (w3dcgb_u8)((w3dcgb_i16)y + (w3dcgb_i16)vertex_y[(__safe_index w3dcgb_u8)a]);
        w3dcgb_line_x1 = (w3dcgb_u8)((w3dcgb_i16)x + (w3dcgb_i16)vertex_x[(__safe_index w3dcgb_u8)b]);
        w3dcgb_line_y1 = (w3dcgb_u8)((w3dcgb_i16)y + (w3dcgb_i16)vertex_y[(__safe_index w3dcgb_u8)b]);
        if (w3dcgb_full_mode != 0) w3dcgb_full_line_fast_asm();
        else w3dcgb_line_stage_asm();
        i = (w3dcgb_u8)(i + 1);
    }
    w3dcgb_line_color = old_color;
}

// Draw up to 64 records of four bytes (x0,y0,x1,y1), ignoring a null list.
// Endpoints must lie in the selected viewport. Preserve the previous color
// and dispatch each line to the normal or full-screen rasterizer.
void Wire3DCGB_DrawLineList2DColor(const w3dcgb_u8* line_xy,
                                  w3dcgb_u8 line_count, w3dcgb_u8 color)
{
    w3dcgb_u8 i;
    w3dcgb_u8 offset;
    w3dcgb_u8 old_color;
    if (line_xy == 0) return;
    if (line_count > WIRE3DCGB_MODEL_EDGE_LIMIT) line_count = WIRE3DCGB_MODEL_EDGE_LIMIT;

    old_color = w3dcgb_line_color;
    w3dcgb_line_color = (w3dcgb_u8)(color & 3);
    i = 0;
    offset = 0;
    while (i < line_count)
    {
        w3dcgb_line_x0 = line_xy[(__safe_index w3dcgb_u8)offset];
        offset = (w3dcgb_u8)(offset + 1);
        w3dcgb_line_y0 = line_xy[(__safe_index w3dcgb_u8)offset];
        offset = (w3dcgb_u8)(offset + 1);
        w3dcgb_line_x1 = line_xy[(__safe_index w3dcgb_u8)offset];
        offset = (w3dcgb_u8)(offset + 1);
        w3dcgb_line_y1 = line_xy[(__safe_index w3dcgb_u8)offset];
        offset = (w3dcgb_u8)(offset + 1);
        if (w3dcgb_full_mode != 0) w3dcgb_full_line_fast_asm();
        else w3dcgb_line_stage_asm();
        i = (w3dcgb_u8)(i + 1);
    }
    w3dcgb_line_color = old_color;
}

#pragma bank 2
// Submit shared byte endpoints to the normal 128x96 rasterizer only.
// This helper does not dispatch to the full-screen path.
static void w3dcgb_draw_line_raw(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by)
{
    w3dcgb_line_x0 = ax;
    w3dcgb_line_y0 = ay;
    w3dcgb_line_x1 = bx;
    w3dcgb_line_y1 = by;
    w3dcgb_line_stage_asm();
}

// Clamp a signed X coordinate to the normal viewport, 0..127.
static w3dcgb_u8 w3dcgb_fast_screen_x(w3dcgb_i16 v)
{
    if (v < 0) return 0;
    if (v > (w3dcgb_i16)(WIRE3DCGB_SCREEN_W - 1)) return (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    return (w3dcgb_u8)v;
}

// Clamp a signed Y coordinate to the normal viewport, 0..95.
static w3dcgb_u8 w3dcgb_fast_screen_y(w3dcgb_i16 v)
{
    if (v < 0) return 0;
    if (v > (w3dcgb_i16)(WIRE3DCGB_SCREEN_H - 1)) return (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);
    return (w3dcgb_u8)v;
}

// Mark the bounding rectangle of the eight precomputed cube corners dirty.
static void w3dcgb_fast_mark_cube(w3dcgb_u8 x0, w3dcgb_u8 y0, w3dcgb_u8 x1, w3dcgb_u8 y1,
                                  w3dcgb_u8 bx0, w3dcgb_u8 by0, w3dcgb_u8 bx1, w3dcgb_u8 by1)
{
    w3dcgb_u8 min_x;
    w3dcgb_u8 max_x;
    w3dcgb_u8 min_y;
    w3dcgb_u8 max_y;

    min_x = x0;
    if (x1 < min_x) min_x = x1;
    if (bx0 < min_x) min_x = bx0;
    if (bx1 < min_x) min_x = bx1;
    max_x = x0;
    if (x1 > max_x) max_x = x1;
    if (bx0 > max_x) max_x = bx0;
    if (bx1 > max_x) max_x = bx1;

    min_y = y0;
    if (y1 < min_y) min_y = y1;
    if (by0 < min_y) min_y = by0;
    if (by1 < min_y) min_y = by1;
    max_y = y0;
    if (y1 > max_y) max_y = y1;
    if (by0 > max_y) max_y = by0;
    if (by1 > max_y) max_y = by1;

    w3dcgb_mark_dirty_rect(min_x, min_y, max_x, max_y);
}

// Draw a 2D cube approximation around (64+x,48-y); camera and rotation
// angles are ignored. Three depth bands select size/offset, and each corner
// is clamped independently. Hidden mode uses a fixed nine-edge outline;
// otherwise draw all twelve edges. Mark its bounds and optionally draw a
// selection cross. This normal-mode helper leaves the line color changed.
static void w3dcgb_draw_fast_cube_one(const Wire3DCGB_FastCube* cube, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected)
{
    w3dcgb_i16 cx16;
    w3dcgb_i16 cy16;
    w3dcgb_i16 cube_size;
    w3dcgb_i16 cube_shift;
    w3dcgb_u8 x0;
    w3dcgb_u8 y0;
    w3dcgb_u8 x1;
    w3dcgb_u8 y1;
    w3dcgb_u8 bx0;
    w3dcgb_u8 by0;
    w3dcgb_u8 bx1;
    w3dcgb_u8 by1;
    w3dcgb_u8 cx;
    w3dcgb_u8 cy;

    cx16 = (w3dcgb_i16)((w3dcgb_i16)64 + cube->x);
    cy16 = (w3dcgb_i16)((w3dcgb_i16)48 - cube->y);
    if (cube->z < (w3dcgb_i16)100)
    {
        cube_size = 16;
        cube_shift = 8;
    }
    else if (cube->z < (w3dcgb_i16)146)
    {
        cube_size = 13;
        cube_shift = 6;
    }
    else
    {
        cube_size = 10;
        cube_shift = 5;
    }

    x0 = w3dcgb_fast_screen_x((w3dcgb_i16)(cx16 - cube_size));
    y0 = w3dcgb_fast_screen_y((w3dcgb_i16)(cy16 - cube_size));
    x1 = w3dcgb_fast_screen_x((w3dcgb_i16)(cx16 + cube_size));
    y1 = w3dcgb_fast_screen_y((w3dcgb_i16)(cy16 + cube_size));
    bx0 = w3dcgb_fast_screen_x((w3dcgb_i16)(cx16 - cube_size + cube_shift));
    by0 = w3dcgb_fast_screen_y((w3dcgb_i16)(cy16 - cube_size - cube_shift));
    bx1 = w3dcgb_fast_screen_x((w3dcgb_i16)(cx16 + cube_size + cube_shift));
    by1 = w3dcgb_fast_screen_y((w3dcgb_i16)(cy16 + cube_size - cube_shift));

    w3dcgb_fast_mark_cube(x0, y0, x1, y1, bx0, by0, bx1, by1);
    if (selected != 0)
    {
        cx = w3dcgb_fast_screen_x(cx16);
        cy = w3dcgb_fast_screen_y(cy16);
        if ((cx > 3) && (cx < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 4)) &&
            (cy > 3) && (cy < (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 4)))
        {
            w3dcgb_mark_dirty_rect((w3dcgb_u8)(cx - 3), (w3dcgb_u8)(cy - 3), (w3dcgb_u8)(cx + 3), (w3dcgb_u8)(cy + 3));
        }
    }

    w3dcgb_line_color = cube->color;
    if (hidden_enabled == 0)
    {
        w3dcgb_draw_line_raw(bx0, by0, bx1, by0);
        w3dcgb_draw_line_raw(bx1, by0, bx1, by1);
        w3dcgb_draw_line_raw(bx1, by1, bx0, by1);
        w3dcgb_draw_line_raw(bx0, by1, bx0, by0);
        w3dcgb_draw_line_raw(x0, y0, bx0, by0);
        w3dcgb_draw_line_raw(x1, y0, bx1, by0);
        w3dcgb_draw_line_raw(x1, y1, bx1, by1);
        w3dcgb_draw_line_raw(x0, y1, bx0, by1);
    }
    else
    {
        w3dcgb_draw_line_raw(bx0, by0, bx1, by0);
        w3dcgb_draw_line_raw(bx1, by0, bx1, by1);
        w3dcgb_draw_line_raw(x0, y0, bx0, by0);
        w3dcgb_draw_line_raw(x1, y0, bx1, by0);
        w3dcgb_draw_line_raw(x1, y1, bx1, by1);
    }

    w3dcgb_draw_line_raw(x0, y0, x1, y0);
    w3dcgb_draw_line_raw(x1, y0, x1, y1);
    w3dcgb_draw_line_raw(x1, y1, x0, y1);
    w3dcgb_draw_line_raw(x0, y1, x0, y0);

    if (selected != 0)
    {
        if ((cx > 3) && (cx < (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 4)))
        {
            w3dcgb_line_color = WIRE3DCGB_COLOR_HIGH;
            w3dcgb_draw_line_raw((w3dcgb_u8)(cx - 3), cy, (w3dcgb_u8)(cx + 3), cy);
        }
        if ((cy > 3) && (cy < (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 4)))
        {
            w3dcgb_line_color = WIRE3DCGB_COLOR_HIGH;
            w3dcgb_draw_line_raw(cx, (w3dcgb_u8)(cy - 3), cx, (w3dcgb_u8)(cy + 3));
        }
    }
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Draw at most three 2D cube approximations, sorted far-to-near by raw Z
// without reordering the caller array. Null/empty input is ignored. selected
// is an original array index. Camera/rotation are unused and color is not restored.
void Wire3DCGB_DrawFastCubes(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected)
{
    w3dcgb_u8 scene_count;
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 c;
    w3dcgb_u8 x;

    if (cubes == 0) return;
    scene_count = count;
    if (scene_count == 0) return;
    if (scene_count > 3) scene_count = 3;

    a = 0;
    b = 1;
    c = 2;
    if (scene_count == 1)
    {
        w3dcgb_draw_fast_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0));
        return;
    }

    if (scene_count == 2)
    {
        if (cubes[0].z < cubes[1].z)
        {
            w3dcgb_draw_fast_cube_one(&cubes[1], hidden_enabled, (w3dcgb_u8)(selected == 1));
            w3dcgb_draw_fast_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0));
        }
        else
        {
            w3dcgb_draw_fast_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0));
            w3dcgb_draw_fast_cube_one(&cubes[1], hidden_enabled, (w3dcgb_u8)(selected == 1));
        }
        return;
    }

    if (cubes[a].z < cubes[b].z)
    {
        w3dcgb_u8 t;
        t = a;
        a = b;
        b = t;
    }
    if (cubes[b].z < cubes[c].z)
    {
        w3dcgb_u8 t;
        t = b;
        b = c;
        c = t;
    }
    if (cubes[a].z < cubes[b].z)
    {
        w3dcgb_u8 t;
        t = a;
        a = b;
        b = t;
    }

    w3dcgb_draw_fast_cube_one(&cubes[a], hidden_enabled, (w3dcgb_u8)(selected == a));
    w3dcgb_draw_fast_cube_one(&cubes[b], hidden_enabled, (w3dcgb_u8)(selected == b));
    w3dcgb_draw_fast_cube_one(&cubes[c], hidden_enabled, (w3dcgb_u8)(selected == c));
}

// Draw a six-edge projectile at an already-projected center using four
// depth size bands and four phase orientations. Reject centers outside
// X=10..117 or Y=10..85, mark the bounds dirty and leave the requested color
// selected. This uses the normal rasterizer and does not apply the camera.
void Wire3DCGB_DrawFastProjectile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_i16 z,
                                  w3dcgb_u8 phase, w3dcgb_u8 color)
{
    w3dcgb_u8 radius;
    w3dcgb_u8 half;
    w3dcgb_u8 depth;
    w3dcgb_u8 ax;
    w3dcgb_u8 ay;
    w3dcgb_u8 bx;
    w3dcgb_u8 by;
    w3dcgb_u8 cx;
    w3dcgb_u8 cy;
    w3dcgb_u8 dx;
    w3dcgb_u8 dy;
    w3dcgb_u8 spin;

    if ((x < 10) || (x > 117) || (y < 10) || (y > 85)) return;
    radius = 3;
    depth = 1;
    if (z <= 160) { radius = 4; depth = 2; }
    if (z <= 112) { radius = 5; depth = 3; }
    if (z <= 64) { radius = 7; depth = 4; }
    half = (w3dcgb_u8)(radius >> 1);

    ax = x;
    ay = (w3dcgb_u8)(y - radius);
    bx = (w3dcgb_u8)(x - radius);
    by = (w3dcgb_u8)(y + half);
    cx = (w3dcgb_u8)(x + radius);
    cy = by;
    spin = (w3dcgb_u8)((phase >> 3) & 3);
    dx = (w3dcgb_u8)(x + depth);
    dy = (w3dcgb_u8)(y - depth);
    if (spin == 1) dy = (w3dcgb_u8)(y + depth);
    else if (spin == 2)
    {
        dx = (w3dcgb_u8)(x - depth);
        dy = (w3dcgb_u8)(y + depth);
    }
    else if (spin == 3) dx = (w3dcgb_u8)(x - depth);

    w3dcgb_mark_dirty_rect((w3dcgb_u8)(x - radius), (w3dcgb_u8)(y - radius),
                           (w3dcgb_u8)(x + radius), (w3dcgb_u8)(y + radius));
    w3dcgb_line_color = color;
    w3dcgb_draw_line_raw(ax, ay, bx, by);
    w3dcgb_draw_line_raw(bx, by, cx, cy);
    w3dcgb_draw_line_raw(cx, cy, ax, ay);
    w3dcgb_draw_line_raw(ax, ay, dx, dy);
    w3dcgb_draw_line_raw(bx, by, dx, dy);
    w3dcgb_draw_line_raw(cx, cy, dx, dy);
}

// Draw small selection/hidden-mode indicators in color 3 and mark their
// normal-mode bounds dirty. selected is expected in 0..2; it is not checked.
// These indicators use lines, not font tiles, and the old color is not restored.
void Wire3DCGB_DrawFastStatus(w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected)
{
    w3dcgb_u8 x;

    w3dcgb_mark_dirty_rect(0, 0, 31, 7);
    w3dcgb_mark_dirty_rect(116, 0, 127, 7);

    x = (w3dcgb_u8)(4 + (selected * 8));
    w3dcgb_line_color = WIRE3DCGB_COLOR_HIGH;
    w3dcgb_draw_line_raw(x, 3, (w3dcgb_u8)(x + 5), 3);
    w3dcgb_draw_line_raw(x, 6, (w3dcgb_u8)(x + 5), 6);
    if (hidden_enabled != 0)
    {
        w3dcgb_draw_line_raw(118, 3, 124, 3);
        w3dcgb_draw_line_raw(118, 6, 124, 6);
    }
    else
    {
        w3dcgb_draw_line_raw(118, 3, 124, 6);
        w3dcgb_draw_line_raw(118, 6, 124, 3);
    }
}

#pragma bank 2
// Sort at most eight objects near-to-far by camera-space origin depth,
// without changing the caller array. Clear the mask, draw visible models
// against earlier coverage, then add their faces as occluders. Model arrays
// and face indices must be valid. Color zero inherits the current color,
// possibly from the preceding object. Restore entry color and disable
// occlusion at the end. This model path targets the normal 128x96 stage.
void Wire3DCGB_DrawScene(Wire3DCGB_Object* objects, w3dcgb_u8 count)
{
    w3dcgb_u8 scene_count;
    w3dcgb_u8 i;
    w3dcgb_u8 old_color;

    if (objects == 0) return;

    scene_count = count;
    if (scene_count > WIRE3DCGB_SCENE_OBJECT_LIMIT) scene_count = WIRE3DCGB_SCENE_OBJECT_LIMIT;

    i = 0;
    while (i < scene_count)
    {
        w3dcgb_scene_order[(__safe_index w3dcgb_u8)i] = i;
        w3dcgb_scene_depth[(__safe_index w3dcgb_u8)i] = w3dcgb_scene_object_depth(&objects[(__safe_index w3dcgb_u8)i]);
        i = (w3dcgb_u8)(i + 1);
    }

    i = 0;
    while (i < scene_count)
    {
        w3dcgb_u8 j;
        j = (w3dcgb_u8)(i + 1);
        while (j < scene_count)
        {
            w3dcgb_u8 oi;
            w3dcgb_u8 oj;
            oi = w3dcgb_scene_order[(__safe_index w3dcgb_u8)i];
            oj = w3dcgb_scene_order[(__safe_index w3dcgb_u8)j];

            if (w3dcgb_scene_depth[(__safe_index w3dcgb_u8)oj] < w3dcgb_scene_depth[(__safe_index w3dcgb_u8)oi])
            {
                w3dcgb_scene_order[(__safe_index w3dcgb_u8)i] = oj;
                w3dcgb_scene_order[(__safe_index w3dcgb_u8)j] = oi;
            }
            j = (w3dcgb_u8)(j + 1);
        }
        i = (w3dcgb_u8)(i + 1);
    }

    old_color = w3dcgb_line_color;
    w3dcgb_clear_occlusion_mask();
    w3dcgb_occlusion_active = 0;

    i = 0;
    while (i < scene_count)
    {
        Wire3DCGB_Object* obj;
        obj = &objects[(__safe_index w3dcgb_u8)w3dcgb_scene_order[(__safe_index w3dcgb_u8)i]];
        if ((obj->visible != 0) && (obj->model != 0))
        {
            if (i == 0) w3dcgb_occlusion_active = 0;
            else w3dcgb_occlusion_active = 1;
            if (obj->color != 0) Wire3DCGB_SetLineColor(obj->color);
            Wire3DCGB_DrawModelScaled(obj->model, obj->x, obj->y, obj->z, (w3dcgb_i8)obj->rx, (w3dcgb_i8)obj->ry, (w3dcgb_i8)obj->rz, obj->scale_q8);
            w3dcgb_occlusion_active = 0;
            w3dcgb_mark_model_occluder(obj->model);
        }
        i = (w3dcgb_u8)(i + 1);
    }

    w3dcgb_occlusion_active = 0;
    w3dcgb_line_color = old_color;
}
#endif

#pragma bank 4
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Project both endpoints through the camera into the 128x96 viewport.
// Reject the whole line if either endpoint fails the depth test; do not clip
// near-plane crossings. DrawLine2D dispatches by mode, but projection remains
// 128x96. Normal lines accumulate color planes and do not mark sparse bounds;
// use a full upload or explicitly invalidate frame history for sparse upload.
void Wire3DCGB_DrawLine3D(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz)
{
    w3dcgb_u8 sx0;
    w3dcgb_u8 sy0;
    w3dcgb_u8 sx1;
    w3dcgb_u8 sy1;

    if (w3dcgb_project_world(ax, ay, az, &sx0, &sy0) == 0) return;
    if (w3dcgb_project_world(bx, by, bz, &sx1, &sy1) == 0) return;
    Wire3DCGB_DrawLine2D(sx0, sy0, sx1, sy1);
}

// Temporarily select color & 3 for DrawLine3D, then restore the old color.
void Wire3DCGB_DrawLine3DColor(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz, w3dcgb_u8 color)
{
    w3dcgb_u8 old_color;
    old_color = w3dcgb_line_color;
    Wire3DCGB_SetLineColor(color);
    Wire3DCGB_DrawLine3D(ax, ay, az, bx, by, bz);
    w3dcgb_line_color = old_color;
}

// Draw a model at unity Q8 scale (256), using DrawModelScaled contracts.
void Wire3DCGB_DrawModel(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz)
{
    Wire3DCGB_DrawModelScaled(model, x, y, z, rx, ry, rz, 256);
}

// Draw a model at unity Q8 scale with a temporary color, restoring it afterward.
void Wire3DCGB_DrawModelColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_u8 color)
{
    Wire3DCGB_DrawModelScaledColor(model, x, y, z, rx, ry, rz, 256, color);
}

#pragma bank 1
// For a hidden-line model with vertices/faces, transform at most 24 vertices
// and erase front-facing triangles among at most 16 faces. Nonpositive scale
// means unity; this path still multiplies at unity and runs all three rotation
// helpers at zero angles. Reject invalid/depth-failed faces. Replace the shared
// projection cache and use the normal C stage clearer, which requires the
// caller or allocation to provide the correct WRAM mapping.
void Wire3DCGB_EraseModelFaces(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8)
{
    w3dcgb_u8 i;
    w3dcgb_u8 count;
    w3dcgb_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3DCGB_MODEL_HIDDEN_LINES) == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3DCGB_MODEL_VERTEX_LIMIT) count = WIRE3DCGB_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const Wire3DCGB_Vec3* v;
        w3dcgb_i16 vx;
        w3dcgb_i16 vy;
        w3dcgb_i16 vz;
        w3dcgb_u8 sx;
        w3dcgb_u8 sy;

        v = &model->vertices[(__safe_index w3dcgb_u8)i];
        vx = (w3dcgb_i16)((v->x * scale_q8) >> 8);
        vy = (w3dcgb_i16)((v->y * scale_q8) >> 8);
        vz = (w3dcgb_i16)((v->z * scale_q8) >> 8);

        w3dcgb_rotate_y(&vx, &vz, (w3dcgb_i8)ry);
        w3dcgb_rotate_x(&vy, &vz, (w3dcgb_i8)rx);
        w3dcgb_rotate_z(&vx, &vy, (w3dcgb_i8)rz);

        vx = (w3dcgb_i16)(vx + x);
        vy = (w3dcgb_i16)(vy + y);
        vz = (w3dcgb_i16)(vz + z);

        if (w3dcgb_project_world(vx, vy, vz, &sx, &sy))
        {
            w3dcgb_screen_x[(__safe_index w3dcgb_u8)i] = sx;
            w3dcgb_screen_y[(__safe_index w3dcgb_u8)i] = sy;
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)i] = 1;
        }
        else
        {
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)i] = 0;
        }
        i = (w3dcgb_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3DCGB_MODEL_FACE_LIMIT) face_count = WIRE3DCGB_MODEL_FACE_LIMIT;
    w3dcgb_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3DCGB_Face* f;
        f = &model->faces[(__safe_index w3dcgb_u8)i];
        if (w3dcgb_face_visible[(__safe_index w3dcgb_u8)i] &&
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->a] &&
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->b] &&
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)f->c])
        {
            w3dcgb_clear_triangle(
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->a],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->a],
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->b],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->b],
                w3dcgb_screen_x[(__safe_index w3dcgb_u8)f->c],
                w3dcgb_screen_y[(__safe_index w3dcgb_u8)f->c]);
        }
        i = (w3dcgb_u8)(i + 1);
    }
}

#pragma bank 4
// Erase a triangle through the normal C stage clearer. Its clipped spans
// mark touched tiles dirty; caller/allocation must provide the stage WRAM mapping.
void Wire3DCGB_EraseTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy)
{
    w3dcgb_clear_triangle(ax, ay, bx, by, cx, cy);
}

#endif

// Order X endpoints, reject an off-screen row or left edge, and clamp the
// right edge to 127. Mark the range dirty and use the ASM span clearer, which
// selects and restores WRAM bank 2.
void Wire3DCGB_EraseSpan2D(w3dcgb_u8 y, w3dcgb_u8 x0, w3dcgb_u8 x1)
{
    w3dcgb_u8 tx;

    if (x0 > x1)
    {
        tx = x0;
        x0 = x1;
        x1 = tx;
    }
    if ((y >= WIRE3DCGB_SCREEN_H) || (x0 >= WIRE3DCGB_SCREEN_W)) return;
    if (x1 >= WIRE3DCGB_SCREEN_W) x1 = WIRE3DCGB_SCREEN_W - 1;
    w3dcgb_mark_dirty_rect(x0, y, x1, y);
    w3dcgb_mask_span_y = y;
    w3dcgb_mask_span_min_x = x0;
    w3dcgb_mask_span_max_x = x1;
    w3dcgb_stage_clear_span_asm();
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Order and clip the rectangle to 128x96. Full-height left strips of 16 or
// 24 pixels use bank-preserving ASM clears and dirty marking. Other rectangles
// use C row clears and require the correct stage WRAM mapping from the caller.
void Wire3DCGB_EraseRect2D(w3dcgb_u8 x0, w3dcgb_u8 y0, w3dcgb_u8 x1, w3dcgb_u8 y1)
{
    w3dcgb_u8 t;
    w3dcgb_u8 y;

    if (x0 > x1)
    {
        t = x0;
        x0 = x1;
        x1 = t;
    }
    if (y0 > y1)
    {
        t = y0;
        y0 = y1;
        y1 = t;
    }
    if (x0 >= WIRE3DCGB_SCREEN_W) return;
    if (y0 >= WIRE3DCGB_SCREEN_H) return;
    if (x1 >= WIRE3DCGB_SCREEN_W) x1 = (w3dcgb_u8)(WIRE3DCGB_SCREEN_W - 1);
    if (y1 >= WIRE3DCGB_SCREEN_H) y1 = (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1);

    if ((x0 == 0) &&
        (x1 == 15) &&
        (y0 == 0) &&
        (y1 == (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1)))
    {
        w3dcgb_erase_left_guard16_asm();
        w3dcgb_mark_dirty_rect(0, 0, 15, (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1));
        return;
    }
    if ((x0 == 0) &&
        (x1 == 23) &&
        (y0 == 0) &&
        (y1 == (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1)))
    {
        w3dcgb_erase_left_guard24_asm();
        w3dcgb_mark_dirty_rect(0, 0, 23, (w3dcgb_u8)(WIRE3DCGB_SCREEN_H - 1));
        return;
    }

    y = y0;
    while (y <= y1)
    {
        w3dcgb_stage_clear_span(y, x0, x1);
        if (y == y1) break;
        y = (w3dcgb_u8)(y + 1);
    }
}
#endif

#pragma fixed_bank 2
// Clear the leftmost 24 stage columns through bank-preserving ASM. No dirty
// range is marked; sparse-transfer callers must include the cleared tiles.
void Wire3DCGB_EraseLeftGuard24Fast()
{
    w3dcgb_erase_left_guard24_asm();
}

// Save/select/restore WRAM bank 2 while replacing the outermost stage
// pixels with two-bit color 2. Masked writes preserve interior pixels.
// This does not change the palette or dirty history.
void w3dcgb_draw_white_border_asm()
{
    __asm {
        LDH_A_MEM 112
        PUSH_AF
        LD_A_IMM 2
        LDH_MEM_A 112
        LD_HL_IMM w3dcgb_stage
        LD_B_IMM 12

w3dbw_left_tile:
        LD_C_IMM 8
w3dbw_left_row:
        LD_A_HL
        AND_IMM 0x7F
        LDI_HL_A
        LD_A_HL
        LD_D_A
        LD_A_IMM 0x80
        OR_D
        LDI_HL_A
        DEC_C
        JR_NZ w3dbw_left_row
        DEC_B
        JR_NZ w3dbw_left_tile

        LD_HL_IMM 0xDE40
        LD_B_IMM 12

w3dbw_right_tile:
        LD_C_IMM 8
w3dbw_right_row:
        LD_A_HL
        AND_IMM 0xFE
        LDI_HL_A
        LD_A_HL
        LD_D_A
        LD_A_IMM 0x01
        OR_D
        LDI_HL_A
        DEC_C
        JR_NZ w3dbw_right_row
        DEC_B
        JR_NZ w3dbw_right_tile

        LD_HL_IMM w3dcgb_stage
        LD_DE_IMM 0x00C0
        LD_B_IMM 16

w3dbw_top_tile:
        XOR_A
        LDI_HL_A
        LD_A_IMM 0xFF
        LD_HL_A
        DEC_HL
        ADD_HL_DE
        DEC_B
        JR_NZ w3dbw_top_tile

        LD_HL_IMM 0xD3BE
        LD_DE_IMM 0x00C0
        LD_B_IMM 16

w3dbw_bottom_tile:
        XOR_A
        LDI_HL_A
        LD_A_IMM 0xFF
        LD_HL_A
        DEC_HL
        ADD_HL_DE
        DEC_B
        JR_NZ w3dbw_bottom_tile
        POP_AF
        LDH_MEM_A 112
        RET
    }
}

// Write a one-pixel color-2 border around the normal stage, preserving
// interior pixels and WRAM bank selection. Its visible color depends on the
// palette. No dirty range is marked; sparse callers must arrange its upload.
void Wire3DCGB_DrawWhiteBorderFast()
{
    w3dcgb_draw_white_border_asm();
}
#pragma fixed_bank -1

#pragma bank 4
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Submit projected byte endpoints directly to the normal 128x96 rasterizer.
// Model drawing does not dispatch to the full-screen rasterizer here.
static void w3dcgb_draw_model_line(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by)
{
    w3dcgb_line_x0 = ax;
    w3dcgb_line_y0 = ay;
    w3dcgb_line_x1 = bx;
    w3dcgb_line_y1 = by;
    w3dcgb_line_stage_asm();
}

// Project up to 24 vertices with positive Q8 scale; nonpositive scale means
// unity. Skip unity multiplication and zero-angle rotations. Intermediates
// must fit s16. Build up to 16 front-face flags when adjacency filtering is
// available, then draw depth-valid edges with valid vertex indices. The model
// edge count is not capped at 64. There is no near-plane clipping. Replace
// the shared projection cache and OR line colors into the normal 128x96 stage.
// This path does not mark sparse bounds; arrange a full upload or explicitly
// invalidate frame history before a sparse upload.
void Wire3DCGB_DrawModelScaled(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8)
{
    w3dcgb_u8 i;
    w3dcgb_u8 count;
    w3dcgb_u8 edge_count;
    w3dcgb_u8 face_count;
    w3dcgb_u8 camera_unrotated;
    w3dcgb_u8 unit_scale;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->edges == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3DCGB_MODEL_VERTEX_LIMIT) count = WIRE3DCGB_MODEL_VERTEX_LIMIT;
    unit_scale = 0;
    if (scale_q8 == 256) unit_scale = 1;
    camera_unrotated = 0;
    if ((w3dcgb_cam_yaw == 0) && (w3dcgb_cam_pitch == 0) && (w3dcgb_cam_roll == 0))
    {
        camera_unrotated = 1;
    }

    i = 0;
    while (i < count)
    {
        const Wire3DCGB_Vec3* v;
        w3dcgb_i16 vx;
        w3dcgb_i16 vy;
        w3dcgb_i16 vz;
        w3dcgb_u8 sx;
        w3dcgb_u8 sy;
        w3dcgb_u8 projected;

        v = &model->vertices[(__safe_index w3dcgb_u8)i];
        if (unit_scale != 0)
        {
            vx = v->x;
            vy = v->y;
            vz = v->z;
        }
        else
        {
            vx = (w3dcgb_i16)((v->x * scale_q8) >> 8);
            vy = (w3dcgb_i16)((v->y * scale_q8) >> 8);
            vz = (w3dcgb_i16)((v->z * scale_q8) >> 8);
        }

        if (ry != 0) w3dcgb_rotate_y(&vx, &vz, (w3dcgb_i8)ry);
        if (rx != 0) w3dcgb_rotate_x(&vy, &vz, (w3dcgb_i8)rx);
        if (rz != 0) w3dcgb_rotate_z(&vx, &vy, (w3dcgb_i8)rz);

        vx = (w3dcgb_i16)(vx + x);
        vy = (w3dcgb_i16)(vy + y);
        vz = (w3dcgb_i16)(vz + z);

        if (camera_unrotated != 0)
        {
            vx = (w3dcgb_i16)(vx - w3dcgb_cam_x);
            vy = (w3dcgb_i16)(vy - w3dcgb_cam_y);
            vz = (w3dcgb_i16)(vz - w3dcgb_cam_z);
            projected = w3dcgb_project_camera_space(vx, vy, vz, &sx, &sy);
        }
        else
        {
            projected = w3dcgb_project_world(vx, vy, vz, &sx, &sy);
        }

        if (projected != 0)
        {
            w3dcgb_screen_x[(__safe_index w3dcgb_u8)i] = sx;
            w3dcgb_screen_y[(__safe_index w3dcgb_u8)i] = sy;
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)i] = 1;
        }
        else
        {
            w3dcgb_screen_visible[(__safe_index w3dcgb_u8)i] = 0;
        }
        i = (w3dcgb_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3DCGB_MODEL_FACE_LIMIT) face_count = WIRE3DCGB_MODEL_FACE_LIMIT;
    if ((model->flags & WIRE3DCGB_MODEL_HIDDEN_LINES) && (model->faces != 0) && (model->edge_faces != 0))
    {
        w3dcgb_build_face_visibility(model, count, face_count);
    }

    edge_count = model->edge_count;
    i = 0;
    while (i < edge_count)
    {
        const Wire3DCGB_Edge* e;
        e = &model->edges[(__safe_index w3dcgb_u8)i];
        if ((e->a < count) && (e->b < count))
        {
            if (w3dcgb_screen_visible[(__safe_index w3dcgb_u8)e->a] &&
                w3dcgb_screen_visible[(__safe_index w3dcgb_u8)e->b] &&
                w3dcgb_is_edge_visible(model, i, face_count))
            {
                w3dcgb_draw_model_line(
                    w3dcgb_screen_x[(__safe_index w3dcgb_u8)e->a],
                    w3dcgb_screen_y[(__safe_index w3dcgb_u8)e->a],
                    w3dcgb_screen_x[(__safe_index w3dcgb_u8)e->b],
                    w3dcgb_screen_y[(__safe_index w3dcgb_u8)e->b]);
            }
        }
        i = (w3dcgb_u8)(i + 1);
    }
}

#pragma bank 4
// Select color & 3 around DrawModelScaled and restore entry color afterward.
void Wire3DCGB_DrawModelScaledColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8, w3dcgb_u8 color)
{
    w3dcgb_u8 old_color;
    old_color = w3dcgb_line_color;
    Wire3DCGB_SetLineColor(color);
    Wire3DCGB_DrawModelScaled(model, x, y, z, rx, ry, rz, scale_q8);
    w3dcgb_line_color = old_color;
}
#endif

#pragma bank 4
/* Exact signed a*b/c without an overflowing 16-bit intermediate. Clip
   intersections always have abs(b) <= abs(c). Endpoints are limited to 2047. */
// Compute signed a*b/c with truncation toward zero; return zero for c=0.
// Small operands use direct multiplication; larger ones use a 16-step
// quotient/remainder loop without forming the full product. This helper is
// for the bounded clipping coordinates, not arbitrary s16 arithmetic: the
// absolute values, intermediate remainders and signed result must fit.
static w3dcgb_i16 w3dcgb_clip_muldiv(w3dcgb_i16 a, w3dcgb_i16 b, w3dcgb_i16 c)
{
    w3dcgb_u16 quotient;
    w3dcgb_u16 remainder;
    w3dcgb_u16 bit;
    w3dcgb_u8 negative;
    negative = 0;
    if (a < 0) { a = (w3dcgb_i16)(0 - a); negative = 1; }
    if (b < 0) { b = (w3dcgb_i16)(0 - b); negative = (w3dcgb_u8)(negative ^ 1); }
    if (c < 0) { c = (w3dcgb_i16)(0 - c); negative = (w3dcgb_u8)(negative ^ 1); }
    if (c == 0) return 0;
    if (((a <= 255) && (b <= 127)) || ((b <= 255) && (a <= 127)))
    {
        quotient = (w3dcgb_u16)((a * b) / c);
        if (negative != 0) return (w3dcgb_i16)(0 - quotient);
        return (w3dcgb_i16)quotient;
    }
    quotient = 0;
    remainder = 0;
    bit = 0x8000;
    while (bit != 0)
    {
        quotient = (w3dcgb_u16)(quotient << 1);
        remainder = (w3dcgb_u16)(remainder << 1);
        if (((w3dcgb_u16)a & bit) != 0) remainder = (w3dcgb_u16)(remainder + b);
        while (remainder >= (w3dcgb_u16)c)
        {
            remainder = (w3dcgb_u16)(remainder - c);
            quotient = (w3dcgb_u16)(quotient + 1);
        }
        bit = (w3dcgb_u16)(bit >> 1);
    }
    if (negative != 0) return (w3dcgb_i16)(0 - quotient);
    return (w3dcgb_i16)quotient;
}

w3dcgb_u8 w3dcgb_projection_magnitude;
w3dcgb_u8 w3dcgb_projection_depth;
w3dcgb_i16 w3dcgb_projection_result;
w3dcgb_i16 w3dcgb_projection_value;
w3dcgb_i16 w3dcgb_projection_z;

// Multiply the shared unsigned byte magnitude by 48, then divide the
// 16-bit product by the nonzero byte depth with 16 shift/subtract steps.
// Store an unsigned quotient; the caller applies its sign.
void w3dcgb_project48_asm()
{
    __asm {
        LD_A_MEM w3dcgb_projection_magnitude
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_D_H
        LD_E_L
        ADD_HL_HL
        ADD_HL_DE
        LD_A_MEM w3dcgb_projection_depth
        LD_D_A
        LD_E_IMM 16
        XOR_A
w3dproj_div:
        ADD_HL_HL
        RLA
        JR_C w3dproj_sub
        CP_D
        JR_C w3dproj_next
w3dproj_sub:
        SUB_D
        INC_L
w3dproj_next:
        DEC_E
        JR_NZ w3dproj_div
        LD_A_L
        LD_MEM_A w3dcgb_projection_result
        LD_A_H
        LD_MEM_A w3dcgb_projection_result+1
        RET
    }
}

// Convert signed value to unsigned magnitude, clamp negative/Z<4 depth
// to 4, and jointly shift magnitude/depth until depth fits a byte. Clamp
// magnitude to 160, divide magnitude*48 by depth, then restore the sign.
// The result is an offset, with no camera, viewport or near/far rejection.
void w3dcgb_project_axis48_asm()
{
    __asm {
        LD_A_MEM w3dcgb_projection_value
        LD_L_A
        LD_A_MEM w3dcgb_projection_value+1
        LD_H_A
        LD_C_IMM 0
        AND_IMM 0x80
        JR_Z w3dpax_abs_done
        INC_C
        LD_A_L
        CPL
        ADD_A_IMM 1
        LD_L_A
        LD_A_H
        CPL
        ADC_IMM 0
        LD_H_A
w3dpax_abs_done:
        LD_A_MEM w3dcgb_projection_z
        LD_E_A
        LD_A_MEM w3dcgb_projection_z+1
        LD_D_A
        AND_IMM 0x80
        JR_NZ w3dpax_near
        LD_A_D
        OR_A
        JR_NZ w3dpax_normalize
        LD_A_E
        CP_IMM 4
        JR_NC w3dpax_clamp
w3dpax_near:
        LD_D_IMM 0
        LD_E_IMM 4
        JR w3dpax_clamp
w3dpax_normalize:
        LD_A_D
        OR_A
        RRA
        LD_D_A
        LD_A_E
        RRA
        LD_E_A
        LD_A_H
        OR_A
        RRA
        LD_H_A
        LD_A_L
        RRA
        LD_L_A
        LD_A_D
        OR_A
        JR_NZ w3dpax_normalize
w3dpax_clamp:
        LD_A_E
        LD_MEM_A w3dcgb_projection_depth
        LD_A_H
        OR_A
        JR_NZ w3dpax_limit
        LD_A_L
        CP_IMM 161
        JR_C w3dpax_magnitude
w3dpax_limit:
        LD_A_IMM 160
w3dpax_magnitude:
        LD_MEM_A w3dcgb_projection_magnitude
        CALL w3dcgb_project48_asm
        LD_A_C
        OR_A
        RET_Z
        LD_A_MEM w3dcgb_projection_result
        CPL
        ADD_A_IMM 1
        LD_MEM_A w3dcgb_projection_result
        LD_A_MEM w3dcgb_projection_result+1
        CPL
        ADC_IMM 0
        LD_MEM_A w3dcgb_projection_result+1
        RET
    }
}

// Approximate value*48/z without a camera or screen-center offset. Negative
// or very small Z becomes 4. For Z above 255, shift depth and magnitude
// right together until depth fits a byte, then cap magnitude at 160.
// Return a signed offset; this helper does not reject near/far depths.
w3dcgb_i16 Wire3DCGB_ProjectAxis48(w3dcgb_i16 value, w3dcgb_i16 z)
{
    w3dcgb_projection_value = value;
    w3dcgb_projection_z = z;
    w3dcgb_project_axis48_asm();
    return w3dcgb_projection_result;
}

// Mark current and both previous sparse ranges as all 192 normal-mode
// tiles. Pixel data is not cleared; subsequent uploads cannot trust old ranges.
void Wire3DCGB_InvalidateFrameHistory()
{
    w3dcgb_dirty_min_tile = 0;
    w3dcgb_dirty_max_tile = 191;
    w3dcgb_prev_dirty_min_tile = 0;
    w3dcgb_prev_dirty_max_tile = 191;
    w3dcgb_older_dirty_min_tile = 0;
    w3dcgb_older_dirty_max_tile = 191;
}

// Return left/right/top/bottom bits (1/2/4/8) for signed coordinates,
// using 128x96 or 160x144 bounds according to the active mode.
static w3dcgb_u8 w3dcgb_clip_outcode(w3dcgb_i16 x, w3dcgb_i16 y)
{
    w3dcgb_u8 code;
    w3dcgb_i16 width;
    w3dcgb_i16 height;
    width = WIRE3DCGB_SCREEN_W;
    height = WIRE3DCGB_SCREEN_H;
    if (w3dcgb_full_mode != 0) { width = WIRE3DCGB_FULL_SCREEN_W; height = WIRE3DCGB_FULL_SCREEN_H; }
    code = 0;
    if (x < 0) code = 1;
    else if (x >= width) code = 2;
    if (y < 0) code = (w3dcgb_u8)(code | 4);
    else if (y >= height) code = (w3dcgb_u8)(code | 8);
    return code;
}

// Clip a signed 2D line to the selected viewport, then draw with temporary
// color. Inputs outside +/-2047 are rejected. Fully in-bounds 128x96 lines
// use a fast path; other lines use outcodes and bounded integer intersections
// with an eight-iteration guard. Accepted normal-mode lines mark dirty bounds.
// This is screen clipping only, with no 3D projection or depth clipping.
void Wire3DCGB_DrawLineClipped2D(w3dcgb_i16 x0, w3dcgb_i16 y0,
                               w3dcgb_i16 x1, w3dcgb_i16 y1, w3dcgb_u8 color)
{
    w3dcgb_u8 c0;
    w3dcgb_u8 c1;
    w3dcgb_u8 outside;
    w3dcgb_u8 guard;
    w3dcgb_i16 x;
    w3dcgb_i16 y;
    c0 = 0;
    c1 = 0;
    if ((((x0 | y0 | x1 | y1) & 0xFF80) != 0) || (y0 >= 96) || (y1 >= 96))
    {
        if ((x0 < -2047) || (x0 > 2047) || (y0 < -2047) || (y0 > 2047) ||
            (x1 < -2047) || (x1 > 2047) || (y1 < -2047) || (y1 > 2047)) return;
        c0 = w3dcgb_clip_outcode(x0, y0);
        c1 = w3dcgb_clip_outcode(x1, y1);
    }
    guard = 0;
    while (guard < 8)
    {
        if ((c0 | c1) == 0)
        {
            /* The existing ASM rasterizer handles the hot, in-window path. */
            if (w3dcgb_full_mode == 0) w3dcgb_mark_dirty_line((w3dcgb_u8)x0, (w3dcgb_u8)y0,
                                  (w3dcgb_u8)x1, (w3dcgb_u8)y1);
            Wire3DCGB_DrawLine2DColor((w3dcgb_u8)x0, (w3dcgb_u8)y0,
                                    (w3dcgb_u8)x1, (w3dcgb_u8)y1, color);
            return;
        }
        if ((c0 & c1) != 0) return;
        outside = c0;
        if (outside == 0) outside = c1;
        if ((outside & 12) != 0)
        {
            y = 0;
            if ((outside & 8) != 0) y = WIRE3DCGB_SCREEN_H - 1;
            if (((outside & 8) != 0) && (w3dcgb_full_mode != 0)) y = WIRE3DCGB_FULL_SCREEN_H - 1;
            x = (w3dcgb_i16)(x0 + w3dcgb_clip_muldiv((w3dcgb_i16)(x1 - x0),
                (w3dcgb_i16)(y - y0), (w3dcgb_i16)(y1 - y0)));
        }
        else
        {
            x = 0;
            if ((outside & 2) != 0) x = WIRE3DCGB_SCREEN_W - 1;
            if (((outside & 2) != 0) && (w3dcgb_full_mode != 0)) x = WIRE3DCGB_FULL_SCREEN_W - 1;
            y = (w3dcgb_i16)(y0 + w3dcgb_clip_muldiv((w3dcgb_i16)(y1 - y0),
                (w3dcgb_i16)(x - x0), (w3dcgb_i16)(x1 - x0)));
        }
        if (outside == c0) { x0 = x; y0 = y; c0 = w3dcgb_clip_outcode(x0, y0); }
        else { x1 = x; y1 = y; c1 = w3dcgb_clip_outcode(x1, y1); }
        guard = (w3dcgb_u8)(guard + 1);
    }
}

// Validate pointers, 1..24 vertices, at most 64 edges, every index and
// input coordinates/centers in +/-2047 before drawing. Fully visible geometry
// uses signed-byte scratch and the fast indexed path; other edges use the
// signed line clipper. That clipper may reject translated endpoints beyond
// +/-2047 even when individual inputs passed. Replace shared screen scratch
// and preserve the previous color through the called line APIs.
void Wire3DCGB_DrawEdgeListClipped2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
    const w3dcgb_i16* vx, const w3dcgb_i16* vy, w3dcgb_u8 vertex_count,
    w3dcgb_i16 cx, w3dcgb_i16 cy, w3dcgb_u8 color)
{
    w3dcgb_u8 i;
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 inside;
    w3dcgb_i16 x;
    w3dcgb_i16 y;
    w3dcgb_i16 min_x;
    w3dcgb_i16 min_y;
    w3dcgb_i16 max_x;
    w3dcgb_i16 max_y;
    if ((edges == 0) || (vx == 0) || (vy == 0)) return;
    if ((cx < -2047) || (cx > 2047) || (cy < -2047) || (cy > 2047)) return;
    if ((vertex_count == 0) || (vertex_count > WIRE3DCGB_MODEL_VERTEX_LIMIT) ||
        (edge_count > WIRE3DCGB_MODEL_EDGE_LIMIT)) return;
    i = 0;
    while (i < edge_count)
    {
        if ((edges[(__safe_index w3dcgb_u8)i].a >= vertex_count) ||
            (edges[(__safe_index w3dcgb_u8)i].b >= vertex_count)) return;
        i = (w3dcgb_u8)(i + 1);
    }
    min_x = 32767;
    min_y = 32767;
    max_x = -32767;
    max_y = -32767;
    inside = 1;
    i = 0;
    while (i < vertex_count)
    {
        if ((vx[(__safe_index w3dcgb_u8)i] < -2047) ||
            (vx[(__safe_index w3dcgb_u8)i] > 2047) ||
            (vy[(__safe_index w3dcgb_u8)i] < -2047) ||
            (vy[(__safe_index w3dcgb_u8)i] > 2047)) return;
        x = (w3dcgb_i16)(cx + vx[(__safe_index w3dcgb_u8)i]);
        y = (w3dcgb_i16)(cy + vy[(__safe_index w3dcgb_u8)i]);
        if (x < min_x) min_x = x;
        if (x > max_x) max_x = x;
        if (y < min_y) min_y = y;
        if (y > max_y) max_y = y;
        /* Reuse projected-vertex scratch; drawing is non-reentrant. */
        w3dcgb_screen_x[(__safe_index w3dcgb_u8)i] = (w3dcgb_u8)(x - 64);
        w3dcgb_screen_y[(__safe_index w3dcgb_u8)i] = (w3dcgb_u8)(y - 48);
        i = (w3dcgb_u8)(i + 1);
    }
    if ((w3dcgb_clip_outcode(min_x, min_y) | w3dcgb_clip_outcode(max_x, max_y)) != 0) inside = 0;
    if (inside != 0)
    {
        if (w3dcgb_full_mode == 0) w3dcgb_mark_dirty_rect((w3dcgb_u8)min_x,
            (w3dcgb_u8)min_y, (w3dcgb_u8)max_x, (w3dcgb_u8)max_y);
        Wire3DCGB_DrawEdgeList2D(edges, edge_count, (const w3dcgb_i8*)w3dcgb_screen_x,
            (const w3dcgb_i8*)w3dcgb_screen_y, 64, 48, color);
        return;
    }
    i = 0;
    while (i < edge_count)
    {
        a = edges[(__safe_index w3dcgb_u8)i].a;
        b = edges[(__safe_index w3dcgb_u8)i].b;
        Wire3DCGB_DrawLineClipped2D((w3dcgb_i16)(cx + vx[(__safe_index w3dcgb_u8)a]),
            (w3dcgb_i16)(cy + vy[(__safe_index w3dcgb_u8)a]),
            (w3dcgb_i16)(cx + vx[(__safe_index w3dcgb_u8)b]),
            (w3dcgb_i16)(cy + vy[(__safe_index w3dcgb_u8)b]), color);
        i = (w3dcgb_u8)(i + 1);
    }
}

#pragma bank 1
// Normal mode flushes queued BG writes, transfers the full stage and
// presents the completed tile bank. Full-screen mode transfers/presents its
// allocated tiles and does not flush the HUD queue. Requires an enabled LCD
// and the transfer helpers' bank/interrupt contracts; may span multiple frames.
void Wire3DCGB_EndFrame()
{
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_transfer_asm();
        w3dcgb_full_present_asm();
        return;
    }
    if (w3dcgb_bgq_count != 0)
    {
        w3dcgb_wait_vblank_start();
        w3dcgb_flush_bg_queue();
    }
    w3dcgb_transfer_stage_asm();
    w3dcgb_present_stage_asm();
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Transfer and present the complete selected-mode frame without flushing
// the BG queue. It does not consume queued HUD writes. Requires the same
// LCD/bank/interrupt conditions as EndFrame; full-runtime build required.
void Wire3DCGB_EndFrameFast()
{
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_transfer_asm();
        w3dcgb_full_present_asm();
        return;
    }
    w3dcgb_transfer_stage_asm();
    w3dcgb_present_stage_asm();
}

// In normal mode, wait for a fresh VBlank and invoke EndFrameSparseNow.
// Full-screen mode uses its full transfer/presentation instead. Requires the
// full runtime and an enabled LCD; it is not guaranteed to complete in one frame.
void Wire3DCGB_EndFrameSparse()
{
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_transfer_asm();
        w3dcgb_full_present_asm();
        return;
    }
    w3dcgb_wait_vblank_start();
    Wire3DCGB_EndFrameSparseNow();
}
#endif

// Flush queued BG writes, upload the sparse current/N-2 tile range to the
// back bank and present it. Skip the initial fresh-VBlank wait, but normal
// map mode still waits when restoring its blank HUD tile; DMA/presentation
// may also wait. Full-runtime full-screen mode delegates to its own path.
// Require an enabled LCD and keep SVBK/VBK stable while DMA is active.
void Wire3DCGB_EndFrameSparseNow()
{
#ifndef WIRE3DCGB_MINIMAL_RUNTIME
    if (w3dcgb_full_mode != 0)
    {
        w3dcgb_full_transfer_asm();
        w3dcgb_full_present_asm();
        return;
    }
#endif
    /* Atomic mode never writes a displayed tile and does not need a
       frame-long VBlank wait before starting an off-screen upload. */
    if (w3dcgb_atomic_maps == 0) w3dcgb_restore_hud_blank_tile_asm();
    if (w3dcgb_bgq_count != 0) w3dcgb_flush_bg_queue();
    w3dcgb_transfer_dirty_tiles();
    w3dcgb_present_stage_asm();
}

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
// Return the current full-screen allocation count (up to 127). This is
// not a byte count or the number of visible/nonzero pixels. Full runtime only.
w3dcgb_u8 Wire3DCGB_GetFullScreenTileCount()
{
    return w3dcgb_full_tile_count;
}

// Return the full-screen frame fault flag: the fast line path sets it for
// tile allocation exhaustion or an invalid coordinate. Once set, further
// fast-path plots are suppressed until BeginFrame resets it. This is not a
// general hardware status flag; the getter requires the full runtime.
w3dcgb_u8 Wire3DCGB_GetFullScreenOverflow()
{
    return w3dcgb_full_overflow;
}
#endif

#ifndef WIRE3DCGB_MINIMAL_RUNTIME
#pragma bank 1
// Use two immediate 24-block GDMA transfers to copy 384 tile numbers
// D300->9800 in VBK zero and 384 attributes D500->9800 in VBK one. Leave
// VBK zero. No LCD wait, SVBK selection or interrupt protection occurs here;
// the caller supplies a safe transfer interval and stable source-bank mapping.
void w3dcgb_fast_map_flush_gdma_asm()
{
    __asm {
        XOR_A
        LDH_MEM_A 79
        LD_A_IMM 0xD3
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_IMM 0x98
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        LD_A_IMM 0x17
        LDH_MEM_A 85

        LD_A_IMM 1
        LDH_MEM_A 79
        LD_A_IMM 0xD5
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_IMM 0x98
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        LD_A_IMM 0x17
        LDH_MEM_A 85
        XOR_A
        LDH_MEM_A 79
        RET
    }
}

// Copy 384 tile numbers D300->9800 with immediate GDMA in VBK zero, leaving
// attributes unchanged. The caller supplies a safe transfer interval and
// WRAM mapping; this helper neither waits for VBlank nor selects SVBK.
void w3dcgb_fast_map_flush_tiles_gdma_asm()
{
    __asm {
        XOR_A
        LDH_MEM_A 79
        LD_A_IMM 0xD3
        LDH_MEM_A 81
        XOR_A
        LDH_MEM_A 82
        LD_A_IMM 0x98
        LDH_MEM_A 83
        XOR_A
        LDH_MEM_A 84
        LD_A_IMM 0x17
        LDH_MEM_A 85
        RET
    }
}

// Set all 384 tile-shadow bytes to blank ID 8A and the corresponding
// attributes at offset 512 to zero. This includes all 32 columns in twelve
// map rows. C accesses use the caller/allocation WRAM mapping; no upload occurs.
static void w3dcgb_fast_map_clear_shadow()
{
    w3dcgb_u16 i;
    i = 0;
    while (i < W3DCGB_FAST_MAP_BYTES)
    {
        w3dcgb_stage[(__safe_index w3dcgb_u16)i] = W3DCGB_HUD_TILE_BLANK;
        w3dcgb_stage[(__safe_index w3dcgb_u16)(W3DCGB_FAST_ATTR_OFFSET + i)] = 0;
        i = (w3dcgb_u16)(i + 1);
    }
}

// Replace one tile/attribute pair inside the 16x12 FastMap viewport, using
// a 32-byte row stride and attribute offset 512. Reset the attribute to zero;
// ignore off-screen cells. Retain the caller/allocation WRAM mapping.
static void w3dcgb_fast_map_put(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 tile)
{
    w3dcgb_u16 off;
    if (tx >= 16) return;
    if (ty >= 12) return;
    off = (w3dcgb_u16)(((w3dcgb_u16)ty << 5) + (w3dcgb_u16)tx);
    w3dcgb_stage[(__safe_index w3dcgb_u16)off] = tile;
    w3dcgb_stage[(__safe_index w3dcgb_u16)(W3DCGB_FAST_ATTR_OFFSET + off)] = 0;
}

// Replace one in-bounds FastMap tile and its attribute byte in the shadow.
// Ignore cells outside 16x12; no upload or explicit SVBK selection occurs.
static void w3dcgb_fast_map_put_attr(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 tile, w3dcgb_u8 attr)
{
    w3dcgb_u16 off;
    if (tx >= 16) return;
    if (ty >= 12) return;
    off = (w3dcgb_u16)(((w3dcgb_u16)ty << 5) + (w3dcgb_u16)tx);
    w3dcgb_stage[(__safe_index w3dcgb_u16)off] = tile;
    w3dcgb_stage[(__safe_index w3dcgb_u16)(W3DCGB_FAST_ATTR_OFFSET + off)] = attr;
}

// Clear a nonempty tile rectangle and its attributes, clipping its far
// edges to 16x12. Callers keep width/height small enough that byte endpoint
// sums do not wrap. Reset shared Y before the second ASM pass because each
// clearer advances it. Both ASM writers require the intended WRAM mapping.
static void w3dcgb_fast_map_clear_rect(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 tw, w3dcgb_u8 th)
{
    if (tx >= 16) return;
    if (ty >= 12) return;
    if (tw == 0) return;
    if (th == 0) return;
    if ((w3dcgb_u8)(tx + tw) > 16) tw = (w3dcgb_u8)(16 - tx);
    if ((w3dcgb_u8)(ty + th) > 12) th = (w3dcgb_u8)(12 - ty);

    w3dcgb_fm_x0 = tx;
    w3dcgb_fm_y0 = ty;
    w3dcgb_fm_count_x = tw;
    w3dcgb_fm_count_y = th;
    w3dcgb_fast_map_clear_rect_asm();
    w3dcgb_fm_x0 = tx;
    w3dcgb_fm_y0 = ty;
    w3dcgb_fm_count_x = tw;
    w3dcgb_fm_count_y = th;
    w3dcgb_fast_map_clear_attr_rect_asm();
}

// OR side bits into an existing outline tile only when it has the same
// color; otherwise replace its mask. Reject off-screen cells or a zero mask.
// Color zero selects MAIN and values above 3 clamp to 3. Only four mask bits
// are stored and the attribute becomes zero. C access retains the WRAM mapping.
static void w3dcgb_fast_map_or_cell(w3dcgb_i16 tx, w3dcgb_i16 ty, w3dcgb_u8 color, w3dcgb_u8 mask)
{
    w3dcgb_u16 off;
    w3dcgb_u8 old_tile;
    w3dcgb_u8 old_rel;
    w3dcgb_u8 tile;

    if (tx < 0) return;
    if (ty < 0) return;
    if (tx >= 16) return;
    if (ty >= 12) return;
    if (mask == 0) return;
    if (color == 0) color = WIRE3DCGB_COLOR_MAIN;
    if (color > 3) color = 3;

    off = (w3dcgb_u16)(((w3dcgb_u16)ty << 5) + (w3dcgb_u16)tx);
    old_tile = w3dcgb_stage[(__safe_index w3dcgb_u16)off];
    if ((old_tile >= W3DCGB_FAST_TILE_BASE) && (old_tile < (w3dcgb_u8)(W3DCGB_FAST_TILE_BASE + 48)))
    {
        old_rel = (w3dcgb_u8)(old_tile - W3DCGB_FAST_TILE_BASE);
        if ((w3dcgb_u8)((old_rel >> 4) + 1) == color)
        {
            mask = (w3dcgb_u8)(mask | (old_rel & 15));
        }
    }

    tile = (w3dcgb_u8)(W3DCGB_FAST_TILE_BASE + ((color - 1) << 4) + (mask & 15));
    w3dcgb_stage[(__safe_index w3dcgb_u16)off] = tile;
    w3dcgb_stage[(__safe_index w3dcgb_u16)(W3DCGB_FAST_ATTR_OFFSET + off)] = 0;
}

// Mark FastMap prepared and clear tile numbers/attributes in its 16x12
// viewport. Shadow columns 16..31 are retained even though flush copies
// 32 columns per row. Requires the full runtime and a stable FastMap WRAM
// mapping; this does not initialize hardware or upload the map.
void Wire3DCGB_FastMapBegin()
{
    w3dcgb_fast_map_ready = 1;
    w3dcgb_fast_map_clear_rect(0, 0, 16, 12);
}

// Mark FastMap prepared and clear only its 16x12 tile-number viewport.
// Keep attributes and columns 16..31. Requires the full runtime and the
// intended WRAM mapping; hardware initialization/upload is separate.
void Wire3DCGB_FastMapBeginTilesOnly()
{
    w3dcgb_fast_map_ready = 1;
    w3dcgb_fm_x0 = 0;
    w3dcgb_fm_y0 = 0;
    w3dcgb_fm_count_x = 16;
    w3dcgb_fm_count_y = 12;
    w3dcgb_fast_map_clear_rect_asm();
}

// Add outline sides to a cell: bits 1/2/4/8 mean top/bottom/left/right.
// Same-color masks combine; another color replaces the old mask. Zero mask
// is ignored, zero color means MAIN, and color above 3 clamps to 3. Target
// the 16x12 shadow with stable WRAM mapping; full runtime required.
void Wire3DCGB_FastMapCell(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 color, w3dcgb_u8 mask)
{
    w3dcgb_fast_map_or_cell((w3dcgb_i16)tx, (w3dcgb_i16)ty, color, mask);
}

// Order and clip byte tile coordinates to 16x12, then replace selected
// border cells. Side bits 1/2/4/8 mean top/bottom/left/right. Zero color
// means MAIN; values above 3 clamp to 3. Attributes and interior cells remain.
// Use at least two rows/columns for distinct corners; coincident corners
// are overwritten in sequence. Requires full runtime and stable WRAM mapping.
void Wire3DCGB_FastMapRect(w3dcgb_u8 tx0, w3dcgb_u8 ty0, w3dcgb_u8 tx1, w3dcgb_u8 ty1, w3dcgb_u8 color, w3dcgb_u8 sides)
{
    w3dcgb_u8 t;

    if (tx0 > tx1)
    {
        t = tx0;
        tx0 = tx1;
        tx1 = t;
    }
    if (ty0 > ty1)
    {
        t = ty0;
        ty0 = ty1;
        ty1 = t;
    }
    if (tx0 >= 16) return;
    if (ty0 >= 12) return;
    if (tx1 >= 16) tx1 = 15;
    if (ty1 >= 12) ty1 = 11;
    if (color == 0) color = WIRE3DCGB_COLOR_MAIN;
    if (color > 3) color = 3;

    w3dcgb_fm_x0 = tx0;
    w3dcgb_fm_y0 = ty0;
    w3dcgb_fm_x1 = tx1;
    w3dcgb_fm_y1 = ty1;
    w3dcgb_fm_count_x = (w3dcgb_u8)(tx1 - tx0 + 1);
    w3dcgb_fm_count_y = (w3dcgb_u8)(ty1 - ty0 + 1);
    w3dcgb_fm_tile_base = (w3dcgb_u8)(W3DCGB_FAST_TILE_BASE + ((color - 1) << 4));
    w3dcgb_fm_sides = (w3dcgb_u8)(sides & 15);
    w3dcgb_fast_map_rect_asm();
}

// Wait for a fresh VBlank when LCD is enabled, then upload the first
// twelve complete map rows of tile numbers and attributes to map 9800.
// LCD-off callers proceed immediately. Requires the full runtime, stable
// WRAM mapping and enough safe time for both immediate GDMA transfers.
void Wire3DCGB_FastMapFlush()
{
    w3dcgb_wait_vblank_start();
    w3dcgb_fast_map_flush_gdma_asm();
}

// Wait for a fresh VBlank when LCD is enabled and upload only the first
// twelve complete rows of tile numbers to map 9800. Attributes remain.
// Requires the full runtime, stable WRAM mapping and a safe GDMA interval.
void Wire3DCGB_FastMapFlushTilesOnly()
{
    w3dcgb_wait_vblank_start();
    w3dcgb_fast_map_flush_tiles_gdma_asm();
}

// Retained, currently unused diagonal-tile selector; kind is ignored.
// Keep same-color mask bits from an outline/diagonal tile, then select its
// diagonal variant and attribute zero. This assumes those tile patterns
// were loaded; the normal FastMap initializer does not establish that contract.
static void w3dcgb_fast_map_diag_cell(w3dcgb_i16 tx, w3dcgb_i16 ty, w3dcgb_u8 color, w3dcgb_u8 kind)
{
    w3dcgb_u16 off;
    w3dcgb_u8 old_tile;
    w3dcgb_u8 old_rel;
    w3dcgb_u8 mask;
    w3dcgb_u8 tile;

    kind = kind;
    if (tx < 0) return;
    if (ty < 0) return;
    if (tx >= 16) return;
    if (ty >= 12) return;
    if (color == 0) color = WIRE3DCGB_COLOR_MAIN;
    if (color > 3) color = 3;

    off = (w3dcgb_u16)(((w3dcgb_u16)ty << 5) + (w3dcgb_u16)tx);
    mask = 0;
    old_tile = w3dcgb_stage[(__safe_index w3dcgb_u16)off];
    if ((old_tile >= W3DCGB_FAST_TILE_BASE) && (old_tile < (w3dcgb_u8)(W3DCGB_FAST_TILE_BASE + 48)))
    {
        old_rel = (w3dcgb_u8)(old_tile - W3DCGB_FAST_TILE_BASE);
        if ((w3dcgb_u8)((old_rel >> 4) + 1) == color) mask = (w3dcgb_u8)(old_rel & 15);
    }
    else if ((old_tile >= W3DCGB_FAST_DIAG_BASE) && (old_tile < (w3dcgb_u8)(W3DCGB_FAST_DIAG_BASE + 48)))
    {
        old_rel = (w3dcgb_u8)(old_tile - W3DCGB_FAST_DIAG_BASE);
        if ((w3dcgb_u8)((old_rel >> 4) + 1) == color) mask = (w3dcgb_u8)(old_rel & 15);
    }

    tile = (w3dcgb_u8)(W3DCGB_FAST_DIAG_BASE + ((color - 1) << 4) + mask);
    w3dcgb_stage[(__safe_index w3dcgb_u16)off] = tile;
    w3dcgb_stage[(__safe_index w3dcgb_u16)(W3DCGB_FAST_ATTR_OFFSET + off)] = 0;
}

// Replace requested border cells in the D300 shadow, then write corner
// combinations in top-left/top-right/bottom-left/bottom-right order. Shared
// counts must be nonzero and coordinates valid. Do not merge existing masks
// or write attributes. Coincident corners use the last write. No SVBK switch.
void w3dcgb_fast_map_rect_asm()
{
    __asm {
        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 1
        JP_Z w3dfm_skip_top
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_addr_x0
        LD_A_MEM w3dcgb_fm_count_x
        LD_B_A
        LD_A_MEM w3dcgb_fm_tile_base
        INC_A
w3dfm_top_loop:
        LDI_HL_A
        DEC_B
        JP_NZ w3dfm_top_loop
w3dfm_skip_top:

        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 2
        JP_Z w3dfm_skip_bottom
        LD_A_MEM w3dcgb_fm_y1
        CALL w3dfm_addr_x0
        LD_A_MEM w3dcgb_fm_count_x
        LD_B_A
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_A_IMM 2
w3dfm_bottom_loop:
        LDI_HL_A
        DEC_B
        JP_NZ w3dfm_bottom_loop
w3dfm_skip_bottom:

        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 4
        JP_Z w3dfm_skip_left
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_addr_x0
        LD_A_MEM w3dcgb_fm_count_y
        LD_B_A
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_A_IMM 4
        LD_D_IMM 0
        LD_E_IMM 32
w3dfm_left_loop:
        LD_HL_A
        ADD_HL_DE
        DEC_B
        JP_NZ w3dfm_left_loop
w3dfm_skip_left:

        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 8
        JP_Z w3dfm_done
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_addr_x1
        LD_A_MEM w3dcgb_fm_count_y
        LD_B_A
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_A_IMM 8
        LD_D_IMM 0
        LD_E_IMM 32
w3dfm_right_loop:
        LD_HL_A
        ADD_HL_DE
        DEC_B
        JP_NZ w3dfm_right_loop
        JP w3dfm_done

w3dfm_addr_x0:
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_x0
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD3
        LD_E_IMM 0
        ADD_HL_DE
        RET

w3dfm_addr_x1:
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_x1
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD3
        LD_E_IMM 0
        ADD_HL_DE
        RET

// Patch corner combinations after straight edges; later corners overwrite
// earlier ones when the requested width or height is one cell.
w3dfm_done:
        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 5
        JP_Z w3dfm_corner_tr
        LD_B_A
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_addr_x0
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_B
        LD_HL_A

w3dfm_corner_tr:
        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 9
        JP_Z w3dfm_corner_bl
        LD_B_A
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_addr_x1
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_B
        LD_HL_A

w3dfm_corner_bl:
        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 6
        JP_Z w3dfm_corner_br
        LD_B_A
        LD_A_MEM w3dcgb_fm_y1
        CALL w3dfm_addr_x0
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_B
        LD_HL_A

w3dfm_corner_br:
        LD_A_MEM w3dcgb_fm_sides
        AND_IMM 10
        JP_Z w3dfm_rect_ret
        LD_B_A
        LD_A_MEM w3dcgb_fm_y1
        CALL w3dfm_addr_x1
        LD_A_MEM w3dcgb_fm_tile_base
        ADD_B
        LD_HL_A

w3dfm_rect_ret:
        RET
    }
}

// Fill a validated nonempty rectangle with blank tile 8A at D300, using
// a 32-byte row stride. Advance shared fm_y0 by the row count. Zero counts
// would wrap the decrement loops; no clipping or SVBK selection occurs.
void w3dcgb_fast_map_clear_rect_asm()
{
    __asm {
        LD_A_MEM w3dcgb_fm_count_y
        LD_C_A
w3dfm_clr_row:
        LD_A_MEM w3dcgb_fm_y0
        CALL w3dfm_clr_addr_x0
        LD_A_MEM w3dcgb_fm_count_x
        LD_B_A
        LD_A_IMM 0x8A
w3dfm_clr_col:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfm_clr_col
        LD_A_MEM w3dcgb_fm_y0
        INC_A
        LD_MEM_A w3dcgb_fm_y0
        DEC_C
        JR_NZ w3dfm_clr_row
        RET

w3dfm_clr_addr_x0:
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_x0
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD3
        LD_E_IMM 0
        ADD_HL_DE
        RET
    }
}

// Zero a validated nonempty attribute rectangle at D500 with 32-byte rows.
// Advance shared fm_y0 by the row count; zero counts are invalid. The caller
// handles bounds and WRAM mapping. Tile numbers are untouched.
void w3dcgb_fast_map_clear_attr_rect_asm()
{
    __asm {
        LD_A_MEM w3dcgb_fm_count_y
        LD_C_A
w3dfm_aclr_row:
        LD_A_MEM w3dcgb_fm_y0
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_x0
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD5
        LD_E_IMM 0
        ADD_HL_DE
        LD_A_MEM w3dcgb_fm_count_x
        LD_B_A
        XOR_A
w3dfm_aclr_col:
        LDI_HL_A
        DEC_B
        JR_NZ w3dfm_aclr_col
        LD_A_MEM w3dcgb_fm_y0
        INC_A
        LD_MEM_A w3dcgb_fm_y0
        DEC_C
        JR_NZ w3dfm_aclr_row
        RET
    }
}

// Write sixteen consecutive tile IDs as a 4x4 stamp at D300 and the same
// attribute byte to all sixteen cells at D500. Rows have stride 32. Require
// the entire stamp inside the viewport and a stable WRAM mapping; no clipping.
void w3dcgb_fast_map_stamp_4x4_asm()
{
    __asm {
        LD_A_MEM w3dcgb_fm_stamp_y
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_stamp_x
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD3
        LD_E_IMM 0
        ADD_HL_DE
        LD_A_MEM w3dcgb_fm_stamp_tile
        LD_B_A
        LD_DE_IMM 28

        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        ADD_HL_DE

        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        ADD_HL_DE

        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        ADD_HL_DE

        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A
        INC_B
        LD_A_B
        LDI_HL_A

        LD_A_MEM w3dcgb_fm_stamp_y
        LD_L_A
        LD_H_IMM 0
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        ADD_HL_HL
        LD_A_MEM w3dcgb_fm_stamp_x
        LD_E_A
        LD_D_IMM 0
        ADD_HL_DE
        LD_D_IMM 0xD5
        LD_E_IMM 0
        ADD_HL_DE
        LD_A_MEM w3dcgb_fm_stamp_attr
        LD_DE_IMM 28

        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        ADD_HL_DE
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        ADD_HL_DE
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        ADD_HL_DE
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        LDI_HL_A
        RET
    }
}

// Order signed tile coordinates, reject fully off-screen rectangles and
// clip to 16x12 before the ASM border writer. Normalize color and ignore
// zero sides. Clipped endpoints become the border; this does not geometrically
// clip the original outline. Attributes remain and WRAM mapping is caller-owned.
static void w3dcgb_fast_map_draw_rect(w3dcgb_i16 x0, w3dcgb_i16 y0, w3dcgb_i16 x1, w3dcgb_i16 y1, w3dcgb_u8 color, w3dcgb_u8 sides)
{
    w3dcgb_i16 x;
    w3dcgb_i16 y;

    if (x0 > x1)
    {
        x = x0;
        x0 = x1;
        x1 = x;
    }
    if (y0 > y1)
    {
        y = y0;
        y0 = y1;
        y1 = y;
    }

    if (x1 < 0) return;
    if (y1 < 0) return;
    if (x0 >= 16) return;
    if (y0 >= 12) return;
    if (x0 < 0) x0 = 0;
    if (y0 < 0) y0 = 0;
    if (x1 > 15) x1 = 15;
    if (y1 > 11) y1 = 11;
    if (sides == 0) return;
    if (color == 0) color = WIRE3DCGB_COLOR_MAIN;
    if (color > 3) color = 3;

    w3dcgb_fm_x0 = (w3dcgb_u8)x0;
    w3dcgb_fm_y0 = (w3dcgb_u8)y0;
    w3dcgb_fm_x1 = (w3dcgb_u8)x1;
    w3dcgb_fm_y1 = (w3dcgb_u8)y1;
    w3dcgb_fm_count_x = (w3dcgb_u8)(x1 - x0 + 1);
    w3dcgb_fm_count_y = (w3dcgb_u8)(y1 - y0 + 1);
    w3dcgb_fm_tile_base = (w3dcgb_u8)(W3DCGB_FAST_TILE_BASE + ((color - 1) << 4));
    w3dcgb_fm_sides = sides;
    w3dcgb_fast_map_rect_asm();
}

// Clamp a signed tile coordinate to the inclusive range 0..max.
static w3dcgb_u8 w3dcgb_fast_tile_clamp(w3dcgb_i16 v, w3dcgb_u8 max)
{
    if (v < 0) return 0;
    if (v > (w3dcgb_i16)max) return max;
    return (w3dcgb_u8)v;
}

// Select one of four precomputed orientations from coarse rx/ry/rz bits.
// This is a stamp choice, not a matrix rotation or continuous projection.
static w3dcgb_u8 w3dcgb_fast_cube_variant(const Wire3DCGB_FastCube* cube)
{
    w3dcgb_u8 variant;

    variant = (w3dcgb_u8)((((w3dcgb_u8)cube->ry >> 2) & 1) | ((((w3dcgb_u8)cube->rx >> 2) & 1) << 1));
    variant = (w3dcgb_u8)(variant ^ (((w3dcgb_u8)cube->rz >> 2) & 3));
    return (w3dcgb_u8)(variant & 3);
}

// Select one of eight sixteen-tile stamp sets from a two-bit variant and
// one-bit all-edges flag. Remap offsets >=64 to ID 90 to avoid reserved IDs.
static w3dcgb_u8 w3dcgb_fast_stamp_tile_base(w3dcgb_u8 variant, w3dcgb_u8 show_all)
{
    w3dcgb_u8 index;

    index = (w3dcgb_u8)(((variant & 3) << 5) + ((show_all & 1) << 4));
    if (index < 64) return index;
    return (w3dcgb_u8)(0x90 + (index - 64));
}

// Place a fixed 4x4 cube stamp at the quantized 2D center; depth does not
// scale it and selected is ignored. Choose orientation/hidden-line pattern
// and color palette 1..3, recording clipped prior bounds for indices <3.
// Use an unrolled writer for fully visible stamps, otherwise clip cells
// individually while advancing their original tile IDs. Keep WRAM mapping stable.
static void w3dcgb_fast_map_draw_cube_one(const Wire3DCGB_FastCube* cube, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected, w3dcgb_u8 index)
{
    w3dcgb_i16 tx;
    w3dcgb_i16 ty;
    w3dcgb_i16 min_x;
    w3dcgb_i16 min_y;
    w3dcgb_i16 max_x;
    w3dcgb_i16 max_y;
    w3dcgb_u8 color;
    w3dcgb_u8 show_all;
    w3dcgb_u8 variant;
    w3dcgb_u8 row;
    w3dcgb_u8 col;
    w3dcgb_u8 tile;
    w3dcgb_u8 attr;

    selected = selected;
    color = cube->color;
    if (color == 0) color = WIRE3DCGB_COLOR_MAIN;
    if (color > 3) color = 3;
    if (hidden_enabled == 0) show_all = 1;
    else show_all = 0;
    variant = w3dcgb_fast_cube_variant(cube);
    tile = w3dcgb_fast_stamp_tile_base(variant, show_all);
    attr = (w3dcgb_u8)(color & 7);

    tx = (w3dcgb_i16)((((w3dcgb_i16)64 + cube->x) >> 3) - 2);
    ty = (w3dcgb_i16)((((w3dcgb_i16)48 - cube->y) >> 3) - 2);
    min_x = tx;
    min_y = ty;
    max_x = (w3dcgb_i16)(tx + 3);
    max_y = (w3dcgb_i16)(ty + 3);

    if (index < 3)
    {
        w3dcgb_u8 cx0;
        w3dcgb_u8 cy0;
        w3dcgb_u8 cx1;
        w3dcgb_u8 cy1;
        cx0 = w3dcgb_fast_tile_clamp(min_x, 15);
        cy0 = w3dcgb_fast_tile_clamp(min_y, 11);
        cx1 = w3dcgb_fast_tile_clamp(max_x, 15);
        cy1 = w3dcgb_fast_tile_clamp(max_y, 11);
        w3dcgb_fast_prev_x[(__safe_index w3dcgb_u8)index] = cx0;
        w3dcgb_fast_prev_y[(__safe_index w3dcgb_u8)index] = cy0;
        w3dcgb_fast_prev_w[(__safe_index w3dcgb_u8)index] = (w3dcgb_u8)(cx1 - cx0 + 1);
        w3dcgb_fast_prev_h[(__safe_index w3dcgb_u8)index] = (w3dcgb_u8)(cy1 - cy0 + 1);
    }

    if ((tx >= 0) && (ty >= 0) && (tx <= 12) && (ty <= 8))
    {
        w3dcgb_fm_stamp_x = (w3dcgb_u8)tx;
        w3dcgb_fm_stamp_y = (w3dcgb_u8)ty;
        w3dcgb_fm_stamp_tile = tile;
        w3dcgb_fm_stamp_attr = attr;
        w3dcgb_fast_map_stamp_4x4_asm();
        return;
    }

    row = 0;
    while (row < W3DCGB_FAST_STAMP_SIZE)
    {
        col = 0;
        while (col < W3DCGB_FAST_STAMP_SIZE)
        {
            w3dcgb_i16 dx;
            w3dcgb_i16 dy;
            dx = (w3dcgb_i16)(tx + col);
            dy = (w3dcgb_i16)(ty + row);
            if ((dx >= 0) && (dy >= 0) && (dx < 16) && (dy < 12))
            {
                w3dcgb_fast_map_put_attr((w3dcgb_u8)dx, (w3dcgb_u8)dy, tile, attr);
            }
            col = (w3dcgb_u8)(col + 1);
            tile = (w3dcgb_u8)(tile + 1);
        }
        row = (w3dcgb_u8)(row + 1);
    }
}

// On first use, clear all twelve shadow rows and reset prior-cube count,
// then mark FastMap ready. An already-ready map and its history are retained.
static void w3dcgb_fast_map_prepare()
{
    if (w3dcgb_fast_map_ready != 0) return;
    w3dcgb_fast_map_clear_shadow();
    w3dcgb_fast_map_ready = 1;
    w3dcgb_fast_prev_count = 0;
}

// Prepare the map, erase prior bounds/status cells, sort up to three cube
// stamps far-to-near by raw Z, then draw and upload numbers/attributes at
// VBlank. The caller array is not reordered. Non-null count zero clears
// old stamps; a null array is a complete no-op. selected & 3 chooses the
// status cell only, not a cube highlight. Requires loaded cube stamps, the
// full runtime and consistent FastMap WRAM mapping; it does not initialize them.
void Wire3DCGB_DrawFastCubeFrame(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected)
{
    w3dcgb_u8 i;
    w3dcgb_u8 scene_count;
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 c;
    w3dcgb_u8 x;

    if (cubes == 0) return;
    scene_count = count;
    if (scene_count > 3) scene_count = 3;

    w3dcgb_fast_map_prepare();

    i = 0;
    while (i < w3dcgb_fast_prev_count)
    {
        w3dcgb_fast_map_clear_rect(
            w3dcgb_fast_prev_x[(__safe_index w3dcgb_u8)i],
            w3dcgb_fast_prev_y[(__safe_index w3dcgb_u8)i],
            w3dcgb_fast_prev_w[(__safe_index w3dcgb_u8)i],
            w3dcgb_fast_prev_h[(__safe_index w3dcgb_u8)i]);
        i = (w3dcgb_u8)(i + 1);
    }
    w3dcgb_fast_map_clear_rect(0, 0, 4, 1);
    w3dcgb_fast_map_clear_rect(14, 0, 2, 1);

    if (scene_count == 0)
    {
        w3dcgb_fast_prev_count = 0;
        w3dcgb_wait_vblank_start();
        w3dcgb_fast_map_flush_gdma_asm();
        return;
    }

    a = 0;
    b = 1;
    c = 2;
    if (scene_count == 1)
    {
        w3dcgb_fast_map_draw_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0), 0);
        w3dcgb_fast_prev_count = 1;
    }
    else if (scene_count == 2)
    {
        if (cubes[0].z < cubes[1].z)
        {
            w3dcgb_fast_map_draw_cube_one(&cubes[1], hidden_enabled, (w3dcgb_u8)(selected == 1), 1);
            w3dcgb_fast_map_draw_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0), 0);
        }
        else
        {
            w3dcgb_fast_map_draw_cube_one(&cubes[0], hidden_enabled, (w3dcgb_u8)(selected == 0), 0);
            w3dcgb_fast_map_draw_cube_one(&cubes[1], hidden_enabled, (w3dcgb_u8)(selected == 1), 1);
        }
        w3dcgb_fast_prev_count = 2;
    }
    else
    {
        if (cubes[a].z < cubes[b].z)
        {
            w3dcgb_u8 t;
            t = a;
            a = b;
            b = t;
        }
        if (cubes[b].z < cubes[c].z)
        {
            w3dcgb_u8 t;
            t = b;
            b = c;
            c = t;
        }
        if (cubes[a].z < cubes[b].z)
        {
            w3dcgb_u8 t;
            t = a;
            a = b;
            b = t;
        }
        w3dcgb_fast_map_draw_cube_one(&cubes[a], hidden_enabled, (w3dcgb_u8)(selected == a), a);
        w3dcgb_fast_map_draw_cube_one(&cubes[b], hidden_enabled, (w3dcgb_u8)(selected == b), b);
        w3dcgb_fast_map_draw_cube_one(&cubes[c], hidden_enabled, (w3dcgb_u8)(selected == c), c);
        w3dcgb_fast_prev_count = 3;
    }

    x = (w3dcgb_u8)(selected & 3);
    w3dcgb_fast_map_or_cell((w3dcgb_i16)x, (w3dcgb_i16)0, WIRE3DCGB_COLOR_HIGH, 15);
    if (hidden_enabled != 0) w3dcgb_fast_map_or_cell((w3dcgb_i16)15, (w3dcgb_i16)0, WIRE3DCGB_COLOR_HIGH, 15);
    else w3dcgb_fast_map_or_cell((w3dcgb_i16)14, (w3dcgb_i16)0, WIRE3DCGB_COLOR_LOW, 15);

    w3dcgb_wait_vblank_start();
    w3dcgb_fast_map_flush_gdma_asm();
}
#endif
