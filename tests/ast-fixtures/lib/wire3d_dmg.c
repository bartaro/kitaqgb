// Shared DMG wireframe renderer. All profile selection is compile-time.
// Compile this file once; do not also compile a compatibility entry point.
#include "wire3d_dmg.h"


#if WIRE3D_DMG_HEIGHT == 96
// ROM bank 1 contains the C renderer; the low-level transfer/plot helpers below
// use explicit fixed-bank placement. This module owns shared drawing state and
// is not reentrant. It neither saves nor changes the caller's SVBK/VRAM bank.
#else
// The renderer uses shared non-reentrant state. C routines occupy bank 1,
// with explicit fixed-bank placement for low-level helpers below. Do not interleave
// rendering from interrupts; this module does not save/select SVBK or VRAM banks.
#endif
#pragma bank 1

#define WIRE3D_DMG_TILE_W ((w3ddmg_u8)16)
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_TILE_H ((w3ddmg_u8)12)
#define WIRE3D_DMG_ROW_BYTES ((w3ddmg_u8)0x60)
#else
#define WIRE3D_DMG_TILE_H ((w3ddmg_u8)15)
#define WIRE3D_DMG_ROW_BYTES ((w3ddmg_u8)0x78)
#endif
#define WIRE3D_DMG_NEAR_Z ((w3ddmg_i16)8)
#define WIRE3D_DMG_FAR_Z ((w3ddmg_i16)255)
#define WIRE3D_DMG_CENTER_X ((w3ddmg_i16)64)
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_CENTER_Y ((w3ddmg_i16)48)
#else
#define WIRE3D_DMG_CENTER_Y ((w3ddmg_i16)60)
#endif
#define WIRE3D_DMG_TRANSFORM_LIMIT ((w3ddmg_i16)220)
#define WIRE3D_DMG_PROJECT_LIMIT ((w3ddmg_i16)120)
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_HUD_TILE_BASE ((w3ddmg_u8)0x80)
#define WIRE3D_DMG_HUD_TILE_BLANK ((w3ddmg_u8)0x8A)
#define WIRE3D_DMG_HUD_TILE_COUNT ((w3ddmg_u8)14)
#endif
#define WIRE3D_DMG_BG_QUEUE_LIMIT ((w3ddmg_u8)48)
#if WIRE3D_DMG_HEIGHT == 120
#define WIRE3D_DMG_AUX_ROWS ((w3ddmg_u8)6)
#define WIRE3D_DMG_AUX_COL_BYTES ((w3ddmg_u8)0x38)
#endif

__location(0xFF40) w3ddmg_u8 w3ddmg_reg_lcdc;
__location(0xFF42) w3ddmg_u8 w3ddmg_reg_scy;
__location(0xFF43) w3ddmg_u8 w3ddmg_reg_scx;
__location(0xFF44) w3ddmg_u8 w3ddmg_reg_ly;
__location(0xFF47) w3ddmg_u8 w3ddmg_reg_bgp;

#if WIRE3D_DMG_HEIGHT == 96
// The renderer owns tile VRAM 0x8000..0x97FF and map 0x9800..0x9BFF.
// The 4096-byte WRAM stage uses column pages with 96 live bytes per page.
// Keep these fixed regions separate from other graphics and banked RAM users.
#endif
__location(0x8000) w3ddmg_u8 w3ddmg_vram_tiles[6144];
__location(0x9800) w3ddmg_u8 w3ddmg_bg_map_9800[1024];
__location(0xD000) w3ddmg_u8 w3ddmg_stage[4096];

__prg_rom w3ddmg_u8 w3ddmg_bit_mask[8] = {
    0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01
};

#if WIRE3D_DMG_HEIGHT == 96
// HUD glyphs 0123456789, space, S, L and V come from DAISUKE OBA
// original ASCII font under MIT, using the supplied GB tile conversion.
__prg_rom w3ddmg_u8 w3ddmg_hud_tiles[224] = {
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
#endif

#if WIRE3D_DMG_HEIGHT == 96
// Sixteen evenly spaced orientations use signed Q6 coefficients: 64 is unity.
#else

#endif
__prg_rom w3ddmg_i8 w3ddmg_sin_q6[16] = {
     0,  24,  45,  59,  64,  59,  45,  24,
     0, -24, -45, -59, -64, -59, -45, -24
};

__prg_rom w3ddmg_i8 w3ddmg_cos_q6[16] = {
     64,  59,  45,  24,   0, -24, -45, -59,
    -64, -59, -45, -24,   0,  24,  45,  59
};

#if WIRE3D_DMG_HEIGHT == 96
// Quantized perspective lookup, approximately 1536/Z where representable.
// The projection path rejects indices below 8 and above 255.
#endif
__prg_rom w3ddmg_u8 w3ddmg_inv_depth[256] = {
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

w3ddmg_i16 w3ddmg_cam_x;
w3ddmg_i16 w3ddmg_cam_y;
w3ddmg_i16 w3ddmg_cam_z;
#if WIRE3D_DMG_HEIGHT == 96
w3ddmg_i8 w3ddmg_cam_pitch;
w3ddmg_i8 w3ddmg_cam_yaw;
w3ddmg_i8 w3ddmg_cam_roll;
#else
w3ddmg_u8 w3ddmg_cam_pitch;
w3ddmg_u8 w3ddmg_cam_yaw;
w3ddmg_u8 w3ddmg_cam_roll;
#endif

w3ddmg_u8 w3ddmg_plot_x;
w3ddmg_u8 w3ddmg_plot_y;
w3ddmg_u8 w3ddmg_plot_tx;
w3ddmg_u8 w3ddmg_line_x0;
w3ddmg_u8 w3ddmg_line_y0;
w3ddmg_u8 w3ddmg_line_x1;
w3ddmg_u8 w3ddmg_line_y1;
w3ddmg_u8 w3ddmg_line_x;
w3ddmg_u8 w3ddmg_line_y;
w3ddmg_u8 w3ddmg_line_dx;
w3ddmg_u8 w3ddmg_line_dy;
w3ddmg_u8 w3ddmg_line_sx;
w3ddmg_u8 w3ddmg_line_sy;
w3ddmg_u8 w3ddmg_line_err;

#if WIRE3D_DMG_HEIGHT == 96
// Projection, visibility, line and scene scratch belong to the most recent draw.
// Do not interleave another model transform or interrupt-driven renderer call.
#endif
w3ddmg_u8 w3ddmg_screen_x[WIRE3D_DMG_MODEL_VERTEX_LIMIT];
w3ddmg_u8 w3ddmg_screen_y[WIRE3D_DMG_MODEL_VERTEX_LIMIT];
w3ddmg_u8 w3ddmg_screen_visible[WIRE3D_DMG_MODEL_VERTEX_LIMIT];
w3ddmg_u8 w3ddmg_face_visible[WIRE3D_DMG_MODEL_FACE_LIMIT];
#if WIRE3D_DMG_HEIGHT == 96
w3ddmg_u8 w3ddmg_edge_flags[WIRE3D_DMG_MODEL_EDGE_LIMIT];
w3ddmg_u8 w3ddmg_occlusion_mask[1536];
#else
w3ddmg_u8 w3ddmg_occlusion_mask[2048];
#endif
w3ddmg_u8 w3ddmg_occlusion_active;
#if WIRE3D_DMG_HEIGHT == 120
w3ddmg_u8 w3ddmg_aux_transfer_enabled;
w3ddmg_u8 w3ddmg_dirty_transfer_enabled;
// Column-major tile flags: current drawing and the previously uploaded frame.
// Their union ensures a moving object's old tiles are cleared on the next upload.
w3ddmg_u8 w3ddmg_dirty_tiles[240];
w3ddmg_u8 w3ddmg_prev_dirty_tiles[240];
w3ddmg_u8 w3ddmg_dirty_src_hi;
w3ddmg_u8 w3ddmg_dirty_src_lo;
w3ddmg_u8 w3ddmg_dirty_dst_hi;
w3ddmg_u8 w3ddmg_dirty_dst_lo;
#endif
w3ddmg_u8 w3ddmg_scene_order[WIRE3D_DMG_SCENE_OBJECT_LIMIT];
w3ddmg_i16 w3ddmg_scene_depth[WIRE3D_DMG_SCENE_OBJECT_LIMIT];
w3ddmg_u8 w3ddmg_bgq_x[48];
w3ddmg_u8 w3ddmg_bgq_y[48];
w3ddmg_u8 w3ddmg_bgq_tile[48];
w3ddmg_u8 w3ddmg_bgq_count;
w3ddmg_u8 w3ddmg_bgq_addr_hi;
w3ddmg_u8 w3ddmg_bgq_addr_lo;
w3ddmg_u8 w3ddmg_bgq_value;

void w3ddmg_clear_vram_asm();
void w3ddmg_fill_bg_map_asm();
void w3ddmg_clear_occlusion_mask_asm();
void w3ddmg_put_bg_tile_safe_asm();
#if WIRE3D_DMG_HEIGHT == 120
void w3ddmg_clear_stage_asm();
void w3ddmg_plot_stage_asm();
void w3ddmg_line_stage_asm();
void w3ddmg_line_inside_asm();
void w3ddmg_transfer_stage_asm();
void w3ddmg_transfer_dirty_tile_asm();
void w3ddmg_clear_stage_aux_asm();
void w3ddmg_transfer_stage_aux_asm();
#endif


static w3ddmg_i16 w3ddmg_clamp_i16(w3ddmg_i16 v, w3ddmg_i16 lo, w3ddmg_i16 hi);
static w3ddmg_u8 w3ddmg_clamp_screen(w3ddmg_i16 v, w3ddmg_u8 max);
#if WIRE3D_DMG_HEIGHT == 96
static w3ddmg_i8 w3ddmg_neg_angle(w3ddmg_i8 v);
static void w3ddmg_rotate_y(w3ddmg_i16* px, w3ddmg_i16* pz, w3ddmg_i8 angle);
static void w3ddmg_rotate_x(w3ddmg_i16* py, w3ddmg_i16* pz, w3ddmg_i8 angle);
static void w3ddmg_rotate_z(w3ddmg_i16* px, w3ddmg_i16* py, w3ddmg_i8 angle);
#else
static w3ddmg_u8 w3ddmg_angle_index(w3ddmg_u8 v);
static w3ddmg_u8 w3ddmg_neg_angle(w3ddmg_u8 v);
static void w3ddmg_rotate_y(w3ddmg_i16* px, w3ddmg_i16* pz, w3ddmg_u8 angle);
static void w3ddmg_rotate_x(w3ddmg_i16* py, w3ddmg_i16* pz, w3ddmg_u8 angle);
static void w3ddmg_rotate_z(w3ddmg_i16* px, w3ddmg_i16* py, w3ddmg_u8 angle);
#endif
static w3ddmg_u8 w3ddmg_project_camera_space(w3ddmg_i16 vx, w3ddmg_i16 vy, w3ddmg_i16 vz, w3ddmg_u8* sx, w3ddmg_u8* sy);
static w3ddmg_u8 w3ddmg_project_world(w3ddmg_i16 wx, w3ddmg_i16 wy, w3ddmg_i16 wz, w3ddmg_u8* sx, w3ddmg_u8* sy);
w3ddmg_u8 Wire3DDMG_ProjectPoint(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8* sx, w3ddmg_u8* sy);
#if WIRE3D_DMG_HEIGHT == 96
static void w3ddmg_load_hud_tiles();
#else
void Wire3DDMG_RotatePoint(w3ddmg_i16* x, w3ddmg_i16* y, w3ddmg_i16* z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz);
#endif
void Wire3DDMG_PutBgTile(w3ddmg_u8 x, w3ddmg_u8 y, w3ddmg_u8 tile);
void Wire3DDMG_SetPalette(w3ddmg_u8 bgp);
static void w3ddmg_flush_bg_queue();
static w3ddmg_i16 w3ddmg_scene_object_depth(const Wire3DDMG_Object* obj);
static w3ddmg_i16 w3ddmg_abs_i16(w3ddmg_i16 v);
static void w3ddmg_clear_occlusion_mask();
#if WIRE3D_DMG_HEIGHT == 120
static void w3ddmg_clear_dirty_tables();
static void w3ddmg_mark_tile_dirty(w3ddmg_u8 tx, w3ddmg_u8 ty);
static void w3ddmg_mark_rect_dirty(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1);
static void w3ddmg_mark_line_dirty(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1);
#endif
static w3ddmg_u16 w3ddmg_mask_offset(w3ddmg_u8 x, w3ddmg_u8 y);
static w3ddmg_u8 w3ddmg_mask_get(w3ddmg_u8 x, w3ddmg_u8 y);
static void w3ddmg_mask_set(w3ddmg_u8 x, w3ddmg_u8 y);
static void w3ddmg_stage_clear_pixel(w3ddmg_u8 x, w3ddmg_u8 y);
static void w3ddmg_stage_clear_span(w3ddmg_u8 y, w3ddmg_u8 min_x, w3ddmg_u8 max_x);
static void w3ddmg_mask_set_span(w3ddmg_u8 y, w3ddmg_u8 min_x, w3ddmg_u8 max_x);
static void w3ddmg_line_stage_masked_c(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1);
static w3ddmg_i16 w3ddmg_screen_delta(w3ddmg_u8 a, w3ddmg_u8 b);
static w3ddmg_i16 w3ddmg_edge_area(w3ddmg_i16 ax, w3ddmg_i16 ay, w3ddmg_i16 bx, w3ddmg_i16 by, w3ddmg_i16 px, w3ddmg_i16 py);
static void w3ddmg_mark_triangle(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy);
static void w3ddmg_clear_triangle(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy);
static void w3ddmg_build_face_visibility(const Wire3DDMG_Model* model, w3ddmg_u8 count, w3ddmg_u8 face_count);
static w3ddmg_u8 w3ddmg_is_edge_visible(const Wire3DDMG_Model* model, w3ddmg_u8 edge_index, w3ddmg_u8 face_count);
#if WIRE3D_DMG_HEIGHT == 96
w3ddmg_u16 Wire3DDMG_SelectEdgeMask(const Wire3DDMG_Model* model, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz);
static void w3ddmg_build_edge_flags(const Wire3DDMG_Model* model, w3ddmg_u8 edge_count, w3ddmg_u8 face_count, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz);
#endif
static void w3ddmg_mark_model_occluder(const Wire3DDMG_Model* model);
void w3ddmg_wait_vblank_start();
void w3ddmg_clear_vram_asm();
void w3ddmg_fill_bg_map_asm();
void w3ddmg_clear_occlusion_mask_asm();
void w3ddmg_put_bg_tile_safe_asm();
void w3ddmg_clear_stage_asm();
void w3ddmg_plot_stage_asm();
void w3ddmg_line_stage_asm();
void w3ddmg_transfer_stage_asm();
#if WIRE3D_DMG_HEIGHT == 120
void w3ddmg_transfer_dirty_tile_asm();
void w3ddmg_clear_stage_aux_asm();
void w3ddmg_transfer_stage_aux_asm();
#endif
void Wire3DDMG_Init();
void Wire3DDMG_BeginFrame();
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 pitch, w3ddmg_i8 yaw, w3ddmg_i8 roll);
#else
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 pitch, w3ddmg_u8 yaw, w3ddmg_u8 roll);
#endif
void Wire3DDMG_DrawLine2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by);
void Wire3DDMG_DrawScene(Wire3DDMG_Object* objects, w3ddmg_u8 count);
void Wire3DDMG_DrawLine3D(w3ddmg_i16 ax, w3ddmg_i16 ay, w3ddmg_i16 az, w3ddmg_i16 bx, w3ddmg_i16 by, w3ddmg_i16 bz);
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz);
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8);
#else
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz);
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8);
#endif
void Wire3DDMG_EraseTriangle2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy);
void Wire3DDMG_EraseSpan2D(w3ddmg_u8 y, w3ddmg_u8 x0, w3ddmg_u8 x1);
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8);
#else
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8);
void Wire3DDMG_DrawIndexedEdges(const Wire3DDMG_Vec3* vertices, w3ddmg_u8 vertex_count, const w3ddmg_u8* edges, w3ddmg_u8 edge_count, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 scale);
void Wire3DDMG_SetAuxTransfer(w3ddmg_u8 flag);
static void w3ddmg_transfer_dirty_stage();
void Wire3DDMG_SetDirtyTransfer(w3ddmg_u8 flag);
void Wire3DDMG_TransferMainNow();
void Wire3DDMG_TransferAuxNow();
#endif
void Wire3DDMG_EndFrame();


