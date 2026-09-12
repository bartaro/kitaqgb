#include "wire3d.h"

#pragma bank 1

#define W3D_TILE_W ((w3d_u8)16)
#define W3D_TILE_H ((w3d_u8)12)
#define W3D_ROW_BYTES ((w3d_u8)0x60)
#define W3D_NEAR_Z ((w3d_i16)8)
#define W3D_FAR_Z ((w3d_i16)255)
#define W3D_CENTER_X ((w3d_i16)64)
#define W3D_CENTER_Y ((w3d_i16)48)
#define W3D_TRANSFORM_LIMIT ((w3d_i16)220)
#define W3D_PROJECT_LIMIT ((w3d_i16)120)
#define W3D_HUD_TILE_BASE ((w3d_u8)0x80)
#define W3D_HUD_TILE_BLANK ((w3d_u8)0x8A)
#define W3D_HUD_TILE_COUNT ((w3d_u8)14)
#define W3D_BG_QUEUE_LIMIT ((w3d_u8)48)

__location(0xFF40) w3d_u8 w3d_reg_lcdc;
__location(0xFF42) w3d_u8 w3d_reg_scy;
__location(0xFF43) w3d_u8 w3d_reg_scx;
__location(0xFF44) w3d_u8 w3d_reg_ly;
__location(0xFF47) w3d_u8 w3d_reg_bgp;

__location(0x8000) w3d_u8 w3d_vram_tiles[6144];
__location(0x9800) w3d_u8 w3d_bg_map_9800[1024];
__location(0xD000) w3d_u8 w3d_stage[4096];

__prg_rom w3d_u8 w3d_bit_mask[8] = {
    0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01
};

__prg_rom w3d_u8 w3d_hud_tiles[224] = {
    0x3C, 0x3C, 0x66, 0x66, 0x6E, 0x6E, 0x76, 0x76, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x00, 0x00,
    0x18, 0x18, 0x38, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x3C, 0x00, 0x00,
    0x3C, 0x3C, 0x66, 0x66, 0x06, 0x06, 0x1C, 0x1C, 0x30, 0x30, 0x60, 0x60, 0x7E, 0x7E, 0x00, 0x00,
    0x7C, 0x7C, 0x06, 0x06, 0x06, 0x06, 0x3C, 0x3C, 0x06, 0x06, 0x06, 0x06, 0x7C, 0x7C, 0x00, 0x00,
    0x0C, 0x0C, 0x1C, 0x1C, 0x3C, 0x3C, 0x6C, 0x6C, 0x7E, 0x7E, 0x0C, 0x0C, 0x0C, 0x0C, 0x00, 0x00,
    0x7E, 0x7E, 0x60, 0x60, 0x7C, 0x7C, 0x06, 0x06, 0x06, 0x06, 0x66, 0x66, 0x3C, 0x3C, 0x00, 0x00,
    0x1C, 0x1C, 0x30, 0x30, 0x60, 0x60, 0x7C, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x00, 0x00,
    0x7E, 0x7E, 0x06, 0x06, 0x0C, 0x0C, 0x18, 0x18, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00,
    0x3C, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x00, 0x00,
    0x3C, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x3E, 0x06, 0x06, 0x0C, 0x0C, 0x38, 0x38, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x3E, 0x3E, 0x60, 0x60, 0x60, 0x60, 0x3C, 0x3C, 0x06, 0x06, 0x06, 0x06, 0x7C, 0x7C, 0x00, 0x00,
    0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x7E, 0x00, 0x00,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x00
};

__prg_rom w3d_i8 w3d_sin_q6[16] = {
     0,  24,  45,  59,  64,  59,  45,  24,
     0, -24, -45, -59, -64, -59, -45, -24
};

__prg_rom w3d_i8 w3d_cos_q6[16] = {
     64,  59,  45,  24,   0, -24, -45, -59,
    -64, -59, -45, -24,   0,  24,  45,  59
};