#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Clamp a signed value to an inclusive ordered range; lo must not exceed hi.
static w3ddmg_i16 w3ddmg_clamp_i16(w3ddmg_i16 v, w3ddmg_i16 lo, w3ddmg_i16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Clamp a projected signed coordinate to the inclusive byte range 0..max.
static w3ddmg_u8 w3ddmg_clamp_screen(w3ddmg_i16 v, w3ddmg_u8 max)
{
    if (v < 0) return 0;
    if (v > (w3ddmg_i16)max) return max;
    return (w3ddmg_u8)v;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Negate the stored angle; rotation helpers subsequently use only its low four bits.
static w3ddmg_i8 w3ddmg_neg_angle(w3ddmg_i8 v)
#else
// Normalize to 0..15, then return the opposite rotation modulo one full turn.
static w3ddmg_u8 w3ddmg_neg_angle(w3ddmg_u8 v)
#endif
{
#if WIRE3D_DMG_HEIGHT == 96
    return (w3ddmg_i8)(0 - v);
#else
    v = w3ddmg_angle_index(v);
    if (v == 0) return 0;
    return (w3ddmg_u8)(WIRE3D_DMG_ANGLE_STEPS - v);
#endif
}

#pragma bank 2
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Rotate X/Z using a 16-step full-turn angle and Q6 sine/cosine tables.
// Clamp each input to +/-220 before multiplying; write both results in place.
// Pointers must be valid and distinct to preserve both returned components.
static void w3ddmg_rotate_y(w3ddmg_i16* px, w3ddmg_i16* pz, w3ddmg_i8 angle)
#else
// Rotate X/Z using normalized 16-step angles and Q6 coefficients. Clamp each
// input to +/-220 before multiplying; valid distinct pointers receive the result.
static void w3ddmg_rotate_y(w3ddmg_i16* px, w3ddmg_i16* pz, w3ddmg_u8 angle)
#endif
{
    w3ddmg_u8 ai;
    w3ddmg_i16 s;
    w3ddmg_i16 c;
    w3ddmg_i16 in_x;
    w3ddmg_i16 in_z;
    w3ddmg_i16 out_x;
    w3ddmg_i16 out_z;

#if WIRE3D_DMG_HEIGHT == 96
    ai = (w3ddmg_u8)angle;
    ai = (w3ddmg_u8)(ai & 15);
#else
    ai = w3ddmg_angle_index(angle);
#endif
    in_x = w3ddmg_clamp_i16(*px, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    in_z = w3ddmg_clamp_i16(*pz, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    // Cardinal rotations are exact swaps/sign changes after the same input
    // clamps. Avoid four multiplications and two Q6 shifts for these angles.
    if ((ai & 3) == 0)
    {
        if (ai == 0) { *px = in_x; *pz = in_z; return; }
        if (ai == 4) { *px = in_z; *pz = (w3ddmg_i16)(0 - in_x); return; }
        if (ai == 8) { *px = (w3ddmg_i16)(0 - in_x); *pz = (w3ddmg_i16)(0 - in_z); return; }
        *px = (w3ddmg_i16)(0 - in_z); *pz = in_x; return;
    }
    s = w3ddmg_sin_q6[(__safe_index w3ddmg_u8)ai];
    c = w3ddmg_cos_q6[(__safe_index w3ddmg_u8)ai];
    out_x = (w3ddmg_i16)(((in_x * c) + (in_z * s)) >> 6);
    out_z = (w3ddmg_i16)(((in_z * c) - (in_x * s)) >> 6);
    *px = out_x;
    *pz = out_z;
}

#pragma bank 2
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Rotate Y/Z using the low four angle bits and Q6 coefficients. Clamp inputs
// to +/-220 before the products; valid distinct pointers receive both components.
static void w3ddmg_rotate_x(w3ddmg_i16* py, w3ddmg_i16* pz, w3ddmg_i8 angle)
#else
// Rotate Y/Z using normalized 16-step angles and Q6 coefficients. Clamp each
// input to +/-220 before multiplying; valid distinct pointers receive the result.
static void w3ddmg_rotate_x(w3ddmg_i16* py, w3ddmg_i16* pz, w3ddmg_u8 angle)
#endif
{
    w3ddmg_u8 ai;
    w3ddmg_i16 s;
    w3ddmg_i16 c;
    w3ddmg_i16 in_y;
    w3ddmg_i16 in_z;
    w3ddmg_i16 out_y;
    w3ddmg_i16 out_z;

#if WIRE3D_DMG_HEIGHT == 96
    ai = (w3ddmg_u8)angle;
    ai = (w3ddmg_u8)(ai & 15);
#else
    ai = w3ddmg_angle_index(angle);
#endif
    in_y = w3ddmg_clamp_i16(*py, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    in_z = w3ddmg_clamp_i16(*pz, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    // Cardinal rotations are exact swaps/sign changes after the same input
    // clamps. Avoid four multiplications and two Q6 shifts for these angles.
    if ((ai & 3) == 0)
    {
        if (ai == 0) { *py = in_y; *pz = in_z; return; }
        if (ai == 4) { *py = (w3ddmg_i16)(0 - in_z); *pz = in_y; return; }
        if (ai == 8) { *py = (w3ddmg_i16)(0 - in_y); *pz = (w3ddmg_i16)(0 - in_z); return; }
        *py = in_z; *pz = (w3ddmg_i16)(0 - in_y); return;
    }
    s = w3ddmg_sin_q6[(__safe_index w3ddmg_u8)ai];
    c = w3ddmg_cos_q6[(__safe_index w3ddmg_u8)ai];
    out_y = (w3ddmg_i16)(((in_y * c) - (in_z * s)) >> 6);
    out_z = (w3ddmg_i16)(((in_y * s) + (in_z * c)) >> 6);
    *py = out_y;
    *pz = out_z;
}

#pragma bank 2
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Rotate X/Y using the low four angle bits and Q6 coefficients. Clamp inputs
// to +/-220 before the products; valid distinct pointers receive both components.
static void w3ddmg_rotate_z(w3ddmg_i16* px, w3ddmg_i16* py, w3ddmg_i8 angle)
#else
// Rotate X/Y using normalized 16-step angles and Q6 coefficients. Clamp each
// input to +/-220 before multiplying; valid distinct pointers receive the result.
static void w3ddmg_rotate_z(w3ddmg_i16* px, w3ddmg_i16* py, w3ddmg_u8 angle)
#endif
{
    w3ddmg_u8 ai;
    w3ddmg_i16 s;
    w3ddmg_i16 c;
    w3ddmg_i16 in_x;
    w3ddmg_i16 in_y;
    w3ddmg_i16 out_x;
    w3ddmg_i16 out_y;

#if WIRE3D_DMG_HEIGHT == 96
    ai = (w3ddmg_u8)angle;
    ai = (w3ddmg_u8)(ai & 15);
#else
    ai = w3ddmg_angle_index(angle);
#endif
    in_x = w3ddmg_clamp_i16(*px, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    in_y = w3ddmg_clamp_i16(*py, (w3ddmg_i16)(0 - WIRE3D_DMG_TRANSFORM_LIMIT), WIRE3D_DMG_TRANSFORM_LIMIT);
    // Cardinal rotations are exact swaps/sign changes after the same input
    // clamps. Avoid four multiplications and two Q6 shifts for these angles.
    if ((ai & 3) == 0)
    {
        if (ai == 0) { *px = in_x; *py = in_y; return; }
        if (ai == 4) { *px = (w3ddmg_i16)(0 - in_y); *py = in_x; return; }
        if (ai == 8) { *px = (w3ddmg_i16)(0 - in_x); *py = (w3ddmg_i16)(0 - in_y); return; }
        *px = in_y; *py = (w3ddmg_i16)(0 - in_x); return;
    }
    s = w3ddmg_sin_q6[(__safe_index w3ddmg_u8)ai];
    c = w3ddmg_cos_q6[(__safe_index w3ddmg_u8)ai];
    out_x = (w3ddmg_i16)(((in_x * c) - (in_y * s)) >> 6);
    out_y = (w3ddmg_i16)(((in_x * s) + (in_y * c)) >> 6);
    *px = out_x;
    *py = out_y;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Reject Z outside 8..255 without writing outputs; otherwise clamp X/Y to
// +/-120, project with a reciprocal table and pin coordinates to 0..127 by 0..119.
// Both output pointers must be writable. This screen clamp is not line clipping.
static w3ddmg_u8 w3ddmg_project_camera_space(w3ddmg_i16 vx, w3ddmg_i16 vy, w3ddmg_i16 vz, w3ddmg_u8* sx, w3ddmg_u8* sy)
{
    w3ddmg_u8 iz;
    w3ddmg_i16 px;
    w3ddmg_i16 py;
    w3ddmg_i16 ox;
    w3ddmg_i16 oy;

    if (vz < WIRE3D_DMG_NEAR_Z) return 0;
    if (vz > WIRE3D_DMG_FAR_Z) return 0;

    px = w3ddmg_clamp_i16(vx, (w3ddmg_i16)(0 - WIRE3D_DMG_PROJECT_LIMIT), WIRE3D_DMG_PROJECT_LIMIT);
    py = w3ddmg_clamp_i16(vy, (w3ddmg_i16)(0 - WIRE3D_DMG_PROJECT_LIMIT), WIRE3D_DMG_PROJECT_LIMIT);
    iz = w3ddmg_inv_depth[(__safe_index w3ddmg_u8)((w3ddmg_u8)vz)];

    ox = (w3ddmg_i16)((px * (w3ddmg_i16)iz) >> 5);
    oy = (w3ddmg_i16)((py * (w3ddmg_i16)iz) >> 5);

    *sx = w3ddmg_clamp_screen((w3ddmg_i16)(WIRE3D_DMG_CENTER_X + ox), (w3ddmg_u8)(WIRE3D_DMG_SCREEN_W - 1));
    *sy = w3ddmg_clamp_screen((w3ddmg_i16)(WIRE3D_DMG_CENTER_Y - oy), (w3ddmg_u8)(WIRE3D_DMG_SCREEN_H - 1));
    return 1;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Subtract the camera position, then rotate by negative yaw, pitch and roll
// before projection. Keep coordinate differences representable as s16.
// The rotation clamps also apply to this camera transform.
static w3ddmg_u8 w3ddmg_project_world(w3ddmg_i16 wx, w3ddmg_i16 wy, w3ddmg_i16 wz, w3ddmg_u8* sx, w3ddmg_u8* sy)
{
    w3ddmg_i16 vx;
    w3ddmg_i16 vy;
    w3ddmg_i16 vz;

    vx = (w3ddmg_i16)(wx - w3ddmg_cam_x);
    vy = (w3ddmg_i16)(wy - w3ddmg_cam_y);
    vz = (w3ddmg_i16)(wz - w3ddmg_cam_z);

#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_rotate_y(&vx, &vz, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_yaw));
    w3ddmg_rotate_x(&vy, &vz, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_pitch));
    w3ddmg_rotate_z(&vx, &vy, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_roll));
#else
    w3ddmg_rotate_y(&vx, &vz, w3ddmg_neg_angle(w3ddmg_cam_yaw));
    w3ddmg_rotate_x(&vy, &vz, w3ddmg_neg_angle(w3ddmg_cam_pitch));
    w3ddmg_rotate_z(&vx, &vy, w3ddmg_neg_angle(w3ddmg_cam_roll));
#endif

    return w3ddmg_project_camera_space(vx, vy, vz, sx, sy);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Project a world point with the current camera. Return zero for invalid
// depth without changing outputs; success writes clamped 128x120 coordinates.
// Both output pointers must be valid. Coordinate differences must fit s16.
w3ddmg_u8 Wire3DDMG_ProjectPoint(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8* sx, w3ddmg_u8* sy)
{
    return w3ddmg_project_world(x, y, z, sx, sy);
}

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Copy 14 built-in HUD tiles to VRAM at 0x8800 without checking LCD state.
// Initialization calls this while the LCD is off.
static void w3ddmg_load_hud_tiles()
{
    w3ddmg_u16 src;
    w3ddmg_u16 dst;

    src = 0;
    dst = 0x0800;
    while (src < (w3ddmg_u16)(WIRE3D_DMG_HUD_TILE_COUNT * 16))
    {
        w3ddmg_vram_tiles[(__safe_index w3ddmg_u16)dst] = w3ddmg_hud_tiles[(__safe_index w3ddmg_u16)src];
        src = (w3ddmg_u16)(src + 1);
        dst = (w3ddmg_u16)(dst + 1);
    }
}
#endif

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Queue a tile write to the 32x32 map at 0x9800 for EndFrame. Invalid map
// coordinates and submissions beyond 48 pending writes are silently ignored.
// This queue is separate from the general vram library queue.
void Wire3DDMG_PutBgTile(w3ddmg_u8 x, w3ddmg_u8 y, w3ddmg_u8 tile)
{
    if (x >= 32) return;
    if (y >= 32) return;
    if (w3ddmg_bgq_count >= WIRE3D_DMG_BG_QUEUE_LIMIT) return;
    w3ddmg_bgq_x[(__safe_index w3ddmg_u8)w3ddmg_bgq_count] = x;
    w3ddmg_bgq_y[(__safe_index w3ddmg_u8)w3ddmg_bgq_count] = y;
    w3ddmg_bgq_tile[(__safe_index w3ddmg_u8)w3ddmg_bgq_count] = tile;
    w3ddmg_bgq_count = (w3ddmg_u8)(w3ddmg_bgq_count + 1);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Write the raw DMG BGP palette encoding immediately; no frame synchronization occurs.
void Wire3DDMG_SetPalette(w3ddmg_u8 bgp)
{
    w3ddmg_reg_bgp = bgp;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Convert queued tile coordinates to map addresses and write them in order
// through the STAT-polling helper. Clear the count after all writes complete.
static void w3ddmg_flush_bg_queue()
{
    w3ddmg_u8 i;

    i = 0;
    while (i < w3ddmg_bgq_count)
    {
        w3ddmg_u16 off;

        off = (w3ddmg_u16)(((w3ddmg_u16)w3ddmg_bgq_y[(__safe_index w3ddmg_u8)i] << 5) + (w3ddmg_u16)w3ddmg_bgq_x[(__safe_index w3ddmg_u8)i]);
        off = (w3ddmg_u16)(off + 0x9800);
        w3ddmg_bgq_addr_hi = (w3ddmg_u8)(off >> 8);
        w3ddmg_bgq_addr_lo = (w3ddmg_u8)off;
        w3ddmg_bgq_value = w3ddmg_bgq_tile[(__safe_index w3ddmg_u8)i];
        w3ddmg_put_bg_tile_safe_asm();
        i = (w3ddmg_u8)(i + 1);
    }
    w3ddmg_bgq_count = 0;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Transform only the object origin into camera space for scene ordering.
// This center-depth estimate ignores model extent and scale.
static w3ddmg_i16 w3ddmg_scene_object_depth(const Wire3DDMG_Object* obj)
{
    w3ddmg_i16 vx;
    w3ddmg_i16 vy;
    w3ddmg_i16 vz;

    vx = (w3ddmg_i16)(obj->x - w3ddmg_cam_x);
    vy = (w3ddmg_i16)(obj->y - w3ddmg_cam_y);
    vz = (w3ddmg_i16)(obj->z - w3ddmg_cam_z);

#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_rotate_y(&vx, &vz, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_yaw));
    w3ddmg_rotate_x(&vy, &vz, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_pitch));
    w3ddmg_rotate_z(&vx, &vy, (w3ddmg_i8)w3ddmg_neg_angle(w3ddmg_cam_roll));
#else
    w3ddmg_rotate_y(&vx, &vz, w3ddmg_neg_angle(w3ddmg_cam_yaw));
    w3ddmg_rotate_x(&vy, &vz, w3ddmg_neg_angle(w3ddmg_cam_pitch));
    w3ddmg_rotate_z(&vx, &vy, w3ddmg_neg_angle(w3ddmg_cam_roll));
#endif

    return vz;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Return signed magnitude; -32768 has no positive s16 representation.
static w3ddmg_i16 w3ddmg_abs_i16(w3ddmg_i16 v)
{
    if (v < 0) return (w3ddmg_i16)(0 - v);
    return v;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Clear the separate 2048-byte occlusion allocation without changing pixels
// or dirty flags. Only the first 1920 bytes correspond to the 128x120 viewport.
static void w3ddmg_clear_occlusion_mask()
{
    w3ddmg_clear_occlusion_mask_asm();
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Map a valid 128x120 coordinate to its row-major 1bpp byte at y*16+x/8.
static w3ddmg_u16 w3ddmg_mask_offset(w3ddmg_u8 x, w3ddmg_u8 y)
{
    return (w3ddmg_u16)(((w3ddmg_u16)y << 4) + (w3ddmg_u16)(x >> 3));
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Read the screen mask, treating any out-of-bounds coordinate as occluded.
static w3ddmg_u8 w3ddmg_mask_get(w3ddmg_u8 x, w3ddmg_u8 y)
{
    w3ddmg_u16 ofs;
    w3ddmg_u8 bit;

    if (x >= WIRE3D_DMG_SCREEN_W) return 1;
    if (y >= WIRE3D_DMG_SCREEN_H) return 1;

    ofs = w3ddmg_mask_offset(x, y);
    bit = w3ddmg_bit_mask[(__safe_index w3ddmg_u8)(x & 7)];
    if ((w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] & bit) != 0) return 1;
    return 0;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Set one occlusion bit; out-of-bounds coordinates are ignored.
static void w3ddmg_mask_set(w3ddmg_u8 x, w3ddmg_u8 y)
{
    w3ddmg_u16 ofs;
    w3ddmg_u8 bit;

    if (x >= WIRE3D_DMG_SCREEN_W) return;
    if (y >= WIRE3D_DMG_SCREEN_H) return;

    ofs = w3ddmg_mask_offset(x, y);
    bit = w3ddmg_bit_mask[(__safe_index w3ddmg_u8)(x & 7)];
    w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] = (w3ddmg_u8)(w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] | bit);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Clear one staged bit in the column-page layout, ignoring invalid pixels.
// This helper alone does not mark a dirty tile or modify the occlusion mask.
static void w3ddmg_stage_clear_pixel(w3ddmg_u8 x, w3ddmg_u8 y)
{
    w3ddmg_u16 ofs;
    w3ddmg_u8 bit;

    if (x >= WIRE3D_DMG_SCREEN_W) return;
    if (y >= WIRE3D_DMG_SCREEN_H) return;

    ofs = (w3ddmg_u16)((((w3ddmg_u16)(x >> 3)) << 8) + (((w3ddmg_u16)(y >> 3)) << 3) + (w3ddmg_u16)(y & 7));
    bit = w3ddmg_bit_mask[(__safe_index w3ddmg_u8)(x & 7)];
    w3ddmg_stage[(__safe_index w3ddmg_u16)ofs] = (w3ddmg_u8)(w3ddmg_stage[(__safe_index w3ddmg_u16)ofs] & (w3ddmg_u8)(0xFF ^ bit));
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Clear an inclusive span using individual edge bits and whole middle bytes.
// The caller must supply ordered X coordinates in 0..127; invalid Y is ignored.
// This helper does not clip arbitrary byte-sized X inputs.
#else
// Mark dirty tiles when enabled and erase an inclusive staged span. Use
// ordered X endpoints in 0..127; invalid Y is ignored. Dirty-marking clamps its
// copy of the endpoints, but this erase loop still requires valid X inputs.
#endif
static void w3ddmg_stage_clear_span(w3ddmg_u8 y, w3ddmg_u8 min_x, w3ddmg_u8 max_x)
{
    w3ddmg_u8 x;
    w3ddmg_u16 ofs;

    if (y >= WIRE3D_DMG_SCREEN_H) return;
#if WIRE3D_DMG_HEIGHT == 120
    w3ddmg_mark_rect_dirty(min_x, y, max_x, y);
#endif

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        w3ddmg_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3ddmg_u8)(x + 1);
    }

    while ((w3ddmg_u8)(x + 7) <= max_x)
    {
        ofs = (w3ddmg_u16)((((w3ddmg_u16)(x >> 3)) << 8) + (((w3ddmg_u16)(y >> 3)) << 3) + (w3ddmg_u16)(y & 7));
        w3ddmg_stage[(__safe_index w3ddmg_u16)ofs] = 0;
        x = (w3ddmg_u8)(x + 8);
    }

    while (x <= max_x)
    {
        w3ddmg_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3ddmg_u8)(x + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// OR an inclusive span into its row-major mask bytes. Compute the row
// address once, preserve both partial boundary bytes and fill whole bytes.
// Supply ordered X endpoints in 0..127; invalid Y is ignored.
static void w3ddmg_mask_set_span(w3ddmg_u8 y, w3ddmg_u8 min_x, w3ddmg_u8 max_x)
{
    w3ddmg_u16 ofs;
    w3ddmg_u8 first;
    w3ddmg_u8 last;
    w3ddmg_u8 left;
    w3ddmg_u8 right;

    if (y >= WIRE3D_DMG_SCREEN_H) return;
    first = (w3ddmg_u8)(min_x >> 3);
    last = (w3ddmg_u8)(max_x >> 3);
    ofs = (w3ddmg_u16)(((w3ddmg_u16)y << 4) + first);
    left = (w3ddmg_u8)((w3ddmg_bit_mask[(__safe_index w3ddmg_u8)(min_x & 7)] << 1) - 1);
    right = (w3ddmg_u8)(255 ^ (w3ddmg_bit_mask[(__safe_index w3ddmg_u8)(max_x & 7)] - 1));
    if (first == last)
    {
        w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] = (w3ddmg_u8)(w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] | (left & right));
        return;
    }
    w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] = (w3ddmg_u8)(w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] | left);
    ofs = (w3ddmg_u16)(ofs + 1);
    first = (w3ddmg_u8)(first + 1);
    while (first < last)
    {
        w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] = 255;
        ofs = (w3ddmg_u16)(ofs + 1);
        first = (w3ddmg_u8)(first + 1);
    }
    w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] = (w3ddmg_u8)(w3ddmg_occlusion_mask[(__safe_index w3ddmg_u16)ofs] | right);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Test the endpoints, midpoint and quarter points against the mask. Skip
// the entire line only if all five are covered; otherwise draw the entire line.
// This is a coarse rejection heuristic, not per-pixel occlusion clipping.
static void w3ddmg_line_stage_masked_c(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1)
{
    w3ddmg_u8 mx;
    w3ddmg_u8 my;
    w3ddmg_u8 q0x;
    w3ddmg_u8 q0y;
    w3ddmg_u8 q1x;
    w3ddmg_u8 q1y;

    mx = (w3ddmg_u8)(((w3ddmg_u16)x0 + (w3ddmg_u16)x1) >> 1);
    my = (w3ddmg_u8)(((w3ddmg_u16)y0 + (w3ddmg_u16)y1) >> 1);
    q0x = (w3ddmg_u8)(((w3ddmg_u16)x0 + (w3ddmg_u16)mx) >> 1);
    q0y = (w3ddmg_u8)(((w3ddmg_u16)y0 + (w3ddmg_u16)my) >> 1);
    q1x = (w3ddmg_u8)(((w3ddmg_u16)x1 + (w3ddmg_u16)mx) >> 1);
    q1y = (w3ddmg_u8)(((w3ddmg_u16)y1 + (w3ddmg_u16)my) >> 1);

    if (w3ddmg_mask_get(x0, y0) &&
        w3ddmg_mask_get(q0x, q0y) &&
        w3ddmg_mask_get(mx, my) &&
        w3ddmg_mask_get(q1x, q1y) &&
        w3ddmg_mask_get(x1, y1))
    {
        return;
    }

    w3ddmg_line_x0 = x0;
    w3ddmg_line_y0 = y0;
    w3ddmg_line_x1 = x1;
    w3ddmg_line_y1 = y1;
    // Accepted in-bounds lines use the same connected register loop as direct draws.
    if (x0 < 128 && x1 < 128 && y0 < WIRE3D_DMG_SCREEN_H && y1 < WIRE3D_DMG_SCREEN_H)
        w3ddmg_line_inside_asm();
    else
        w3ddmg_line_stage_asm();
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Widen both byte coordinates before subtracting so negative screen deltas survive.
static w3ddmg_i16 w3ddmg_screen_delta(w3ddmg_u8 a, w3ddmg_u8 b)
{
    return (w3ddmg_i16)((w3ddmg_i16)a - (w3ddmg_i16)b);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Return the signed 2D edge cross product. Screen-sized inputs fit this
// renderer's 16-bit arithmetic; unrestricted coordinates may overflow.
static w3ddmg_i16 w3ddmg_edge_area(w3ddmg_i16 ax, w3ddmg_i16 ay, w3ddmg_i16 bx, w3ddmg_i16 by, w3ddmg_i16 px, w3ddmg_i16 py)
{
    return (w3ddmg_i16)(((bx - ax) * (py - ay)) - ((by - ay) * (px - ax)));
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// For a nondegenerate projected triangle, mark its entire bounding rectangle
// as occluded. This approximation includes pixels outside the triangle.
// All input coordinates must lie in the 128x120 viewport.
static void w3ddmg_mark_triangle(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy)
{
    w3ddmg_u8 min_x;
    w3ddmg_u8 max_x;
    w3ddmg_u8 min_y;
    w3ddmg_u8 max_y;
    w3ddmg_i16 area;
    w3ddmg_u8 y;

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

    area = w3ddmg_edge_area((w3ddmg_i16)ax, (w3ddmg_i16)ay, (w3ddmg_i16)bx, (w3ddmg_i16)by, (w3ddmg_i16)cx, (w3ddmg_i16)cy);
    if (area == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        w3ddmg_mask_set_span(y, min_x, max_x);
        if (y == max_y) break;
        y = (w3ddmg_u8)(y + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Erase scanline spans between integer edge intersections, skipping degenerate
// triangles and horizontal edges. Each span marks dirty tiles when enabled.
// Supply viewport coordinates; this changes the stage, not the mask or VRAM.
static void w3ddmg_clear_triangle(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy)
{
    w3ddmg_u8 min_x;
    w3ddmg_u8 max_x;
    w3ddmg_u8 min_y;
    w3ddmg_u8 max_y;
    w3ddmg_u8 y;

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

    if (w3ddmg_edge_area((w3ddmg_i16)ax, (w3ddmg_i16)ay, (w3ddmg_i16)bx, (w3ddmg_i16)by, (w3ddmg_i16)cx, (w3ddmg_i16)cy) == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        w3ddmg_u8 hits;
        w3ddmg_i16 span_min;
        w3ddmg_i16 span_max;
        w3ddmg_i16 dx;
        w3ddmg_i16 dy;
        w3ddmg_i16 ix;

        hits = 0;
        span_min = 127;
        span_max = 0;

        if (ay != by)
        {
            if (((y >= ay) && (y <= by)) || ((y >= by) && (y <= ay)))
            {
                dx = (w3ddmg_i16)((w3ddmg_i16)bx - (w3ddmg_i16)ax);
                dy = (w3ddmg_i16)((w3ddmg_i16)by - (w3ddmg_i16)ay);
                ix = (w3ddmg_i16)((w3ddmg_i16)ax + ((((w3ddmg_i16)y - (w3ddmg_i16)ay) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3ddmg_u8)(hits + 1);
            }
        }

        if (by != cy)
        {
            if (((y >= by) && (y <= cy)) || ((y >= cy) && (y <= by)))
            {
                dx = (w3ddmg_i16)((w3ddmg_i16)cx - (w3ddmg_i16)bx);
                dy = (w3ddmg_i16)((w3ddmg_i16)cy - (w3ddmg_i16)by);
                ix = (w3ddmg_i16)((w3ddmg_i16)bx + ((((w3ddmg_i16)y - (w3ddmg_i16)by) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3ddmg_u8)(hits + 1);
            }
        }

        if (cy != ay)
        {
            if (((y >= cy) && (y <= ay)) || ((y >= ay) && (y <= cy)))
            {
                dx = (w3ddmg_i16)((w3ddmg_i16)ax - (w3ddmg_i16)cx);
                dy = (w3ddmg_i16)((w3ddmg_i16)ay - (w3ddmg_i16)cy);
                ix = (w3ddmg_i16)((w3ddmg_i16)cx + ((((w3ddmg_i16)y - (w3ddmg_i16)cy) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3ddmg_u8)(hits + 1);
            }
        }

        if (hits >= 2)
        {
            if (span_min < (w3ddmg_i16)min_x) span_min = (w3ddmg_i16)min_x;
            if (span_max > (w3ddmg_i16)max_x) span_max = (w3ddmg_i16)max_x;
            if (span_min < 0) span_min = 0;
            if (span_max > (w3ddmg_i16)(WIRE3D_DMG_SCREEN_W - 1)) span_max = (w3ddmg_i16)(WIRE3D_DMG_SCREEN_W - 1);
            if (span_min <= span_max)
            {
                w3ddmg_stage_clear_span(y, (w3ddmg_u8)span_min, (w3ddmg_u8)span_max);
            }
        }

        if (y == max_y) break;
        y = (w3ddmg_u8)(y + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Cache front-facing triangles using positive projected signed area. Invalid
// vertex indices or any depth-rejected vertex make a face invisible. Supply
// non-null faces and counts already limited to the shared scratch capacities.
static void w3ddmg_build_face_visibility(const Wire3DDMG_Model* model, w3ddmg_u8 count, w3ddmg_u8 face_count)
{
    w3ddmg_u8 i;

    i = 0;
    while (i < face_count)
    {
        const Wire3DDMG_Face* f;
        w3ddmg_i16 abx;
        w3ddmg_i16 aby;
        w3ddmg_i16 acx;
        w3ddmg_i16 acy;
        w3ddmg_i16 area;

        w3ddmg_face_visible[(__safe_index w3ddmg_u8)i] = 0;
        f = &model->faces[(__safe_index w3ddmg_u8)i];

        if ((f->a < count) && (f->b < count) && (f->c < count))
        {
            if (w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->a] &&
                w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->b] &&
                w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->c])
            {
                abx = w3ddmg_screen_delta(w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->b], w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->a]);
                aby = w3ddmg_screen_delta(w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->b], w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->a]);
                acx = w3ddmg_screen_delta(w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->c], w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->a]);
                acy = w3ddmg_screen_delta(w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->c], w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->a]);
                area = (w3ddmg_i16)((abx * acy) - (aby * acx));
                if (area > 0)
                {
                    w3ddmg_face_visible[(__safe_index w3ddmg_u8)i] = 1;
                }
            }
        }
        i = (w3ddmg_u8)(i + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Keep an edge unless hidden-line filtering has valid adjacent faces and
// all of them are back-facing. Missing adjacency, FACE_NONE pairs and invalid
// face IDs fail open. This renderer has no precomputed yaw edge masks.
static w3ddmg_u8 w3ddmg_is_edge_visible(const Wire3DDMG_Model* model, w3ddmg_u8 edge_index, w3ddmg_u8 face_count)
{
    const Wire3DDMG_EdgeFaces* ef;
    w3ddmg_u8 any_face;

    if ((model->flags & WIRE3D_DMG_MODEL_HIDDEN_LINES) == 0) return 1;
    if (model->faces == 0) return 1;
    if (model->edge_faces == 0) return 1;

    ef = &model->edge_faces[(__safe_index w3ddmg_u8)edge_index];
    any_face = 0;

    if (ef->f0 != WIRE3D_DMG_FACE_NONE)
    {
        any_face = 1;
        if (ef->f0 >= face_count) return 1;
        if (w3ddmg_face_visible[(__safe_index w3ddmg_u8)ef->f0]) return 1;
    }

    if (ef->f1 != WIRE3D_DMG_FACE_NONE)
    {
        any_face = 1;
        if (ef->f1 >= face_count) return 1;
        if (w3ddmg_face_visible[(__safe_index w3ddmg_u8)ef->f1]) return 1;
    }

    if (any_face == 0) return 1;
    return 0;
}

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Select a precomputed mask from the low four bits of object yaw; pitch,
// roll and camera orientation are ignored. Use 16, 8, 4 or 2 bins; other positive
// counts select entry zero (counts >=16 use the first 16). Missing data returns
// 0xFFFF. Keep the mask table readable through the current data-pointer mapping.
w3ddmg_u16 Wire3DDMG_SelectEdgeMask(const Wire3DDMG_Model* model, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz)
{
    w3ddmg_u8 count;
    w3ddmg_u8 bin;

    if (model == 0) return 0xFFFF;
    if (model->edge_masks == 0) return 0xFFFF;
    count = model->edge_mask_count;
    if (count == 0) return 0xFFFF;

    (void)rx;
    (void)rz;

    bin = (w3ddmg_u8)(((w3ddmg_u8)ry) & 15);
    if (count >= 16) return model->edge_masks[(__safe_index w3ddmg_u8)bin];
    if (count == 8) bin = (w3ddmg_u8)(bin >> 1);
    else if (count == 4) bin = (w3ddmg_u8)(bin >> 2);
    else if (count == 2) bin = (w3ddmg_u8)(bin >> 3);
    else bin = 0;

    if (bin >= count) bin = (w3ddmg_u8)(count - 1);
    return model->edge_masks[(__safe_index w3ddmg_u8)bin];
}
#endif

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Combine the yaw mask with adjacent front-face visibility for up to 16
// edges. Missing/invalid face adjacency leaves an enabled mask bit visible.
// Caller-provided counts and tables must fit the shared scratch arrays.
static void w3ddmg_build_edge_flags(const Wire3DDMG_Model* model, w3ddmg_u8 edge_count, w3ddmg_u8 face_count, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz)
{
    w3ddmg_u8 i;
    w3ddmg_u16 edge_mask;
    w3ddmg_u16 bit;

    edge_mask = Wire3DDMG_SelectEdgeMask(model, rx, ry, rz);
    bit = 1;
    i = 0;
    while (i < edge_count)
    {
        w3ddmg_u8 visible;

        visible = 0;
        if ((edge_mask & bit) != 0)
        {
            visible = 1;
            if ((model->flags & WIRE3D_DMG_MODEL_HIDDEN_LINES) &&
                (model->faces != 0) &&
                (model->edge_faces != 0))
            {
                const Wire3DDMG_EdgeFaces* ef;
                w3ddmg_u8 any_face;

                ef = &model->edge_faces[(__safe_index w3ddmg_u8)i];
                any_face = 0;
                visible = 0;

                if (ef->f0 != WIRE3D_DMG_FACE_NONE)
                {
                    any_face = 1;
                    if ((ef->f0 >= face_count) ||
                        (w3ddmg_face_visible[(__safe_index w3ddmg_u8)ef->f0] != 0))
                    {
                        visible = 1;
                    }
                }

                if ((visible == 0) && (ef->f1 != WIRE3D_DMG_FACE_NONE))
                {
                    any_face = 1;
                    if ((ef->f1 >= face_count) ||
                        (w3ddmg_face_visible[(__safe_index w3ddmg_u8)ef->f1] != 0))
                    {
                        visible = 1;
                    }
                }

                if (any_face == 0) visible = 1;
            }
        }

        w3ddmg_edge_flags[(__safe_index w3ddmg_u8)i] = visible;
        bit = (w3ddmg_u16)(bit << 1);
        i = (w3ddmg_u8)(i + 1);
    }
}
#endif

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Mark bounding rectangles of front-facing hidden-line model faces using
// the most recently projected vertex cache. Call only after successfully drawing
// this same valid model; no transform is performed here.
static void w3ddmg_mark_model_occluder(const Wire3DDMG_Model* model)
{
    w3ddmg_u8 count;
    w3ddmg_u8 face_count;
    w3ddmg_u8 i;

    if (model == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3D_DMG_MODEL_HIDDEN_LINES) == 0) return;

    count = model->vertex_count;
    if (count > WIRE3D_DMG_MODEL_VERTEX_LIMIT) count = WIRE3D_DMG_MODEL_VERTEX_LIMIT;
    face_count = model->face_count;
    if (face_count > WIRE3D_DMG_MODEL_FACE_LIMIT) face_count = WIRE3D_DMG_MODEL_FACE_LIMIT;

    w3ddmg_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3DDMG_Face* f;
        f = &model->faces[(__safe_index w3ddmg_u8)i];
        if (w3ddmg_face_visible[(__safe_index w3ddmg_u8)i])
        {
            w3ddmg_mark_triangle(
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->a],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->a],
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->b],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->b],
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->c],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->c]);
        }
        i = (w3ddmg_u8)(i + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Return immediately if LCD is off. Otherwise wait out any current VBlank,
// then wait for the next one; this is a busy wait with no interrupt management.
void w3ddmg_wait_vblank_start()
{
    if ((w3ddmg_reg_lcdc & 0x80) == 0) return;
    while (w3ddmg_reg_ly >= 144) { }
    while (w3ddmg_reg_ly < 144) { }
}

#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 92
// Zero 24 pages (6144 bytes) from VRAM 0x8000. The caller must disable the
// LCD or otherwise provide unrestricted VRAM access; this loop does not poll STAT.
void w3ddmg_clear_vram_asm()
{
    __asm {
w3ddmgcv_enter:
        LD_HL_IMM w3ddmg_vram_tiles
        XOR_A
        LD_D_IMM 24

w3ddmgcv_page:
        LD_C_IMM 0

w3ddmgcv_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3ddmgcv_loop
        DEC_D
        JR_NZ w3ddmgcv_page
        RET
    }
}

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 94
// Fill all 1024 map entries at 0x9800 with HUD blank tile 0x8A.
// The caller provides unrestricted VRAM access; no LCD-state check occurs here.
void w3ddmg_fill_bg_map_asm()
{
    __asm {
w3ddmgfb_enter:
        LD_HL_IMM w3ddmg_bg_map_9800
        LD_A_IMM 0x8A
        LD_D_IMM 4

w3ddmgfb_page:
        LD_C_IMM 0

w3ddmgfb_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3ddmgfb_loop
        DEC_D
        JR_NZ w3ddmgfb_page
        RET
    }
}
#else
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 94
// Fill all 1024 entries at 0x9800 with tile 0x80. The caller must provide
// unrestricted VRAM access; initialization uses this while the LCD is off.
void w3ddmg_fill_bg_map_asm()
{
    __asm {
w3ddmgfb_enter:
        LD_HL_IMM w3ddmg_bg_map_9800
        LD_A_IMM 0x80
        LD_D_IMM 4

w3ddmgfb_page:
        LD_C_IMM 0

w3ddmgfb_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3ddmgfb_loop
        DEC_D
        JR_NZ w3ddmgfb_page
        RET
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 96
// Zero six 256-byte pages of occlusion storage without touching the stage.
void w3ddmg_clear_occlusion_mask_asm()
{
    __asm {
w3ddmgco_enter:
        LD_HL_IMM w3ddmg_occlusion_mask
        XOR_A
        LD_D_IMM 6

w3ddmgco_page:
        LD_C_IMM 0

w3ddmgco_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3ddmgco_loop
        DEC_D
        JR_NZ w3ddmgco_page
        RET
    }
}
#else
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 96
// Zero eight pages (2048 bytes) of mask storage; leave stage/dirty state unchanged.
void w3ddmg_clear_occlusion_mask_asm()
{
    __asm {
w3ddmgco_enter:
        LD_HL_IMM w3ddmg_occlusion_mask
        XOR_A
        LD_D_IMM 8

w3ddmgco_page:
        LD_C_IMM 0

w3ddmgco_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3ddmgco_loop
        DEC_D
        JR_NZ w3ddmgco_page
        RET
    }
}
#endif

#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 98
// Poll STAT until mode bit 1 is clear (mode 0 or 1), then perform one queued
// map write. Address/value come from shared globals. Interrupts are not masked,
// so the caller must prevent handlers from disrupting this access window.
void w3ddmg_put_bg_tile_safe_asm()
{
    __asm {
w3ddmgbg_enter:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ w3ddmgbg_enter
        LD_A_MEM w3ddmg_bgq_addr_hi
        LD_H_A
        LD_A_MEM w3ddmg_bgq_addr_lo
        LD_L_A
        LD_A_MEM w3ddmg_bgq_value
        LD_HL_A
        RET
    }
}

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 100
// Clear only the 96 used bytes in each of 16 pages at 0xD000..0xDFFF.
// Leave the unused page tails unchanged; this routine does not select a WRAM bank.
void w3ddmg_clear_stage_asm()
{
    __asm {
w3ddmgcs_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD0
        LD_C_IMM 16

w3ddmgcs_outer:
        LD_L_IMM 0
        LD_B_IMM 0x60

w3ddmgcs_inner:
        LDI_HL_A
        DEC_B
        JR_NZ w3ddmgcs_inner
        INC_H
        DEC_C
        JR_NZ w3ddmgcs_outer
        RET
    }
}
#else
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 100
// Zero 120 live bytes in each of 16 pages at 0xD000..0xDFFF. Unused tails
// remain untouched. This includes the aliased auxiliary source region; no bank
// is selected and dirty flags are not changed.
void w3ddmg_clear_stage_asm()
{
    __asm {
w3ddmgcs_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD0
        LD_C_IMM 16

w3ddmgcs_outer:
        LD_L_IMM 0
        LD_B_IMM 0x78

w3ddmgcs_inner:
        LDI_HL_A
        DEC_B
        JR_NZ w3ddmgcs_inner
        INC_H
        DEC_C
        JR_NZ w3ddmgcs_outer
        RET
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 110
// Bounds-check shared X/Y and OR the corresponding 1bpp bit into the stage.
// High address byte selects an eight-pixel column; low byte selects its scanline.
// The caller supplies the expected WRAM mapping; shared registers/state are clobbered.
void w3ddmg_plot_stage_asm()
{
    __asm {
w3ddmgps_enter:
        LD_A_MEM w3ddmg_plot_x
        CP_IMM 0x80
        JP_NC w3ddmgps_ret

        LD_A_MEM w3ddmg_plot_y
        CP_IMM 0x60
        JP_NC w3ddmgps_ret

        LD_A_MEM w3ddmg_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3ddmg_plot_tx

        LD_A_MEM w3ddmg_plot_tx
        LD_B_A
        LD_A_IMM 0xD0
        ADD_B
        LD_H_A

        LD_A_MEM w3ddmg_plot_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A
        ADD_A
        ADD_A
        LD_B_A

        LD_A_MEM w3ddmg_plot_y
        AND_IMM 7
        ADD_B
        LD_L_A

        LD_A_MEM w3ddmg_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3ddmg_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

        LD_A_HL
        OR_B
        LD_HL_A

w3ddmgps_ret:
        RET
    }
}
#else
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 110
// Bounds-check shared coordinates and OR a pixel into a column page of the
// 128x120 stage. The caller supplies the WRAM mapping and handles dirty marking;
// this low-level plotter does not update the dirty table.
void w3ddmg_plot_stage_asm()
{
    __asm {
w3ddmgps_enter:
        LD_A_MEM w3ddmg_plot_x
        CP_IMM 0x80
        JP_NC w3ddmgps_ret

        LD_A_MEM w3ddmg_plot_y
        CP_IMM 0x78
        JP_NC w3ddmgps_ret

        LD_A_MEM w3ddmg_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3ddmg_plot_tx

        LD_A_MEM w3ddmg_plot_tx
        LD_B_A
        LD_A_IMM 0xD0
        ADD_B
        LD_H_A

        LD_A_MEM w3ddmg_plot_y
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A
        ADD_A
        ADD_A
        LD_B_A

        LD_A_MEM w3ddmg_plot_y
        AND_IMM 7
        ADD_B
        LD_L_A

        LD_A_MEM w3ddmg_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3ddmg_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

        LD_A_HL
        OR_B
        LD_HL_A

w3ddmgps_ret:
        RET
    }
}
#endif

#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 120
// Rasterize the shared endpoints with separate shallow/steep integer paths.
// Draw an extra bridge pixel when advancing the minor axis to keep lines connected.
// The plotter rejects off-screen pixels; callers should use viewport endpoints.
// All working coordinates and error terms live in shared non-reentrant state.
void w3ddmg_line_stage_asm()
{
    __asm {
w3ddmgls_enter:
        LD_A_MEM w3ddmg_line_x0
        LD_B_A
        LD_A_MEM w3ddmg_line_x1
        CP_B
        JP_C w3ddmgls_x_reverse
        SUB_B
        LD_MEM_A w3ddmg_line_dx
        LD_A_IMM 1
        LD_MEM_A w3ddmg_line_sx
        JP w3ddmgls_y_start

w3ddmgls_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_dx
        LD_A_IMM 255
        LD_MEM_A w3ddmg_line_sx

w3ddmgls_y_start:
        LD_A_MEM w3ddmg_line_y0
        LD_B_A
        LD_A_MEM w3ddmg_line_y1
        CP_B
        JP_C w3ddmgls_y_reverse
        SUB_B
        LD_MEM_A w3ddmg_line_dy
        LD_A_IMM 1
        LD_MEM_A w3ddmg_line_sy
        JP w3ddmgls_branch

w3ddmgls_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_dy
        LD_A_IMM 255
        LD_MEM_A w3ddmg_line_sy

w3ddmgls_branch:
        LD_A_MEM w3ddmg_line_dx
        LD_B_A
        LD_A_MEM w3ddmg_line_dy
        CP_B
        JP_C w3ddmgls_shallow
        JP_Z w3ddmgls_shallow
        JP w3ddmgls_steep

// Advance X each iteration; add a bridge pixel when the minor Y step occurs.
w3ddmgls_shallow:
        LD_A_MEM w3ddmg_line_x0
        LD_MEM_A w3ddmg_line_x
        LD_A_MEM w3ddmg_line_y0
        LD_MEM_A w3ddmg_line_y
        LD_A_MEM w3ddmg_line_dx
        OR_A
        RRA
        LD_MEM_A w3ddmg_line_err

w3ddmgls_shallow_loop:
        LD_A_MEM w3ddmg_line_x
        LD_MEM_A w3ddmg_plot_x
        LD_A_MEM w3ddmg_line_y
        LD_MEM_A w3ddmg_plot_y
        CALL w3ddmg_plot_stage_asm

        LD_A_MEM w3ddmg_line_x
        LD_B_A
        LD_A_MEM w3ddmg_line_x1
        CP_B
        JP_Z w3ddmgls_done

        LD_A_MEM w3ddmg_line_dy
        LD_B_A
        LD_A_MEM w3ddmg_line_err
        CP_B
        JP_NC w3ddmgls_shallow_skip_bridge

        LD_A_MEM w3ddmg_line_y
        LD_B_A
        LD_A_MEM w3ddmg_line_sy
        ADD_B
        LD_MEM_A w3ddmg_line_y

        LD_A_MEM w3ddmg_line_x
        LD_MEM_A w3ddmg_plot_x
        LD_A_MEM w3ddmg_line_y
        LD_MEM_A w3ddmg_plot_y
        CALL w3ddmg_plot_stage_asm

        LD_A_MEM w3ddmg_line_dx
        LD_B_A
        LD_A_MEM w3ddmg_line_err
        ADD_B
        LD_MEM_A w3ddmg_line_err

w3ddmgls_shallow_skip_bridge:
        LD_A_MEM w3ddmg_line_err
        LD_B_A
        LD_A_MEM w3ddmg_line_dy
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_err

        LD_A_MEM w3ddmg_line_x
        LD_B_A
        LD_A_MEM w3ddmg_line_sx
        ADD_B
        LD_MEM_A w3ddmg_line_x
        JP w3ddmgls_shallow_loop

// Advance Y each iteration; add a bridge pixel when the minor X step occurs.
w3ddmgls_steep:
        LD_A_MEM w3ddmg_line_x0
        LD_MEM_A w3ddmg_line_x
        LD_A_MEM w3ddmg_line_y0
        LD_MEM_A w3ddmg_line_y
        LD_A_MEM w3ddmg_line_dy
        OR_A
        RRA
        LD_MEM_A w3ddmg_line_err

w3ddmgls_steep_loop:
        LD_A_MEM w3ddmg_line_x
        LD_MEM_A w3ddmg_plot_x
        LD_A_MEM w3ddmg_line_y
        LD_MEM_A w3ddmg_plot_y
        CALL w3ddmg_plot_stage_asm

        LD_A_MEM w3ddmg_line_y
        LD_B_A
        LD_A_MEM w3ddmg_line_y1
        CP_B
        JP_Z w3ddmgls_done

        LD_A_MEM w3ddmg_line_dx
        LD_B_A
        LD_A_MEM w3ddmg_line_err
        CP_B
        JP_NC w3ddmgls_steep_skip_bridge

        LD_A_MEM w3ddmg_line_x
        LD_B_A
        LD_A_MEM w3ddmg_line_sx
        ADD_B
        LD_MEM_A w3ddmg_line_x

        LD_A_MEM w3ddmg_line_x
        LD_MEM_A w3ddmg_plot_x
        LD_A_MEM w3ddmg_line_y
        LD_MEM_A w3ddmg_plot_y
        CALL w3ddmg_plot_stage_asm

        LD_A_MEM w3ddmg_line_dy
        LD_B_A
        LD_A_MEM w3ddmg_line_err
        ADD_B
        LD_MEM_A w3ddmg_line_err

w3ddmgls_steep_skip_bridge:
        LD_A_MEM w3ddmg_line_err
        LD_B_A
        LD_A_MEM w3ddmg_line_dx
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_err

        LD_A_MEM w3ddmg_line_y
        LD_B_A
        LD_A_MEM w3ddmg_line_sy
        ADD_B
        LD_MEM_A w3ddmg_line_y
        JP w3ddmgls_steep_loop

w3ddmgls_done:
        RET
    }
}


#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 121
// In-bounds connected lines keep the stage pointer, pixel mask, remaining
// count, minor delta and error in registers. Address calculation happens once;
// crossing an eight-pixel column adjusts H instead of recalculating a tile.
void w3ddmg_line_inside_asm()
{
    __asm {
w3ddmgfast_enter:
        LD_A_MEM w3ddmg_line_x0
        LD_B_A
        LD_A_MEM w3ddmg_line_x1
        CP_B
        JP_C w3ddmgfast_x_reverse
        SUB_B
        LD_MEM_A w3ddmg_line_dx
        LD_A_IMM 1
        LD_MEM_A w3ddmg_line_sx
        JP w3ddmgfast_y_start

w3ddmgfast_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_dx
        LD_A_IMM 255
        LD_MEM_A w3ddmg_line_sx

w3ddmgfast_y_start:
        LD_A_MEM w3ddmg_line_y0
        LD_B_A
        LD_A_MEM w3ddmg_line_y1
        CP_B
        JP_C w3ddmgfast_y_reverse
        SUB_B
        LD_MEM_A w3ddmg_line_dy
        LD_A_IMM 1
        LD_MEM_A w3ddmg_line_sy
        JP w3ddmgfast_branch

w3ddmgfast_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3ddmg_line_dy
        LD_A_IMM 255
        LD_MEM_A w3ddmg_line_sy

w3ddmgfast_branch:
        LD_A_MEM w3ddmg_line_x0
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        LD_HL_IMM w3ddmg_bit_mask
        ADD_HL_DE
        LD_B_HL
        LD_A_MEM w3ddmg_line_x0
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        ADD_A_IMM 0xD0
        LD_H_A
        LD_A_MEM w3ddmg_line_y0
        LD_L_A
        LD_A_MEM w3ddmg_line_dx
        LD_C_A
        LD_A_MEM w3ddmg_line_dy
        CP_C
        JP_C w3ddmgfast_shallow
        JP_Z w3ddmgfast_shallow
        JP w3ddmgfast_steep
w3ddmgfast_shallow:
        LD_A_MEM w3ddmg_line_dy
        LD_D_A
        LD_A_MEM w3ddmg_line_dx
        LD_C_A
        OR_A
        RRA
        LD_E_A
        INC_C
w3ddmgfast_shallow_loop:
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_C
        RET_Z
        LD_A_E
        CP_D
        JP_NC w3ddmgfast_shallow_skip
        CALL w3ddmgfast_step_y
        LD_A_HL
        OR_B
        LD_HL_A
        LD_A_MEM w3ddmg_line_dx
        ADD_E
        LD_E_A
w3ddmgfast_shallow_skip:
        LD_A_E
        SUB_D
        LD_E_A
        CALL w3ddmgfast_step_x
        JP w3ddmgfast_shallow_loop
w3ddmgfast_steep:
        LD_A_MEM w3ddmg_line_dx
        LD_D_A
        LD_A_MEM w3ddmg_line_dy
        LD_C_A
        OR_A
        RRA
        LD_E_A
        INC_C
w3ddmgfast_steep_loop:
        LD_A_HL
        OR_B
        LD_HL_A
        DEC_C
        RET_Z
        LD_A_E
        CP_D
        JP_NC w3ddmgfast_steep_skip
        CALL w3ddmgfast_step_x
        LD_A_HL
        OR_B
        LD_HL_A
        LD_A_MEM w3ddmg_line_dy
        ADD_E
        LD_E_A
w3ddmgfast_steep_skip:
        LD_A_E
        SUB_D
        LD_E_A
        CALL w3ddmgfast_step_y
        JP w3ddmgfast_steep_loop
w3ddmgfast_step_x:
        LD_A_MEM w3ddmg_line_sx
        CP_IMM 1
        JR_Z w3ddmgfast_right
        LD_A_B
        RLCA
        LD_B_A
        RET_NC
        DEC_H
        RET
w3ddmgfast_right:
        LD_A_B
        RRCA
        LD_B_A
        RET_NC
        INC_H
        RET
w3ddmgfast_step_y:
        LD_A_MEM w3ddmg_line_sy
        INC_A
        JR_Z w3ddmgfast_up
        INC_L
        RET
w3ddmgfast_up:
        DEC_L
        RET
    }
}

#if WIRE3D_DMG_HEIGHT == 96
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 130
// Copy 1536 staged row bytes into the high bitplane of tiles at 0x8900,
// leaving their low bitplanes unchanged. Transfer two source bytes per STAT
// access window and consume each pair while copying. No bank or interrupt state
// is saved; the caller must preserve the expected mapping and access timing.
// Paired uploads keep the source in HL and destination in DE. B/C hold
// two pixels while their source bytes are consumed. Unrolled pairs reduce
// loop overhead; STAT is checked before each pair. This uses more ROM
// bytes in exchange for lower transfer time, without extra RAM or DMA.
void w3ddmg_transfer_stage_asm()
{
    __asm {
        LD_HL_IMM w3ddmg_stage
        LD_DE_IMM 0x8901
w3ddmg_pair_block:
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_0:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_0
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_1:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_1
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_2:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_2
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_3:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_3
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_4:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_4
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_5:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_5
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_6:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_6
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_7:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_7
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_8:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_8
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_9:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_9
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_10:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_10
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_11:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_11
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_12:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_12
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_13:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_13
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_14:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_14
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_wait_15:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_wait_15
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_L
        CP_IMM 96
        JP_C w3ddmg_pair_block
        INC_H
        LD_L_IMM 0
        LD_A_H
        CP_IMM 0xE0
        JP_C w3ddmg_pair_block
        RET
    }
}
#else
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 130
// Transfer all 1920 live source bytes into the high bitplanes of 240 tiles
// at 0x8900..0x97FF, consuming each source pair while copying. Low bitplanes
// remain unchanged. Poll STAT before each two-byte write window without masking
// interrupts or changing banks; dirty tables are not updated.
// Paired uploads keep the source in HL and destination in DE. B/C hold
// two pixels while their source bytes are consumed. Unrolled pairs reduce
// loop overhead; STAT is checked before each pair. This uses more ROM
// bytes in exchange for lower transfer time, without extra RAM or DMA.
void w3ddmg_transfer_stage_asm()
{
    __asm {
        LD_HL_IMM w3ddmg_stage
        LD_DE_IMM 0x8901
w3ddmg_pair_main_block:
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_0:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_0
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_1:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_1
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_2:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_2
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_3:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_3
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_4:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_4
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_5:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_5
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_6:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_6
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_7:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_7
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_8:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_8
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_9:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_9
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_10:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_10
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_11:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_11
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_12:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_12
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_13:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_13
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_main_wait_14:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_main_wait_14
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_L
        CP_IMM 120
        JP_C w3ddmg_pair_main_block
        INC_H
        LD_L_IMM 0
        LD_A_H
        CP_IMM 0xE0
        JP_C w3ddmg_pair_main_block
        RET
    }
}
#endif

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Take over the LCD: wait for VBlank, switch it off, clear 6144 tile bytes and
// the BG map, install HUD tiles and the 128x96 signed-tile viewport, then enable
// BG display with scroll zero and BGP=0xB4. Reset camera, BG queue and stage.
// Reserve 0xD000..0xDFFF for staging and retain its WRAM bank mapping.
// Call BeginFrame before drawing to initialize the occlusion state as well.
#else
// Take over LCD state, clear tile VRAM and the BG map, and install a 16x15
// viewport beginning at map tile (3,1). Set scroll (8,0), BGP=0xB4 and BG display.
// Reset camera, queues, transfer modes, both dirty tables and stage. Reserve
// 0xD000..0xDFFF and retain its WRAM mapping; auxiliary storage aliases it.
// Call BeginFrame before drawing to establish occlusion state.
#endif
void Wire3DDMG_Init()
{
    w3ddmg_u8 row;
    w3ddmg_u8 col;
    w3ddmg_u8 tid;
    w3ddmg_u16 off;

    w3ddmg_wait_vblank_start();
    w3ddmg_reg_lcdc = 0;

    w3ddmg_clear_vram_asm();
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_load_hud_tiles();
#endif
    w3ddmg_fill_bg_map_asm();

    row = 0;
    tid = 0x90;
    while (row < WIRE3D_DMG_TILE_H)
    {
#if WIRE3D_DMG_HEIGHT == 96
        off = (w3ddmg_u16)((w3ddmg_u16)row << 5);
#else
        off = (w3ddmg_u16)(0x23 + ((w3ddmg_u16)row << 5));
#endif
        col = 0;
        while (col < WIRE3D_DMG_TILE_W)
        {
            w3ddmg_bg_map_9800[(w3ddmg_u16)(off + (w3ddmg_u16)col)] = tid;
            tid = (w3ddmg_u8)(tid + WIRE3D_DMG_TILE_H);
            col = (w3ddmg_u8)(col + 1);
        }
        row = (w3ddmg_u8)(row + 1);
        tid = (w3ddmg_u8)(0x90 + row);
    }

#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_reg_scx = 0;
#else
    w3ddmg_reg_scx = 8;
#endif
    w3ddmg_reg_scy = 0;
    w3ddmg_reg_bgp = 0xB4;
    w3ddmg_reg_lcdc = 0x81;
    w3ddmg_bgq_count = 0;
#if WIRE3D_DMG_HEIGHT == 120
    w3ddmg_aux_transfer_enabled = 0;
    w3ddmg_dirty_transfer_enabled = 0;
#endif

    Wire3DDMG_SetCamera(0, 0, 0, 0, 0, 0);
    w3ddmg_clear_stage_asm();
#if WIRE3D_DMG_HEIGHT == 120
    w3ddmg_clear_stage_aux_asm();
    w3ddmg_clear_dirty_tables();
#endif
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Clear staged pixels and occlusion mask and disable occlusion filtering.
// This leaves the displayed VRAM frame and pending BG tile queue unchanged.
#else
// Disable occlusion filtering only. Stage clearing normally happens during
// transfer, and Init supplies the first clean stage. This does not discard an
// unsubmitted frame, clear the mask, or reset pending BG writes/dirty flags.
#endif
void Wire3DDMG_BeginFrame()
{
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_clear_stage_asm();
    w3ddmg_clear_occlusion_mask();
#else
    /* The consuming stage transfer clears D000 as it uploads, so BeginFrame only
       resets draw state. Wire3DDMG_Init supplies the first clean buffer. */
#endif
    w3ddmg_occlusion_active = 0;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Store camera position and angles for later draws; a full turn is 16 angle
// steps. No existing drawing is reprojected and no input range validation occurs.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 pitch, w3ddmg_i8 yaw, w3ddmg_i8 roll)
#else
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 pitch, w3ddmg_u8 yaw, w3ddmg_u8 roll)
#endif
{
    w3ddmg_cam_x = x;
    w3ddmg_cam_y = y;
    w3ddmg_cam_z = z;
    w3ddmg_cam_pitch = pitch;
    w3ddmg_cam_yaw = yaw;
    w3ddmg_cam_roll = roll;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Draw a connected line into the 128x96 stage, including endpoints. During
// DrawScene, use the coarse five-sample occlusion rejection; otherwise draw directly.
// Supply viewport coordinates. VRAM is updated only by EndFrame.
#else
// Mark the bounding tile rectangle when dirty transfer is enabled, then draw
// a connected staged line. Scene drawing may reject it using five mask samples.
// Supply 128x120 viewport endpoints; low-level pixel bounds checks do not clip
// the segment geometrically.
#endif
void Wire3DDMG_DrawLine2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by)
{
#if WIRE3D_DMG_HEIGHT == 120
    w3ddmg_mark_line_dirty(ax, ay, bx, by);

#endif
    if (w3ddmg_occlusion_active)
    {
        w3ddmg_line_stage_masked_c(ax, ay, bx, by);
        return;
    }

    w3ddmg_line_x0 = ax;
    w3ddmg_line_y0 = ay;
    w3ddmg_line_x1 = bx;
    w3ddmg_line_y1 = by;
    if (ax < 128 && bx < 128 && ay < WIRE3D_DMG_SCREEN_H && by < WIRE3D_DMG_SCREEN_H)
        w3ddmg_line_inside_asm();
    else
        w3ddmg_line_stage_asm();
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Process at most the first eight objects, sorted near to far by transformed
// origin depth. Draw visible valid models, then mark their front-face bounding
// rectangles to reject fully sampled later lines. This approximates inter-object
// occlusion, not a depth buffer. It clears its mask but retains existing staged
// pixels. Null arrays are ignored; objects and their data must remain readable.
#else
// Sort at most eight object origins by camera depth, near to far. Draw visible
// valid models and mark front-face bounding rectangles for later objects. The
// first sorted slot bypasses mask tests and the last needs no new occluder.
// This is coarse line rejection, not a depth buffer. Null/empty input is ignored;
// caller data and the current projection cache must remain valid during traversal.
#endif
void Wire3DDMG_DrawScene(Wire3DDMG_Object* objects, w3ddmg_u8 count)
{
    w3ddmg_u8 scene_count;
    w3ddmg_u8 i;

    if (objects == 0) return;

    scene_count = count;
    if (scene_count > WIRE3D_DMG_SCENE_OBJECT_LIMIT) scene_count = WIRE3D_DMG_SCENE_OBJECT_LIMIT;
#if WIRE3D_DMG_HEIGHT == 120
    if (scene_count == 0) return;
#endif

    i = 0;
    while (i < scene_count)
    {
        w3ddmg_scene_order[(__safe_index w3ddmg_u8)i] = i;
        w3ddmg_scene_depth[(__safe_index w3ddmg_u8)i] = w3ddmg_scene_object_depth(&objects[(__safe_index w3ddmg_u8)i]);
        i = (w3ddmg_u8)(i + 1);
    }

    i = 0;
    while (i < scene_count)
    {
        w3ddmg_u8 j;
        j = (w3ddmg_u8)(i + 1);
        while (j < scene_count)
        {
            w3ddmg_u8 oi;
            w3ddmg_u8 oj;
            oi = w3ddmg_scene_order[(__safe_index w3ddmg_u8)i];
            oj = w3ddmg_scene_order[(__safe_index w3ddmg_u8)j];

#if WIRE3D_DMG_HEIGHT == 96
            // Put the nearer origin first so its face bounds can suppress later lines.
            // The array itself stays in caller order; only the scratch index list is sorted.
#endif
            if (w3ddmg_scene_depth[(__safe_index w3ddmg_u8)oj] < w3ddmg_scene_depth[(__safe_index w3ddmg_u8)oi])
            {
                w3ddmg_scene_order[(__safe_index w3ddmg_u8)i] = oj;
                w3ddmg_scene_order[(__safe_index w3ddmg_u8)j] = oi;
            }
            j = (w3ddmg_u8)(j + 1);
        }
        i = (w3ddmg_u8)(i + 1);
    }

    w3ddmg_clear_occlusion_mask();
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_occlusion_active = 1;
#else
    w3ddmg_occlusion_active = 0;
#endif

    i = 0;
    while (i < scene_count)
    {
        Wire3DDMG_Object* obj;
        obj = &objects[(__safe_index w3ddmg_u8)w3ddmg_scene_order[(__safe_index w3ddmg_u8)i]];
        if ((obj->visible != 0) && (obj->model != 0))
        {
#if WIRE3D_DMG_HEIGHT == 96
            Wire3DDMG_DrawModelScaled(obj->model, obj->x, obj->y, obj->z, (w3ddmg_i8)obj->rx, (w3ddmg_i8)obj->ry, (w3ddmg_i8)obj->rz, obj->scale_q8);
            w3ddmg_mark_model_occluder(obj->model);
#else
            w3ddmg_occlusion_active = i == 0 ? 0 : 1;
            Wire3DDMG_DrawModelScaled(obj->model, obj->x, obj->y, obj->z, obj->rx, obj->ry, obj->rz, obj->scale_q8);
            if ((w3ddmg_u8)(i + 1) < scene_count)
            {
                w3ddmg_mark_model_occluder(obj->model);
            }
#endif
        }
        i = (w3ddmg_u8)(i + 1);
    }

    w3ddmg_occlusion_active = 0;
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Project both world endpoints and draw only when both pass the depth range.
// Do not intersect a crossing segment with near/far planes; projected endpoints
// are independently clamped to the screen edges.
void Wire3DDMG_DrawLine3D(w3ddmg_i16 ax, w3ddmg_i16 ay, w3ddmg_i16 az, w3ddmg_i16 bx, w3ddmg_i16 by, w3ddmg_i16 bz)
{
    w3ddmg_u8 sx0;
    w3ddmg_u8 sy0;
    w3ddmg_u8 sx1;
    w3ddmg_u8 sy1;

    if (w3ddmg_project_world(ax, ay, az, &sx0, &sy0) == 0) return;
    if (w3ddmg_project_world(bx, by, bz, &sx1, &sy1) == 0) return;
    Wire3DDMG_DrawLine2D(sx0, sy0, sx1, sy1);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Draw an unscaled model by delegating to DrawModelScaled with scale 256.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz)
#else
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz)
#endif
{
    Wire3DDMG_DrawModelScaled(model, x, y, z, rx, ry, rz, 256);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Project up to 24 vertices and erase front-facing triangle interiors from
// the stage for a model with hidden-line flags and face data. Nonpositive scale
// means 256 (unity). Up to 16 faces are considered; depth-crossing faces are
// skipped. This also replaces shared projection caches, but does not update VRAM.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8)
#else
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8)
#endif
{
    w3ddmg_u8 i;
    w3ddmg_u8 count;
    w3ddmg_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3D_DMG_MODEL_HIDDEN_LINES) == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3D_DMG_MODEL_VERTEX_LIMIT) count = WIRE3D_DMG_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const Wire3DDMG_Vec3* v;
        w3ddmg_i16 vx;
        w3ddmg_i16 vy;
        w3ddmg_i16 vz;
        w3ddmg_u8 sx;
        w3ddmg_u8 sy;

        v = &model->vertices[(__safe_index w3ddmg_u8)i];
        vx = (w3ddmg_i16)((v->x * scale_q8) >> 8);
        vy = (w3ddmg_i16)((v->y * scale_q8) >> 8);
        vz = (w3ddmg_i16)((v->z * scale_q8) >> 8);

#if WIRE3D_DMG_HEIGHT == 96
        w3ddmg_rotate_y(&vx, &vz, (w3ddmg_i8)ry);
        w3ddmg_rotate_x(&vy, &vz, (w3ddmg_i8)rx);
        w3ddmg_rotate_z(&vx, &vy, (w3ddmg_i8)rz);
#else
        w3ddmg_rotate_y(&vx, &vz, ry);
        w3ddmg_rotate_x(&vy, &vz, rx);
        w3ddmg_rotate_z(&vx, &vy, rz);
#endif

        vx = (w3ddmg_i16)(vx + x);
        vy = (w3ddmg_i16)(vy + y);
        vz = (w3ddmg_i16)(vz + z);

        if (w3ddmg_project_world(vx, vy, vz, &sx, &sy))
        {
            w3ddmg_screen_x[(__safe_index w3ddmg_u8)i] = sx;
            w3ddmg_screen_y[(__safe_index w3ddmg_u8)i] = sy;
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 1;
        }
        else
        {
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 0;
        }
        i = (w3ddmg_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3D_DMG_MODEL_FACE_LIMIT) face_count = WIRE3D_DMG_MODEL_FACE_LIMIT;
    w3ddmg_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3DDMG_Face* f;
        f = &model->faces[(__safe_index w3ddmg_u8)i];
        if (w3ddmg_face_visible[(__safe_index w3ddmg_u8)i] &&
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->a] &&
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->b] &&
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)f->c])
        {
            w3ddmg_clear_triangle(
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->a],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->a],
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->b],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->b],
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)f->c],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)f->c]);
        }
        i = (w3ddmg_u8)(i + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Erase a triangle from staged pixels using integer scanline intersections.
// Supply viewport coordinates. Degenerate triangles are ignored; the occlusion
// mask and VRAM are unchanged.
void Wire3DDMG_EraseTriangle2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy)
{
    w3ddmg_clear_triangle(ax, ay, bx, by, cx, cy);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Order the X endpoints and erase the inclusive staged span. Both endpoints
// must be in 0..127; invalid Y is ignored. The routine does not clip X for you.
void Wire3DDMG_EraseSpan2D(w3ddmg_u8 y, w3ddmg_u8 x0, w3ddmg_u8 x1)
{
    w3ddmg_u8 tx;

    if (x0 > x1)
    {
        tx = x0;
        x0 = x1;
        x1 = tx;
    }
    w3ddmg_stage_clear_span(y, x0, x1);
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Scale, rotate in Y/X/Z order, translate and project at most 24 vertices;
// then draw up to 16 edges filtered by yaw masks and optional front-facing faces.
// Nonpositive scale becomes unity (256). Null model/vertex/edge pointers are
// ignored; referenced tables must remain readable. Keep scale products and
// translations in signed 16-bit range. Depth-crossing edges are dropped, not clipped.
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8)
#else
// Scale up to 24 vertices, rotate Y/X/Z, translate and project, then traverse
// all edge_count entries (a byte count) with optional adjacent-face filtering.
// Face scratch is capped at 16; edge tables must contain all declared entries.
// Nonpositive scale means 256. Null model/vertex/edge pointers are ignored.
// Keep products/translations in signed 16-bit range; depth-crossing edges are dropped.
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8)
#endif
{
    w3ddmg_u8 i;
    w3ddmg_u8 count;
    w3ddmg_u8 edge_count;
    w3ddmg_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->edges == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3D_DMG_MODEL_VERTEX_LIMIT) count = WIRE3D_DMG_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const Wire3DDMG_Vec3* v;
        w3ddmg_i16 vx;
        w3ddmg_i16 vy;
        w3ddmg_i16 vz;
        w3ddmg_u8 sx;
        w3ddmg_u8 sy;

        v = &model->vertices[(__safe_index w3ddmg_u8)i];
        vx = (w3ddmg_i16)((v->x * scale_q8) >> 8);
        vy = (w3ddmg_i16)((v->y * scale_q8) >> 8);
        vz = (w3ddmg_i16)((v->z * scale_q8) >> 8);

#if WIRE3D_DMG_HEIGHT == 96
        w3ddmg_rotate_y(&vx, &vz, (w3ddmg_i8)ry);
        w3ddmg_rotate_x(&vy, &vz, (w3ddmg_i8)rx);
        w3ddmg_rotate_z(&vx, &vy, (w3ddmg_i8)rz);
#else
        w3ddmg_rotate_y(&vx, &vz, ry);
        w3ddmg_rotate_x(&vy, &vz, rx);
        w3ddmg_rotate_z(&vx, &vy, rz);
#endif

        vx = (w3ddmg_i16)(vx + x);
        vy = (w3ddmg_i16)(vy + y);
        vz = (w3ddmg_i16)(vz + z);

        if (w3ddmg_project_world(vx, vy, vz, &sx, &sy))
        {
            w3ddmg_screen_x[(__safe_index w3ddmg_u8)i] = sx;
            w3ddmg_screen_y[(__safe_index w3ddmg_u8)i] = sy;
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 1;
        }
        else
        {
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 0;
        }
        i = (w3ddmg_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3D_DMG_MODEL_FACE_LIMIT) face_count = WIRE3D_DMG_MODEL_FACE_LIMIT;
    if ((model->flags & WIRE3D_DMG_MODEL_HIDDEN_LINES) && (model->faces != 0) && (model->edge_faces != 0))
    {
        w3ddmg_build_face_visibility(model, count, face_count);
    }

    edge_count = model->edge_count;
#if WIRE3D_DMG_HEIGHT == 96
    if (edge_count > WIRE3D_DMG_MODEL_EDGE_LIMIT) edge_count = WIRE3D_DMG_MODEL_EDGE_LIMIT;
    w3ddmg_build_edge_flags(model, edge_count, face_count, rx, ry, rz);
#endif
    i = 0;
    while (i < edge_count)
    {
        const Wire3DDMG_Edge* e;
        e = &model->edges[(__safe_index w3ddmg_u8)i];
        if ((e->a < count) && (e->b < count))
        {
            if (w3ddmg_screen_visible[(__safe_index w3ddmg_u8)e->a] &&
                w3ddmg_screen_visible[(__safe_index w3ddmg_u8)e->b] &&
#if WIRE3D_DMG_HEIGHT == 96
                w3ddmg_edge_flags[(__safe_index w3ddmg_u8)i])
#else
                w3ddmg_is_edge_visible(model, i, face_count))
#endif
            {
                Wire3DDMG_DrawLine2D(
                    w3ddmg_screen_x[(__safe_index w3ddmg_u8)e->a],
                    w3ddmg_screen_y[(__safe_index w3ddmg_u8)e->a],
                    w3ddmg_screen_x[(__safe_index w3ddmg_u8)e->b],
                    w3ddmg_screen_y[(__safe_index w3ddmg_u8)e->b]);
            }
        }
        i = (w3ddmg_u8)(i + 1);
    }
}

#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
#if WIRE3D_DMG_HEIGHT == 96
// Wait for the next VBlank, flush pending BG map writes, then copy the stage
// to VRAM while polling access windows. Copying consumes the staged pixels.
// The transfer may extend beyond VBlank; this is not an atomic frame swap.
#else
// Wait for the next VBlank and flush BG writes, then optionally consume the
// auxiliary strips before uploading either dirty tiles or the entire main stage.
// Auxiliary/main source overlap makes that ordering significant. STAT polling
// continues across access windows; the update need not fit one VBlank.
#endif
void Wire3DDMG_EndFrame()
{
    w3ddmg_wait_vblank_start();
    w3ddmg_flush_bg_queue();
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_transfer_stage_asm();
#else
    if (w3ddmg_aux_transfer_enabled)
    {
        w3ddmg_transfer_stage_aux_asm();
    }
    if (w3ddmg_dirty_transfer_enabled)
    {
        w3ddmg_transfer_dirty_stage();
    }
    else
    {
        w3ddmg_transfer_stage_asm();
    }
#endif
}

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Mask an unsigned angle into the fixed sixteen-entry orientation table.
static w3ddmg_u8 w3ddmg_angle_index(w3ddmg_u8 v)
{
    return (w3ddmg_u8)(v & 15);
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Rotate in place in Y/X/Z order using 16 steps per full turn. A null
// component pointer makes the whole call a no-op; use distinct pointers.
// Each axis rotation clamps its inputs to +/-220.
void Wire3DDMG_RotatePoint(w3ddmg_i16* x, w3ddmg_i16* y, w3ddmg_i16* z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz)
{
    if (x == 0) return;
    if (y == 0) return;
    if (z == 0) return;

    w3ddmg_rotate_y(x, z, ry);
    w3ddmg_rotate_x(y, z, rx);
    w3ddmg_rotate_z(x, y, rz);
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Reset both current and previous frame tile flags. This does not clear
// VRAM, so forgetting previous dirty tiles can leave an old displayed image.
static void w3ddmg_clear_dirty_tables()
{
    w3ddmg_u8 i;

    i = 0;
    while (i < 240)
    {
        w3ddmg_dirty_tiles[(__safe_index w3ddmg_u8)i] = 0;
        w3ddmg_prev_dirty_tiles[(__safe_index w3ddmg_u8)i] = 0;
        i = (w3ddmg_u8)(i + 1);
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Flag one tile in the column-major 16x15 grid. Invalid coordinates are
// ignored; this low-level helper does not test the dirty-transfer enable flag.
static void w3ddmg_mark_tile_dirty(w3ddmg_u8 tx, w3ddmg_u8 ty)
{
    w3ddmg_u8 idx;

    if (tx >= WIRE3D_DMG_TILE_W) return;
    if (ty >= WIRE3D_DMG_TILE_H) return;
    idx = (w3ddmg_u8)((w3ddmg_u8)(tx * WIRE3D_DMG_TILE_H) + ty);
    w3ddmg_dirty_tiles[(__safe_index w3ddmg_u8)idx] = 1;
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// When dirty transfer is enabled, clamp and order pixel endpoints, then mark
// every tile in their inclusive bounding rectangle. Even a wholly off-screen
// byte-coordinate rectangle is pinned to an edge rather than rejected.
static void w3ddmg_mark_rect_dirty(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1)
{
    w3ddmg_u8 tx;
    w3ddmg_u8 ty;
    w3ddmg_u8 tx0;
    w3ddmg_u8 tx1;
    w3ddmg_u8 ty0;
    w3ddmg_u8 ty1;

    if (w3ddmg_dirty_transfer_enabled == 0) return;
    if (x0 >= WIRE3D_DMG_SCREEN_W) x0 = (w3ddmg_u8)(WIRE3D_DMG_SCREEN_W - 1);
    if (x1 >= WIRE3D_DMG_SCREEN_W) x1 = (w3ddmg_u8)(WIRE3D_DMG_SCREEN_W - 1);
    if (y0 >= WIRE3D_DMG_SCREEN_H) y0 = (w3ddmg_u8)(WIRE3D_DMG_SCREEN_H - 1);
    if (y1 >= WIRE3D_DMG_SCREEN_H) y1 = (w3ddmg_u8)(WIRE3D_DMG_SCREEN_H - 1);
    if (x0 > x1)
    {
        tx = x0;
        x0 = x1;
        x1 = tx;
    }
    if (y0 > y1)
    {
        ty = y0;
        y0 = y1;
        y1 = ty;
    }

    tx0 = (w3ddmg_u8)(x0 >> 3);
    tx1 = (w3ddmg_u8)(x1 >> 3);
    ty0 = (w3ddmg_u8)(y0 >> 3);
    ty1 = (w3ddmg_u8)(y1 >> 3);

    tx = tx0;
    while (tx <= tx1)
    {
        ty = ty0;
        while (ty <= ty1)
        {
            w3ddmg_mark_tile_dirty(tx, ty);
            if (ty == ty1) break;
            ty = (w3ddmg_u8)(ty + 1);
        }
        if (tx == tx1) break;
        tx = (w3ddmg_u8)(tx + 1);
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Mark the line's bounding tile rectangle, not just tiles crossed by the line.
static void w3ddmg_mark_line_dirty(w3ddmg_u8 x0, w3ddmg_u8 y0, w3ddmg_u8 x1, w3ddmg_u8 y1)
{
    w3ddmg_mark_rect_dirty(x0, y0, x1, y1);
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 132
// Copy eight contiguous stage bytes to alternating destination addresses
// (one tile bitplane), clearing each source byte. Source/destination come from
// shared globals; poll STAT per byte and leave dirty-table bookkeeping to C.
void w3ddmg_transfer_dirty_tile_asm()
{
    __asm {
w3ddmgdt_enter:
        LD_A_MEM w3ddmg_dirty_src_hi
        LD_H_A
        LD_A_MEM w3ddmg_dirty_src_lo
        LD_L_A
        LD_A_MEM w3ddmg_dirty_dst_hi
        LD_D_A
        LD_A_MEM w3ddmg_dirty_dst_lo
        LD_E_A
        LD_B_IMM 8

w3ddmgdt_loop:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ w3ddmgdt_loop

        LD_A_HL
        LD_DE_A
        XOR_A
        LD_HL_A
        INC_HL
        INC_DE
        INC_DE
        DEC_B
        JR_NZ w3ddmgdt_loop
        RET
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 133
// Clear offsets 0x18..0x4F in each page 0xD5..0xDA: 336 bytes total.
// These bytes are part of the existing main stage. No bank selection or
// dirty marking occurs.
void w3ddmg_clear_stage_aux_asm()
{
    __asm {
w3ddmgac_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD5
        LD_C_IMM 6

w3ddmgac_row:
        LD_L_IMM 0x18
        LD_B_IMM 0x38

w3ddmgac_col:
        LDI_HL_A
        DEC_B
        JR_NZ w3ddmgac_col
        INC_H
        DEC_C
        JR_NZ w3ddmgac_row
        RET
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank 1
#pragma fixed_order 134
// Transfer six 56-byte source strips at 0xD518 plus page strides into high
// bitplane addresses starting at 0x90A1 with 0xB0 destination stride. Consume
// the source as it is copied. Both source and destination overlap main-renderer
// regions; this is not an independent framebuffer or atomic overlay.
void w3ddmg_transfer_stage_aux_asm()
{
    __asm {
        LD_HL_IMM 0xD518
        LD_DE_IMM 0x90A1
w3ddmg_pair_aux_block:
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_0:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_0
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_1:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_1
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_2:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_2
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_3:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_3
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_4:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_4
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_5:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_5
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_6:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_6
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_7:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_7
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_8:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_8
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_9:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_9
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_10:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_10
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_11:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_11
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_12:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_12
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_13:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_13
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_14:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_14
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_15:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_15
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_16:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_16
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_17:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_17
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_18:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_18
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_19:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_19
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_20:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_20
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_21:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_21
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_22:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_22
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_23:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_23
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_24:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_24
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_25:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_25
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_26:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_26
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        XOR_A
        LD_B_HL
        LDI_HL_A
        LD_C_HL
        LDI_HL_A
w3ddmg_pair_aux_wait_27:
        LDH_A_MEM 65
        AND_IMM 2
        JR_NZ w3ddmg_pair_aux_wait_27
        LD_A_B
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_C
        LD_DE_A
        INC_DE
        INC_DE
        LD_A_H
        CP_IMM 0xDA
        JR_Z w3ddmg_pair_aux_done
        INC_H
        LD_L_IMM 0x18
        LD_A_E
        ADD_A_IMM 64
        LD_E_A
        LD_A_D
        ADC_IMM 0
        LD_D_A
        JP w3ddmg_pair_aux_block
w3ddmg_pair_aux_done:
        RET
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Draw byte-pair edge indices after integer scaling and translation; scale
// zero means one, and there is no object rotation or face filtering. Project at
// most 24 vertices. Supply at most 128 edge pairs because byte offsets wrap
// after 255. Invalid vertex IDs/depth-rejected endpoints are skipped; null arrays
// are ignored. Keep coordinate products/sums representable as s16.
void Wire3DDMG_DrawIndexedEdges(const Wire3DDMG_Vec3* vertices, w3ddmg_u8 vertex_count, const w3ddmg_u8* edges, w3ddmg_u8 edge_count, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 scale)
{
    w3ddmg_u8 i;
    w3ddmg_u8 count;

    if (vertices == 0) return;
    if (edges == 0) return;

    count = vertex_count;
    if (count > WIRE3D_DMG_MODEL_VERTEX_LIMIT) count = WIRE3D_DMG_MODEL_VERTEX_LIMIT;
    if (scale == 0) scale = 1;

    i = 0;
    while (i < count)
    {
        const Wire3DDMG_Vec3* v;
        w3ddmg_i16 vx;
        w3ddmg_i16 vy;
        w3ddmg_i16 vz;
        w3ddmg_u8 sx;
        w3ddmg_u8 sy;

        v = &vertices[(__safe_index w3ddmg_u8)i];
        vx = (w3ddmg_i16)(x + (w3ddmg_i16)(v->x * scale));
        vy = (w3ddmg_i16)(y + (w3ddmg_i16)(v->y * scale));
        vz = (w3ddmg_i16)(z + (w3ddmg_i16)(v->z * scale));

        if (w3ddmg_project_world(vx, vy, vz, &sx, &sy))
        {
            w3ddmg_screen_x[(__safe_index w3ddmg_u8)i] = sx;
            w3ddmg_screen_y[(__safe_index w3ddmg_u8)i] = sy;
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 1;
        }
        else
        {
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)i] = 0;
        }

        i = (w3ddmg_u8)(i + 1);
    }

    i = 0;
    while (i < edge_count)
    {
        w3ddmg_u8 a;
        w3ddmg_u8 b;

        a = edges[(__safe_index w3ddmg_u8)((w3ddmg_u8)(i << 1))];
        b = edges[(__safe_index w3ddmg_u8)((w3ddmg_u8)((i << 1) + 1))];
        if ((a < count) && (b < count) &&
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)a] &&
            w3ddmg_screen_visible[(__safe_index w3ddmg_u8)b])
        {
            Wire3DDMG_DrawLine2D(
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)a],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)a],
                w3ddmg_screen_x[(__safe_index w3ddmg_u8)b],
                w3ddmg_screen_y[(__safe_index w3ddmg_u8)b]);
        }

        i = (w3ddmg_u8)(i + 1);
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Store the EndFrame auxiliary-transfer gate; any nonzero value enables it.
// No data is copied or cleared here. Auxiliary source bytes alias the main stage.
void Wire3DDMG_SetAuxTransfer(w3ddmg_u8 flag)
{
    w3ddmg_aux_transfer_enabled = flag;
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Upload the union of current and previous dirty tiles, consuming their
// staged bytes. Including previous tiles erases pixels absent from the new frame.
// Move each current flag into previous and clear current after its upload.
static void w3ddmg_transfer_dirty_stage()
{
    w3ddmg_u8 tx;
    w3ddmg_u8 ty;
    w3ddmg_u8 idx;
    w3ddmg_u16 dst;

    idx = 0;
    tx = 0;
    while (tx < WIRE3D_DMG_TILE_W)
    {
        ty = 0;
        while (ty < WIRE3D_DMG_TILE_H)
        {
            w3ddmg_u8 cur;
            w3ddmg_u8 prev;

            cur = w3ddmg_dirty_tiles[(__safe_index w3ddmg_u8)idx];
            prev = w3ddmg_prev_dirty_tiles[(__safe_index w3ddmg_u8)idx];
            // Upload old-only tiles too: their cleared stage bytes erase the previous image.
            if ((cur != 0) || (prev != 0))
            {
                w3ddmg_dirty_src_hi = (w3ddmg_u8)(0xD0 + tx);
                w3ddmg_dirty_src_lo = (w3ddmg_u8)(ty << 3);
                dst = (w3ddmg_u16)(0x8901 + ((w3ddmg_u16)idx << 4));
                w3ddmg_dirty_dst_hi = (w3ddmg_u8)(dst >> 8);
                w3ddmg_dirty_dst_lo = (w3ddmg_u8)(dst & 0xFF);
                w3ddmg_transfer_dirty_tile_asm();
                w3ddmg_prev_dirty_tiles[(__safe_index w3ddmg_u8)idx] = cur;
                w3ddmg_dirty_tiles[(__safe_index w3ddmg_u8)idx] = 0;
            }

            idx = (w3ddmg_u8)(idx + 1);
            ty = (w3ddmg_u8)(ty + 1);
        }
        tx = (w3ddmg_u8)(tx + 1);
    }
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Store the dirty-transfer gate and clear both dirty histories and the main
// stage, even if the value is unchanged. VRAM is retained; enabling this over an
// old image can leave unmarked old tiles, so arrange a clean displayed baseline.
void Wire3DDMG_SetDirtyTransfer(w3ddmg_u8 flag)
{
    w3ddmg_dirty_transfer_enabled = flag;
    w3ddmg_clear_dirty_tables();
    w3ddmg_clear_stage_asm();
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Upload and consume the full main stage immediately, polling STAT but not
// waiting for the next VBlank. Ignore transfer gates and leave dirty histories
// and BG tile queue unchanged; the caller coordinates the frame lifecycle.
void Wire3DDMG_TransferMainNow()
{
    w3ddmg_transfer_stage_asm();
}
#endif

#if WIRE3D_DMG_HEIGHT == 120
#pragma bank 1
#pragma fixed_bank -1
#pragma fixed_order -1
// Upload and consume auxiliary source strips immediately, independent of the
// auxiliary gate. No VBlank-start wait or dirty-history/BG-queue update occurs.
// The source overlaps the main stage and must be coordinated with its upload.
void Wire3DDMG_TransferAuxNow()
{
    w3ddmg_transfer_stage_aux_asm();
}
#endif

#pragma fixed_order -1
#pragma fixed_bank -1