__prg_rom w3d_u8 w3d_inv_depth[256] = {
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

w3d_i16 w3d_cam_x;
w3d_i16 w3d_cam_y;
w3d_i16 w3d_cam_z;
w3d_i8 w3d_cam_pitch;
w3d_i8 w3d_cam_yaw;
w3d_i8 w3d_cam_roll;

w3d_u8 w3d_plot_x;
w3d_u8 w3d_plot_y;
w3d_u8 w3d_plot_tx;
w3d_u8 w3d_line_x0;
w3d_u8 w3d_line_y0;
w3d_u8 w3d_line_x1;
w3d_u8 w3d_line_y1;
w3d_u8 w3d_line_x;
w3d_u8 w3d_line_y;
w3d_u8 w3d_line_dx;
w3d_u8 w3d_line_dy;
w3d_u8 w3d_line_sx;
w3d_u8 w3d_line_sy;
w3d_u8 w3d_line_err;

w3d_u8 w3d_screen_x[WIRE3D_MODEL_VERTEX_LIMIT];
w3d_u8 w3d_screen_y[WIRE3D_MODEL_VERTEX_LIMIT];
w3d_u8 w3d_screen_visible[WIRE3D_MODEL_VERTEX_LIMIT];
w3d_u8 w3d_face_visible[WIRE3D_MODEL_FACE_LIMIT];
w3d_u8 w3d_edge_flags[WIRE3D_MODEL_EDGE_LIMIT];
w3d_u8 w3d_occlusion_mask[1536];
w3d_u8 w3d_occlusion_active;
w3d_u8 w3d_scene_order[WIRE3D_SCENE_OBJECT_LIMIT];
w3d_i16 w3d_scene_depth[WIRE3D_SCENE_OBJECT_LIMIT];
w3d_u8 w3d_bgq_x[48];
w3d_u8 w3d_bgq_y[48];
w3d_u8 w3d_bgq_tile[48];
w3d_u8 w3d_bgq_count;
w3d_u8 w3d_bgq_addr_hi;
w3d_u8 w3d_bgq_addr_lo;
w3d_u8 w3d_bgq_value;

void w3d_clear_vram_asm();
void w3d_fill_bg_map_asm();
void w3d_clear_occlusion_mask_asm();
void w3d_put_bg_tile_safe_asm();

static w3d_i16 w3d_clamp_i16(w3d_i16 v, w3d_i16 lo, w3d_i16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

static w3d_u8 w3d_clamp_screen(w3d_i16 v, w3d_u8 max)
{
    if (v < 0) return 0;
    if (v > (w3d_i16)max) return max;
    return (w3d_u8)v;
}

static w3d_i8 w3d_neg_angle(w3d_i8 v)
{
    return (w3d_i8)(0 - v);
}

static void w3d_rotate_y(w3d_i16* px, w3d_i16* pz, w3d_i8 angle)
{
    w3d_u8 ai;
    w3d_i16 s;
    w3d_i16 c;
    w3d_i16 in_x;
    w3d_i16 in_z;
    w3d_i16 out_x;
    w3d_i16 out_z;

    ai = (w3d_u8)angle;
    ai = (w3d_u8)(ai & 15);
    s = w3d_sin_q6[(__safe_index w3d_u8)ai];
    c = w3d_cos_q6[(__safe_index w3d_u8)ai];
    in_x = w3d_clamp_i16(*px, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    in_z = w3d_clamp_i16(*pz, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    out_x = (w3d_i16)(((in_x * c) + (in_z * s)) >> 6);
    out_z = (w3d_i16)(((in_z * c) - (in_x * s)) >> 6);
    *px = out_x;
    *pz = out_z;
}

static void w3d_rotate_x(w3d_i16* py, w3d_i16* pz, w3d_i8 angle)
{
    w3d_u8 ai;
    w3d_i16 s;
    w3d_i16 c;
    w3d_i16 in_y;
    w3d_i16 in_z;
    w3d_i16 out_y;
    w3d_i16 out_z;

    ai = (w3d_u8)angle;
    ai = (w3d_u8)(ai & 15);
    s = w3d_sin_q6[(__safe_index w3d_u8)ai];
    c = w3d_cos_q6[(__safe_index w3d_u8)ai];
    in_y = w3d_clamp_i16(*py, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    in_z = w3d_clamp_i16(*pz, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    out_y = (w3d_i16)(((in_y * c) - (in_z * s)) >> 6);
    out_z = (w3d_i16)(((in_y * s) + (in_z * c)) >> 6);
    *py = out_y;
    *pz = out_z;
}

static void w3d_rotate_z(w3d_i16* px, w3d_i16* py, w3d_i8 angle)
{
    w3d_u8 ai;
    w3d_i16 s;
    w3d_i16 c;
    w3d_i16 in_x;
    w3d_i16 in_y;
    w3d_i16 out_x;
    w3d_i16 out_y;

    ai = (w3d_u8)angle;
    ai = (w3d_u8)(ai & 15);
    s = w3d_sin_q6[(__safe_index w3d_u8)ai];
    c = w3d_cos_q6[(__safe_index w3d_u8)ai];
    in_x = w3d_clamp_i16(*px, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    in_y = w3d_clamp_i16(*py, (w3d_i16)(0 - W3D_TRANSFORM_LIMIT), W3D_TRANSFORM_LIMIT);
    out_x = (w3d_i16)(((in_x * c) - (in_y * s)) >> 6);
    out_y = (w3d_i16)(((in_x * s) + (in_y * c)) >> 6);
    *px = out_x;
    *py = out_y;
}

static w3d_u8 w3d_project_camera_space(w3d_i16 vx, w3d_i16 vy, w3d_i16 vz, w3d_u8* sx, w3d_u8* sy)
{
    w3d_u8 iz;
    w3d_i16 px;
    w3d_i16 py;
    w3d_i16 ox;
    w3d_i16 oy;

    if (vz < W3D_NEAR_Z) return 0;
    if (vz > W3D_FAR_Z) return 0;

    px = w3d_clamp_i16(vx, (w3d_i16)(0 - W3D_PROJECT_LIMIT), W3D_PROJECT_LIMIT);
    py = w3d_clamp_i16(vy, (w3d_i16)(0 - W3D_PROJECT_LIMIT), W3D_PROJECT_LIMIT);
    iz = w3d_inv_depth[(__safe_index w3d_u8)((w3d_u8)vz)];

    ox = (w3d_i16)((px * (w3d_i16)iz) >> 5);
    oy = (w3d_i16)((py * (w3d_i16)iz) >> 5);

    *sx = w3d_clamp_screen((w3d_i16)(W3D_CENTER_X + ox), (w3d_u8)(WIRE3D_SCREEN_W - 1));
    *sy = w3d_clamp_screen((w3d_i16)(W3D_CENTER_Y - oy), (w3d_u8)(WIRE3D_SCREEN_H - 1));
    return 1;
}

static w3d_u8 w3d_project_world(w3d_i16 wx, w3d_i16 wy, w3d_i16 wz, w3d_u8* sx, w3d_u8* sy)
{
    w3d_i16 vx;
    w3d_i16 vy;
    w3d_i16 vz;

    vx = (w3d_i16)(wx - w3d_cam_x);
    vy = (w3d_i16)(wy - w3d_cam_y);
    vz = (w3d_i16)(wz - w3d_cam_z);

    w3d_rotate_y(&vx, &vz, (w3d_i8)w3d_neg_angle(w3d_cam_yaw));
    w3d_rotate_x(&vy, &vz, (w3d_i8)w3d_neg_angle(w3d_cam_pitch));
    w3d_rotate_z(&vx, &vy, (w3d_i8)w3d_neg_angle(w3d_cam_roll));

    return w3d_project_camera_space(vx, vy, vz, sx, sy);
}

w3d_u8 Wire3D_ProjectPoint(w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_u8* sx, w3d_u8* sy)
{
    return w3d_project_world(x, y, z, sx, sy);
}

static void w3d_load_hud_tiles()
{
    w3d_u16 src;
    w3d_u16 dst;

    src = 0;
    dst = 0x0800;
    while (src < (w3d_u16)(W3D_HUD_TILE_COUNT * 16))
    {
        w3d_vram_tiles[(__safe_index w3d_u16)dst] = w3d_hud_tiles[(__safe_index w3d_u16)src];
        src = (w3d_u16)(src + 1);
        dst = (w3d_u16)(dst + 1);
    }
}

void Wire3D_PutBgTile(w3d_u8 x, w3d_u8 y, w3d_u8 tile)
{
    if (x >= 32) return;
    if (y >= 32) return;
    if (w3d_bgq_count >= W3D_BG_QUEUE_LIMIT) return;
    w3d_bgq_x[(__safe_index w3d_u8)w3d_bgq_count] = x;
    w3d_bgq_y[(__safe_index w3d_u8)w3d_bgq_count] = y;
    w3d_bgq_tile[(__safe_index w3d_u8)w3d_bgq_count] = tile;
    w3d_bgq_count = (w3d_u8)(w3d_bgq_count + 1);
}

void Wire3D_SetPalette(w3d_u8 bgp)
{
    w3d_reg_bgp = bgp;
}

static void w3d_flush_bg_queue()
{
    w3d_u8 i;

    i = 0;
    while (i < w3d_bgq_count)
    {
        w3d_u16 off;

        off = (w3d_u16)(((w3d_u16)w3d_bgq_y[(__safe_index w3d_u8)i] << 5) + (w3d_u16)w3d_bgq_x[(__safe_index w3d_u8)i]);
        off = (w3d_u16)(off + 0x9800);
        w3d_bgq_addr_hi = (w3d_u8)(off >> 8);
        w3d_bgq_addr_lo = (w3d_u8)off;
        w3d_bgq_value = w3d_bgq_tile[(__safe_index w3d_u8)i];
        w3d_put_bg_tile_safe_asm();
        i = (w3d_u8)(i + 1);
    }
    w3d_bgq_count = 0;
}

static w3d_i16 w3d_scene_object_depth(const Wire3D_Object* obj)
{
    w3d_i16 vx;
    w3d_i16 vy;
    w3d_i16 vz;

    vx = (w3d_i16)(obj->x - w3d_cam_x);
    vy = (w3d_i16)(obj->y - w3d_cam_y);
    vz = (w3d_i16)(obj->z - w3d_cam_z);

    w3d_rotate_y(&vx, &vz, (w3d_i8)w3d_neg_angle(w3d_cam_yaw));
    w3d_rotate_x(&vy, &vz, (w3d_i8)w3d_neg_angle(w3d_cam_pitch));
    w3d_rotate_z(&vx, &vy, (w3d_i8)w3d_neg_angle(w3d_cam_roll));

    return vz;
}

static w3d_i16 w3d_abs_i16(w3d_i16 v)
{
    if (v < 0) return (w3d_i16)(0 - v);
    return v;
}

static void w3d_clear_occlusion_mask()
{
    w3d_clear_occlusion_mask_asm();
}

static w3d_u16 w3d_mask_offset(w3d_u8 x, w3d_u8 y)
{
    return (w3d_u16)(((w3d_u16)y << 4) + (w3d_u16)(x >> 3));
}

static w3d_u8 w3d_mask_get(w3d_u8 x, w3d_u8 y)
{
    w3d_u16 ofs;
    w3d_u8 bit;

    if (x >= WIRE3D_SCREEN_W) return 1;
    if (y >= WIRE3D_SCREEN_H) return 1;

    ofs = w3d_mask_offset(x, y);
    bit = w3d_bit_mask[(__safe_index w3d_u8)(x & 7)];
    if ((w3d_occlusion_mask[(__safe_index w3d_u16)ofs] & bit) != 0) return 1;
    return 0;
}

static void w3d_mask_set(w3d_u8 x, w3d_u8 y)
{
    w3d_u16 ofs;
    w3d_u8 bit;

    if (x >= WIRE3D_SCREEN_W) return;
    if (y >= WIRE3D_SCREEN_H) return;

    ofs = w3d_mask_offset(x, y);
    bit = w3d_bit_mask[(__safe_index w3d_u8)(x & 7)];
    w3d_occlusion_mask[(__safe_index w3d_u16)ofs] = (w3d_u8)(w3d_occlusion_mask[(__safe_index w3d_u16)ofs] | bit);
}

static void w3d_stage_clear_pixel(w3d_u8 x, w3d_u8 y)
{
    w3d_u16 ofs;
    w3d_u8 bit;

    if (x >= WIRE3D_SCREEN_W) return;
    if (y >= WIRE3D_SCREEN_H) return;

    ofs = (w3d_u16)((((w3d_u16)(x >> 3)) << 8) + (((w3d_u16)(y >> 3)) << 3) + (w3d_u16)(y & 7));
    bit = w3d_bit_mask[(__safe_index w3d_u8)(x & 7)];
    w3d_stage[(__safe_index w3d_u16)ofs] = (w3d_u8)(w3d_stage[(__safe_index w3d_u16)ofs] & (w3d_u8)(0xFF ^ bit));
}

static void w3d_stage_clear_span(w3d_u8 y, w3d_u8 min_x, w3d_u8 max_x)
{
    w3d_u8 x;
    w3d_u16 ofs;

    if (y >= WIRE3D_SCREEN_H) return;

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        w3d_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3d_u8)(x + 1);
    }

    while ((w3d_u8)(x + 7) <= max_x)
    {
        ofs = (w3d_u16)((((w3d_u16)(x >> 3)) << 8) + (((w3d_u16)(y >> 3)) << 3) + (w3d_u16)(y & 7));
        w3d_stage[(__safe_index w3d_u16)ofs] = 0;
        x = (w3d_u8)(x + 8);
    }

    while (x <= max_x)
    {
        w3d_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (w3d_u8)(x + 1);
    }
}

static void w3d_mask_set_span(w3d_u8 y, w3d_u8 min_x, w3d_u8 max_x)
{
    w3d_u8 x;
    w3d_u16 ofs;

    if (y >= WIRE3D_SCREEN_H) return;

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        w3d_mask_set(x, y);
        if (x == max_x) return;
        x = (w3d_u8)(x + 1);
    }

    while ((w3d_u8)(x + 7) <= max_x)
    {
        ofs = w3d_mask_offset(x, y);
        w3d_occlusion_mask[(__safe_index w3d_u16)ofs] = 0xFF;
        x = (w3d_u8)(x + 8);
    }

    while (x <= max_x)
    {
        w3d_mask_set(x, y);
        if (x == max_x) return;
        x = (w3d_u8)(x + 1);
    }
}

static void w3d_line_stage_masked_c(w3d_u8 x0, w3d_u8 y0, w3d_u8 x1, w3d_u8 y1)
{
    w3d_u8 mx;
    w3d_u8 my;
    w3d_u8 q0x;
    w3d_u8 q0y;
    w3d_u8 q1x;
    w3d_u8 q1y;

    mx = (w3d_u8)(((w3d_u16)x0 + (w3d_u16)x1) >> 1);
    my = (w3d_u8)(((w3d_u16)y0 + (w3d_u16)y1) >> 1);
    q0x = (w3d_u8)(((w3d_u16)x0 + (w3d_u16)mx) >> 1);
    q0y = (w3d_u8)(((w3d_u16)y0 + (w3d_u16)my) >> 1);
    q1x = (w3d_u8)(((w3d_u16)x1 + (w3d_u16)mx) >> 1);
    q1y = (w3d_u8)(((w3d_u16)y1 + (w3d_u16)my) >> 1);

    if (w3d_mask_get(x0, y0) &&
        w3d_mask_get(q0x, q0y) &&
        w3d_mask_get(mx, my) &&
        w3d_mask_get(q1x, q1y) &&
        w3d_mask_get(x1, y1))
    {
        return;
    }

    w3d_line_x0 = x0;
    w3d_line_y0 = y0;
    w3d_line_x1 = x1;
    w3d_line_y1 = y1;
    w3d_line_stage_asm();
}

static w3d_i16 w3d_screen_delta(w3d_u8 a, w3d_u8 b)
{
    return (w3d_i16)((w3d_i16)a - (w3d_i16)b);
}

static w3d_i16 w3d_edge_area(w3d_i16 ax, w3d_i16 ay, w3d_i16 bx, w3d_i16 by, w3d_i16 px, w3d_i16 py)
{
    return (w3d_i16)(((bx - ax) * (py - ay)) - ((by - ay) * (px - ax)));
}

static void w3d_mark_triangle(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by, w3d_u8 cx, w3d_u8 cy)
{
    w3d_u8 min_x;
    w3d_u8 max_x;
    w3d_u8 min_y;
    w3d_u8 max_y;
    w3d_i16 area;
    w3d_u8 y;

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

    area = w3d_edge_area((w3d_i16)ax, (w3d_i16)ay, (w3d_i16)bx, (w3d_i16)by, (w3d_i16)cx, (w3d_i16)cy);
    if (area == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        w3d_mask_set_span(y, min_x, max_x);
        if (y == max_y) break;
        y = (w3d_u8)(y + 1);
    }
}

static void w3d_clear_triangle(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by, w3d_u8 cx, w3d_u8 cy)
{
    w3d_u8 min_x;
    w3d_u8 max_x;
    w3d_u8 min_y;
    w3d_u8 max_y;
    w3d_u8 y;

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

    if (w3d_edge_area((w3d_i16)ax, (w3d_i16)ay, (w3d_i16)bx, (w3d_i16)by, (w3d_i16)cx, (w3d_i16)cy) == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        w3d_u8 hits;
        w3d_i16 span_min;
        w3d_i16 span_max;
        w3d_i16 dx;
        w3d_i16 dy;
        w3d_i16 ix;

        hits = 0;
        span_min = 127;
        span_max = 0;

        if (ay != by)
        {
            if (((y >= ay) && (y <= by)) || ((y >= by) && (y <= ay)))
            {
                dx = (w3d_i16)((w3d_i16)bx - (w3d_i16)ax);
                dy = (w3d_i16)((w3d_i16)by - (w3d_i16)ay);
                ix = (w3d_i16)((w3d_i16)ax + ((((w3d_i16)y - (w3d_i16)ay) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3d_u8)(hits + 1);
            }
        }

        if (by != cy)
        {
            if (((y >= by) && (y <= cy)) || ((y >= cy) && (y <= by)))
            {
                dx = (w3d_i16)((w3d_i16)cx - (w3d_i16)bx);
                dy = (w3d_i16)((w3d_i16)cy - (w3d_i16)by);
                ix = (w3d_i16)((w3d_i16)bx + ((((w3d_i16)y - (w3d_i16)by) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3d_u8)(hits + 1);
            }
        }

        if (cy != ay)
        {
            if (((y >= cy) && (y <= ay)) || ((y >= ay) && (y <= cy)))
            {
                dx = (w3d_i16)((w3d_i16)ax - (w3d_i16)cx);
                dy = (w3d_i16)((w3d_i16)ay - (w3d_i16)cy);
                ix = (w3d_i16)((w3d_i16)cx + ((((w3d_i16)y - (w3d_i16)cy) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (w3d_u8)(hits + 1);
            }
        }

        if (hits >= 2)
        {
            if (span_min < (w3d_i16)min_x) span_min = (w3d_i16)min_x;
            if (span_max > (w3d_i16)max_x) span_max = (w3d_i16)max_x;
            if (span_min < 0) span_min = 0;
            if (span_max > (w3d_i16)(WIRE3D_SCREEN_W - 1)) span_max = (w3d_i16)(WIRE3D_SCREEN_W - 1);
            if (span_min <= span_max)
            {
                w3d_stage_clear_span(y, (w3d_u8)span_min, (w3d_u8)span_max);
            }
        }

        if (y == max_y) break;
        y = (w3d_u8)(y + 1);
    }
}

static void w3d_build_face_visibility(const Wire3D_Model* model, w3d_u8 count, w3d_u8 face_count)
{
    w3d_u8 i;

    i = 0;
    while (i < face_count)
    {
        const Wire3D_Face* f;
        w3d_i16 abx;
        w3d_i16 aby;
        w3d_i16 acx;
        w3d_i16 acy;
        w3d_i16 area;

        w3d_face_visible[(__safe_index w3d_u8)i] = 0;
        f = &model->faces[(__safe_index w3d_u8)i];

        if ((f->a < count) && (f->b < count) && (f->c < count))
        {
            if (w3d_screen_visible[(__safe_index w3d_u8)f->a] &&
                w3d_screen_visible[(__safe_index w3d_u8)f->b] &&
                w3d_screen_visible[(__safe_index w3d_u8)f->c])
            {
                abx = w3d_screen_delta(w3d_screen_x[(__safe_index w3d_u8)f->b], w3d_screen_x[(__safe_index w3d_u8)f->a]);
                aby = w3d_screen_delta(w3d_screen_y[(__safe_index w3d_u8)f->b], w3d_screen_y[(__safe_index w3d_u8)f->a]);
                acx = w3d_screen_delta(w3d_screen_x[(__safe_index w3d_u8)f->c], w3d_screen_x[(__safe_index w3d_u8)f->a]);
                acy = w3d_screen_delta(w3d_screen_y[(__safe_index w3d_u8)f->c], w3d_screen_y[(__safe_index w3d_u8)f->a]);
                area = (w3d_i16)((abx * acy) - (aby * acx));
                if (area > 0)
                {
                    w3d_face_visible[(__safe_index w3d_u8)i] = 1;
                }
            }
        }
        i = (w3d_u8)(i + 1);
    }
}

static w3d_u8 w3d_is_edge_visible(const Wire3D_Model* model, w3d_u8 edge_index, w3d_u8 face_count)
{
    const Wire3D_EdgeFaces* ef;
    w3d_u8 any_face;

    if ((model->flags & WIRE3D_MODEL_HIDDEN_LINES) == 0) return 1;
    if (model->faces == 0) return 1;
    if (model->edge_faces == 0) return 1;

    ef = &model->edge_faces[(__safe_index w3d_u8)edge_index];
    any_face = 0;

    if (ef->f0 != WIRE3D_FACE_NONE)
    {
        any_face = 1;
        if (ef->f0 >= face_count) return 1;
        if (w3d_face_visible[(__safe_index w3d_u8)ef->f0]) return 1;
    }

    if (ef->f1 != WIRE3D_FACE_NONE)
    {
        any_face = 1;
        if (ef->f1 >= face_count) return 1;
        if (w3d_face_visible[(__safe_index w3d_u8)ef->f1]) return 1;
    }

    if (any_face == 0) return 1;
    return 0;
}

w3d_u16 Wire3D_SelectEdgeMask(const Wire3D_Model* model, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz)
{
    w3d_u8 count;
    w3d_u8 bin;

    if (model == 0) return 0xFFFF;
    if (model->edge_masks == 0) return 0xFFFF;
    count = model->edge_mask_count;
    if (count == 0) return 0xFFFF;

    (void)rx;
    (void)rz;

    bin = (w3d_u8)(((w3d_u8)ry) & 15);
    if (count >= 16) return model->edge_masks[(__safe_index w3d_u8)bin];
    if (count == 8) bin = (w3d_u8)(bin >> 1);
    else if (count == 4) bin = (w3d_u8)(bin >> 2);
    else if (count == 2) bin = (w3d_u8)(bin >> 3);
    else bin = 0;

    if (bin >= count) bin = (w3d_u8)(count - 1);
    return model->edge_masks[(__safe_index w3d_u8)bin];
}

static void w3d_build_edge_flags(const Wire3D_Model* model, w3d_u8 edge_count, w3d_u8 face_count, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz)
{
    w3d_u8 i;
    w3d_u16 edge_mask;
    w3d_u16 bit;

    edge_mask = Wire3D_SelectEdgeMask(model, rx, ry, rz);
    bit = 1;
    i = 0;
    while (i < edge_count)
    {
        w3d_u8 visible;

        visible = 0;
        if ((edge_mask & bit) != 0)
        {
            visible = 1;
            if ((model->flags & WIRE3D_MODEL_HIDDEN_LINES) &&
                (model->faces != 0) &&
                (model->edge_faces != 0))
            {
                const Wire3D_EdgeFaces* ef;
                w3d_u8 any_face;

                ef = &model->edge_faces[(__safe_index w3d_u8)i];
                any_face = 0;
                visible = 0;

                if (ef->f0 != WIRE3D_FACE_NONE)
                {
                    any_face = 1;
                    if ((ef->f0 >= face_count) ||
                        (w3d_face_visible[(__safe_index w3d_u8)ef->f0] != 0))
                    {
                        visible = 1;
                    }
                }

                if ((visible == 0) && (ef->f1 != WIRE3D_FACE_NONE))
                {
                    any_face = 1;
                    if ((ef->f1 >= face_count) ||
                        (w3d_face_visible[(__safe_index w3d_u8)ef->f1] != 0))
                    {
                        visible = 1;
                    }
                }

                if (any_face == 0) visible = 1;
            }
        }

        w3d_edge_flags[(__safe_index w3d_u8)i] = visible;
        bit = (w3d_u16)(bit << 1);
        i = (w3d_u8)(i + 1);
    }
}

static void w3d_mark_model_occluder(const Wire3D_Model* model)
{
    w3d_u8 count;
    w3d_u8 face_count;
    w3d_u8 i;

    if (model == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3D_MODEL_HIDDEN_LINES) == 0) return;

    count = model->vertex_count;
    if (count > WIRE3D_MODEL_VERTEX_LIMIT) count = WIRE3D_MODEL_VERTEX_LIMIT;
    face_count = model->face_count;
    if (face_count > WIRE3D_MODEL_FACE_LIMIT) face_count = WIRE3D_MODEL_FACE_LIMIT;

    w3d_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3D_Face* f;
        f = &model->faces[(__safe_index w3d_u8)i];
        if (w3d_face_visible[(__safe_index w3d_u8)i])
        {
            w3d_mark_triangle(
                w3d_screen_x[(__safe_index w3d_u8)f->a],
                w3d_screen_y[(__safe_index w3d_u8)f->a],
                w3d_screen_x[(__safe_index w3d_u8)f->b],
                w3d_screen_y[(__safe_index w3d_u8)f->b],
                w3d_screen_x[(__safe_index w3d_u8)f->c],
                w3d_screen_y[(__safe_index w3d_u8)f->c]);
        }
        i = (w3d_u8)(i + 1);
    }
}

void w3d_wait_vblank_start()
{
    if ((w3d_reg_lcdc & 0x80) == 0) return;
    while (w3d_reg_ly >= 144) { }
    while (w3d_reg_ly < 144) { }
}

#pragma fixed_bank 1
#pragma fixed_order 92
void w3d_clear_vram_asm()
{
    __asm {
w3dcv_enter:
        LD_HL_IMM w3d_vram_tiles
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
        RET
    }
}

#pragma fixed_order 94
void w3d_fill_bg_map_asm()
{
    __asm {
w3dfb_enter:
        LD_HL_IMM w3d_bg_map_9800
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

#pragma fixed_order 96
void w3d_clear_occlusion_mask_asm()
{
    __asm {
w3dco_enter:
        LD_HL_IMM w3d_occlusion_mask
        XOR_A
        LD_D_IMM 6

w3dco_page:
        LD_C_IMM 0

w3dco_loop:
        LDI_HL_A
        DEC_C
        JR_NZ w3dco_loop
        DEC_D
        JR_NZ w3dco_page
        RET
    }
}

#pragma fixed_order 98
void w3d_put_bg_tile_safe_asm()
{
    __asm {
w3dbg_enter:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ w3dbg_enter
        LD_A_MEM w3d_bgq_addr_hi
        LD_H_A
        LD_A_MEM w3d_bgq_addr_lo
        LD_L_A
        LD_A_MEM w3d_bgq_value
        LD_HL_A
        RET
    }
}

#pragma fixed_order 100
void w3d_clear_stage_asm()
{
    __asm {
w3dcs_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD0
        LD_C_IMM 16

w3dcs_outer:
        LD_L_IMM 0
        LD_B_IMM 0x60

w3dcs_inner:
        LDI_HL_A
        DEC_B
        JR_NZ w3dcs_inner
        INC_H
        DEC_C
        JR_NZ w3dcs_outer
        RET
    }
}

#pragma fixed_order 110
void w3d_plot_stage_asm()
{
    __asm {
w3dps_enter:
        LD_A_MEM w3d_plot_x
        CP_IMM 0x80
        JP_NC w3dps_ret

        LD_A_MEM w3d_plot_y
        CP_IMM 0x60
        JP_NC w3dps_ret

        LD_A_MEM w3d_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A w3d_plot_tx

        LD_A_MEM w3d_plot_tx
        LD_B_A
        LD_A_IMM 0xD0
        ADD_B
        LD_H_A

        LD_A_MEM w3d_plot_y
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

        LD_A_MEM w3d_plot_y
        AND_IMM 7
        ADD_B
        LD_L_A

        LD_A_MEM w3d_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM w3d_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

        LD_A_HL
        OR_B
        LD_HL_A

w3dps_ret:
        RET
    }
}

#pragma fixed_order 120
void w3d_line_stage_asm()
{
    __asm {
w3dls_enter:
        LD_A_MEM w3d_line_x0
        LD_B_A
        LD_A_MEM w3d_line_x1
        CP_B
        JP_C w3dls_x_reverse
        SUB_B
        LD_MEM_A w3d_line_dx
        LD_A_IMM 1
        LD_MEM_A w3d_line_sx
        JP w3dls_y_start

w3dls_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3d_line_dx
        LD_A_IMM 255
        LD_MEM_A w3d_line_sx

w3dls_y_start:
        LD_A_MEM w3d_line_y0
        LD_B_A
        LD_A_MEM w3d_line_y1
        CP_B
        JP_C w3dls_y_reverse
        SUB_B
        LD_MEM_A w3d_line_dy
        LD_A_IMM 1
        LD_MEM_A w3d_line_sy
        JP w3dls_branch

w3dls_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3d_line_dy
        LD_A_IMM 255
        LD_MEM_A w3d_line_sy

w3dls_branch:
        LD_A_MEM w3d_line_dx
        LD_B_A
        LD_A_MEM w3d_line_dy
        CP_B
        JP_C w3dls_shallow
        JP_Z w3dls_shallow
        JP w3dls_steep

w3dls_shallow:
        LD_A_MEM w3d_line_x0
        LD_MEM_A w3d_line_x
        LD_A_MEM w3d_line_y0
        LD_MEM_A w3d_line_y
        LD_A_MEM w3d_line_dx
        OR_A
        RRA
        LD_MEM_A w3d_line_err

w3dls_shallow_loop:
        LD_A_MEM w3d_line_x
        LD_MEM_A w3d_plot_x
        LD_A_MEM w3d_line_y
        LD_MEM_A w3d_plot_y
        CALL w3d_plot_stage_asm

        LD_A_MEM w3d_line_x
        LD_B_A
        LD_A_MEM w3d_line_x1
        CP_B
        JP_Z w3dls_done

        LD_A_MEM w3d_line_dy
        LD_B_A
        LD_A_MEM w3d_line_err
        CP_B
        JP_NC w3dls_shallow_skip_bridge

        LD_A_MEM w3d_line_y
        LD_B_A
        LD_A_MEM w3d_line_sy
        ADD_B
        LD_MEM_A w3d_line_y

        LD_A_MEM w3d_line_x
        LD_MEM_A w3d_plot_x
        LD_A_MEM w3d_line_y
        LD_MEM_A w3d_plot_y
        CALL w3d_plot_stage_asm

        LD_A_MEM w3d_line_dx
        LD_B_A
        LD_A_MEM w3d_line_err
        ADD_B
        LD_MEM_A w3d_line_err

w3dls_shallow_skip_bridge:
        LD_A_MEM w3d_line_err
        LD_B_A
        LD_A_MEM w3d_line_dy
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3d_line_err

        LD_A_MEM w3d_line_x
        LD_B_A
        LD_A_MEM w3d_line_sx
        ADD_B
        LD_MEM_A w3d_line_x
        JP w3dls_shallow_loop

w3dls_steep:
        LD_A_MEM w3d_line_x0
        LD_MEM_A w3d_line_x
        LD_A_MEM w3d_line_y0
        LD_MEM_A w3d_line_y
        LD_A_MEM w3d_line_dy
        OR_A
        RRA
        LD_MEM_A w3d_line_err

w3dls_steep_loop:
        LD_A_MEM w3d_line_x
        LD_MEM_A w3d_plot_x
        LD_A_MEM w3d_line_y
        LD_MEM_A w3d_plot_y
        CALL w3d_plot_stage_asm

        LD_A_MEM w3d_line_y
        LD_B_A
        LD_A_MEM w3d_line_y1
        CP_B
        JP_Z w3dls_done

        LD_A_MEM w3d_line_dx
        LD_B_A
        LD_A_MEM w3d_line_err
        CP_B
        JP_NC w3dls_steep_skip_bridge

        LD_A_MEM w3d_line_x
        LD_B_A
        LD_A_MEM w3d_line_sx
        ADD_B
        LD_MEM_A w3d_line_x

        LD_A_MEM w3d_line_x
        LD_MEM_A w3d_plot_x
        LD_A_MEM w3d_line_y
        LD_MEM_A w3d_plot_y
        CALL w3d_plot_stage_asm

        LD_A_MEM w3d_line_dy
        LD_B_A
        LD_A_MEM w3d_line_err
        ADD_B
        LD_MEM_A w3d_line_err

w3dls_steep_skip_bridge:
        LD_A_MEM w3d_line_err
        LD_B_A
        LD_A_MEM w3d_line_dx
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A w3d_line_err

        LD_A_MEM w3d_line_y
        LD_B_A
        LD_A_MEM w3d_line_sy
        ADD_B
        LD_MEM_A w3d_line_y
        JP w3dls_steep_loop

w3dls_done:
        RET
    }
}

#pragma fixed_order 130
void w3d_transfer_stage_asm()
{
    __asm {
w3dtf_enter:
        LD_HL_IMM 0x8900
        LD_DE_IMM w3d_stage
        LD_B_IMM 16

        INC_L
w3dtf_row:
        PUSH_BC

w3dtf_col:
        LD_A_DE
        LD_C_A
        INC_E
        LD_A_DE
        LD_B_A
        INC_E
        LD_A_DE
        INC_E
        INC_E
        PUSH_DE
        PUSH_AF
        DEC_E
        LD_A_DE
        LD_D_A
        POP_AF
        LD_E_A

w3dtf_wait:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ w3dtf_wait

        LD_A_C
        LDI_HL_A
        INC_L
        LD_A_B
        LDI_HL_A
        INC_L
        LD_A_E
        LDI_HL_A
        INC_L
        LD_A_D
        LDI_HL_A
        INC_L

        POP_DE
        LD_A_E
        SUB_IMM 4
        LD_E_A
        XOR_A
        LD_DE_A
        INC_E
        LD_DE_A
        INC_E
        LD_DE_A
        INC_E
        LD_DE_A
        INC_E

        LD_A_E
        CP_IMM 0x60
        JR_C w3dtf_col

        LD_E_IMM 0
        INC_D
        POP_BC
        DEC_B
        JR_NZ w3dtf_row
        RET
    }
}
#pragma fixed_order -1
#pragma fixed_bank -1

void Wire3D_Init()
{
    w3d_u8 row;
    w3d_u8 col;
    w3d_u8 tid;
    w3d_u16 off;

    w3d_wait_vblank_start();
    w3d_reg_lcdc = 0;

    w3d_clear_vram_asm();
    w3d_load_hud_tiles();
    w3d_fill_bg_map_asm();

    row = 0;
    tid = 0x90;
    while (row < W3D_TILE_H)
    {
        off = (w3d_u16)((w3d_u16)row << 5);
        col = 0;
        while (col < W3D_TILE_W)
        {
            w3d_bg_map_9800[(w3d_u16)(off + (w3d_u16)col)] = tid;
            tid = (w3d_u8)(tid + W3D_TILE_H);
            col = (w3d_u8)(col + 1);
        }
        row = (w3d_u8)(row + 1);
        tid = (w3d_u8)(0x90 + row);
    }

    w3d_reg_scx = 0;
    w3d_reg_scy = 0;
    w3d_reg_bgp = 0xB4;
    w3d_reg_lcdc = 0x81;
    w3d_bgq_count = 0;

    Wire3D_SetCamera(0, 0, 0, 0, 0, 0);
    w3d_clear_stage_asm();
}

void Wire3D_BeginFrame()
{
    w3d_clear_stage_asm();
    w3d_clear_occlusion_mask();
    w3d_occlusion_active = 0;
}

void Wire3D_SetCamera(w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 pitch, w3d_i8 yaw, w3d_i8 roll)
{
    w3d_cam_x = x;
    w3d_cam_y = y;
    w3d_cam_z = z;
    w3d_cam_pitch = pitch;
    w3d_cam_yaw = yaw;
    w3d_cam_roll = roll;
}

void Wire3D_DrawLine2D(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by)
{
    if (w3d_occlusion_active)
    {
        w3d_line_stage_masked_c(ax, ay, bx, by);
        return;
    }

    w3d_line_x0 = ax;
    w3d_line_y0 = ay;
    w3d_line_x1 = bx;
    w3d_line_y1 = by;
    w3d_line_stage_asm();
}

void Wire3D_DrawScene(Wire3D_Object* objects, w3d_u8 count)
{
    w3d_u8 scene_count;
    w3d_u8 i;

    if (objects == 0) return;

    scene_count = count;
    if (scene_count > WIRE3D_SCENE_OBJECT_LIMIT) scene_count = WIRE3D_SCENE_OBJECT_LIMIT;

    i = 0;
    while (i < scene_count)
    {
        w3d_scene_order[(__safe_index w3d_u8)i] = i;
        w3d_scene_depth[(__safe_index w3d_u8)i] = w3d_scene_object_depth(&objects[(__safe_index w3d_u8)i]);
        i = (w3d_u8)(i + 1);
    }

    i = 0;
    while (i < scene_count)
    {
        w3d_u8 j;
        j = (w3d_u8)(i + 1);
        while (j < scene_count)
        {
            w3d_u8 oi;
            w3d_u8 oj;
            oi = w3d_scene_order[(__safe_index w3d_u8)i];
            oj = w3d_scene_order[(__safe_index w3d_u8)j];

            if (w3d_scene_depth[(__safe_index w3d_u8)oj] < w3d_scene_depth[(__safe_index w3d_u8)oi])
            {
                w3d_scene_order[(__safe_index w3d_u8)i] = oj;
                w3d_scene_order[(__safe_index w3d_u8)j] = oi;
            }
            j = (w3d_u8)(j + 1);
        }
        i = (w3d_u8)(i + 1);
    }

    w3d_clear_occlusion_mask();
    w3d_occlusion_active = 1;

    i = 0;
    while (i < scene_count)
    {
        Wire3D_Object* obj;
        obj = &objects[(__safe_index w3d_u8)w3d_scene_order[(__safe_index w3d_u8)i]];
        if ((obj->visible != 0) && (obj->model != 0))
        {
            Wire3D_DrawModelScaled(obj->model, obj->x, obj->y, obj->z, (w3d_i8)obj->rx, (w3d_i8)obj->ry, (w3d_i8)obj->rz, obj->scale_q8);
            w3d_mark_model_occluder(obj->model);
        }
        i = (w3d_u8)(i + 1);
    }

    w3d_occlusion_active = 0;
}

void Wire3D_DrawLine3D(w3d_i16 ax, w3d_i16 ay, w3d_i16 az, w3d_i16 bx, w3d_i16 by, w3d_i16 bz)
{
    w3d_u8 sx0;
    w3d_u8 sy0;
    w3d_u8 sx1;
    w3d_u8 sy1;

    if (w3d_project_world(ax, ay, az, &sx0, &sy0) == 0) return;
    if (w3d_project_world(bx, by, bz, &sx1, &sy1) == 0) return;
    Wire3D_DrawLine2D(sx0, sy0, sx1, sy1);
}

void Wire3D_DrawModel(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz)
{
    Wire3D_DrawModelScaled(model, x, y, z, rx, ry, rz, 256);
}

void Wire3D_EraseModelFaces(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz, w3d_i16 scale_q8)
{
    w3d_u8 i;
    w3d_u8 count;
    w3d_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & WIRE3D_MODEL_HIDDEN_LINES) == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3D_MODEL_VERTEX_LIMIT) count = WIRE3D_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const Wire3D_Vec3* v;
        w3d_i16 vx;
        w3d_i16 vy;
        w3d_i16 vz;
        w3d_u8 sx;
        w3d_u8 sy;

        v = &model->vertices[(__safe_index w3d_u8)i];
        vx = (w3d_i16)((v->x * scale_q8) >> 8);
        vy = (w3d_i16)((v->y * scale_q8) >> 8);
        vz = (w3d_i16)((v->z * scale_q8) >> 8);

        w3d_rotate_y(&vx, &vz, (w3d_i8)ry);
        w3d_rotate_x(&vy, &vz, (w3d_i8)rx);
        w3d_rotate_z(&vx, &vy, (w3d_i8)rz);

        vx = (w3d_i16)(vx + x);
        vy = (w3d_i16)(vy + y);
        vz = (w3d_i16)(vz + z);

        if (w3d_project_world(vx, vy, vz, &sx, &sy))
        {
            w3d_screen_x[(__safe_index w3d_u8)i] = sx;
            w3d_screen_y[(__safe_index w3d_u8)i] = sy;
            w3d_screen_visible[(__safe_index w3d_u8)i] = 1;
        }
        else
        {
            w3d_screen_visible[(__safe_index w3d_u8)i] = 0;
        }
        i = (w3d_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3D_MODEL_FACE_LIMIT) face_count = WIRE3D_MODEL_FACE_LIMIT;
    w3d_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const Wire3D_Face* f;
        f = &model->faces[(__safe_index w3d_u8)i];
        if (w3d_face_visible[(__safe_index w3d_u8)i] &&
            w3d_screen_visible[(__safe_index w3d_u8)f->a] &&
            w3d_screen_visible[(__safe_index w3d_u8)f->b] &&
            w3d_screen_visible[(__safe_index w3d_u8)f->c])
        {
            w3d_clear_triangle(
                w3d_screen_x[(__safe_index w3d_u8)f->a],
                w3d_screen_y[(__safe_index w3d_u8)f->a],
                w3d_screen_x[(__safe_index w3d_u8)f->b],
                w3d_screen_y[(__safe_index w3d_u8)f->b],
                w3d_screen_x[(__safe_index w3d_u8)f->c],
                w3d_screen_y[(__safe_index w3d_u8)f->c]);
        }
        i = (w3d_u8)(i + 1);
    }
}

void Wire3D_EraseTriangle2D(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by, w3d_u8 cx, w3d_u8 cy)
{
    w3d_clear_triangle(ax, ay, bx, by, cx, cy);
}

void Wire3D_EraseSpan2D(w3d_u8 y, w3d_u8 x0, w3d_u8 x1)
{
    w3d_u8 tx;

    if (x0 > x1)
    {
        tx = x0;
        x0 = x1;
        x1 = tx;
    }
    w3d_stage_clear_span(y, x0, x1);
}

void Wire3D_DrawModelScaled(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz, w3d_i16 scale_q8)
{
    w3d_u8 i;
    w3d_u8 count;
    w3d_u8 edge_count;
    w3d_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->edges == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > WIRE3D_MODEL_VERTEX_LIMIT) count = WIRE3D_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const Wire3D_Vec3* v;
        w3d_i16 vx;
        w3d_i16 vy;
        w3d_i16 vz;
        w3d_u8 sx;
        w3d_u8 sy;

        v = &model->vertices[(__safe_index w3d_u8)i];
        vx = (w3d_i16)((v->x * scale_q8) >> 8);
        vy = (w3d_i16)((v->y * scale_q8) >> 8);
        vz = (w3d_i16)((v->z * scale_q8) >> 8);

        w3d_rotate_y(&vx, &vz, (w3d_i8)ry);
        w3d_rotate_x(&vy, &vz, (w3d_i8)rx);
        w3d_rotate_z(&vx, &vy, (w3d_i8)rz);

        vx = (w3d_i16)(vx + x);
        vy = (w3d_i16)(vy + y);
        vz = (w3d_i16)(vz + z);

        if (w3d_project_world(vx, vy, vz, &sx, &sy))
        {
            w3d_screen_x[(__safe_index w3d_u8)i] = sx;
            w3d_screen_y[(__safe_index w3d_u8)i] = sy;
            w3d_screen_visible[(__safe_index w3d_u8)i] = 1;
        }
        else
        {
            w3d_screen_visible[(__safe_index w3d_u8)i] = 0;
        }
        i = (w3d_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > WIRE3D_MODEL_FACE_LIMIT) face_count = WIRE3D_MODEL_FACE_LIMIT;
    if ((model->flags & WIRE3D_MODEL_HIDDEN_LINES) && (model->faces != 0) && (model->edge_faces != 0))
    {
        w3d_build_face_visibility(model, count, face_count);
    }

    edge_count = model->edge_count;
    if (edge_count > WIRE3D_MODEL_EDGE_LIMIT) edge_count = WIRE3D_MODEL_EDGE_LIMIT;
    w3d_build_edge_flags(model, edge_count, face_count, rx, ry, rz);
    i = 0;
    while (i < edge_count)
    {
        const Wire3D_Edge* e;
        e = &model->edges[(__safe_index w3d_u8)i];
        if ((e->a < count) && (e->b < count))
        {
            if (w3d_screen_visible[(__safe_index w3d_u8)e->a] &&
                w3d_screen_visible[(__safe_index w3d_u8)e->b] &&
                w3d_edge_flags[(__safe_index w3d_u8)i])
            {
                Wire3D_DrawLine2D(
                    w3d_screen_x[(__safe_index w3d_u8)e->a],
                    w3d_screen_y[(__safe_index w3d_u8)e->a],
                    w3d_screen_x[(__safe_index w3d_u8)e->b],
                    w3d_screen_y[(__safe_index w3d_u8)e->b]);
            }
        }
        i = (w3d_u8)(i + 1);
    }
}

void Wire3D_EndFrame()
{
    w3d_wait_vblank_start();
    w3d_flush_bg_queue();
    w3d_transfer_stage_asm();
}
