#include "x3d.h"

#pragma bank 1

#define X3D_TILE_W ((X3D_u8)16)
#define X3D_TILE_H ((X3D_u8)15)
#define X3D_ROW_BYTES ((X3D_u8)0x78)
#define X3D_NEAR_Z ((X3D_i16)8)
#define X3D_FAR_Z ((X3D_i16)255)
#define X3D_CENTER_X ((X3D_i16)64)
#define X3D_CENTER_Y ((X3D_i16)60)
#define X3D_TRANSFORM_LIMIT ((X3D_i16)220)
#define X3D_PROJECT_LIMIT ((X3D_i16)120)
#define X3D_BG_QUEUE_LIMIT ((X3D_u8)48)
#define X3D_AUX_ROWS ((X3D_u8)6)
#define X3D_AUX_COL_BYTES ((X3D_u8)0x38)

__location(0xFF40) X3D_u8 X3D_reg_lcdc;
__location(0xFF42) X3D_u8 X3D_reg_scy;
__location(0xFF43) X3D_u8 X3D_reg_scx;
__location(0xFF44) X3D_u8 X3D_reg_ly;
__location(0xFF47) X3D_u8 X3D_reg_bgp;

__location(0x8000) X3D_u8 X3D_vram_tiles[6144];
__location(0x9800) X3D_u8 X3D_bg_map_9800[1024];
__location(0xD000) X3D_u8 X3D_stage[4096];
__location(0xD518) X3D_u8 X3D_stage_aux[1536];

__prg_rom X3D_u8 X3D_bit_mask[8] = {
    0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01
};



__prg_rom X3D_i8 X3D_sin_q6[16] = {
     0,  24,  45,  59,  64,  59,  45,  24,
     0, -24, -45, -59, -64, -59, -45, -24
};

__prg_rom X3D_i8 X3D_cos_q6[16] = {
     64,  59,  45,  24,   0, -24, -45, -59,
    -64, -59, -45, -24,   0,  24,  45,  59
};

__prg_rom X3D_u8 X3D_inv_depth[256] = {
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

X3D_i16 X3D_cam_x;
X3D_i16 X3D_cam_y;
X3D_i16 X3D_cam_z;
X3D_u8 X3D_cam_pitch;
X3D_u8 X3D_cam_yaw;
X3D_u8 X3D_cam_roll;

X3D_u8 X3D_plot_x;
X3D_u8 X3D_plot_y;
X3D_u8 X3D_plot_tx;
X3D_u8 X3D_line_x0;
X3D_u8 X3D_line_y0;
X3D_u8 X3D_line_x1;
X3D_u8 X3D_line_y1;
X3D_u8 X3D_line_x;
X3D_u8 X3D_line_y;
X3D_u8 X3D_line_dx;
X3D_u8 X3D_line_dy;
X3D_u8 X3D_line_sx;
X3D_u8 X3D_line_sy;
X3D_u8 X3D_line_err;

X3D_u8 X3D_screen_x[X3D_MODEL_VERTEX_LIMIT];
X3D_u8 X3D_screen_y[X3D_MODEL_VERTEX_LIMIT];
X3D_u8 X3D_screen_visible[X3D_MODEL_VERTEX_LIMIT];
X3D_u8 X3D_face_visible[X3D_MODEL_FACE_LIMIT];
X3D_u8 X3D_occlusion_mask[2048];
X3D_u8 X3D_occlusion_active;
X3D_u8 X3D_aux_transfer_enabled;
X3D_u8 X3D_dirty_transfer_enabled;
X3D_u8 X3D_dirty_tiles[240];
X3D_u8 X3D_prev_dirty_tiles[240];
X3D_u8 X3D_dirty_src_hi;
X3D_u8 X3D_dirty_src_lo;
X3D_u8 X3D_dirty_dst_hi;
X3D_u8 X3D_dirty_dst_lo;
X3D_u8 X3D_scene_order[X3D_SCENE_OBJECT_LIMIT];
X3D_i16 X3D_scene_depth[X3D_SCENE_OBJECT_LIMIT];
X3D_u8 X3D_bgq_x[48];
X3D_u8 X3D_bgq_y[48];
X3D_u8 X3D_bgq_tile[48];
X3D_u8 X3D_bgq_count;
X3D_u8 X3D_bgq_addr_hi;
X3D_u8 X3D_bgq_addr_lo;
X3D_u8 X3D_bgq_value;

void X3D_clear_vram_asm();
void X3D_fill_bg_map_asm();
void X3D_clear_occlusion_mask_asm();
void X3D_put_bg_tile_safe_asm();
void X3D_clear_stage_asm();
void X3D_plot_stage_asm();
void X3D_line_stage_asm();
void X3D_transfer_stage_asm();
void X3D_transfer_dirty_tile_asm();
void X3D_clear_stage_aux_asm();
void X3D_transfer_stage_aux_asm();

static X3D_i16 X3D_clamp_i16(X3D_i16 v, X3D_i16 lo, X3D_i16 hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

static X3D_u8 X3D_clamp_screen(X3D_i16 v, X3D_u8 max)
{
    if (v < 0) return 0;
    if (v > (X3D_i16)max) return max;
    return (X3D_u8)v;
}

static X3D_u8 X3D_angle_index(X3D_u8 v)
{
    while (v >= X3D_ANGLE_STEPS) v = (X3D_u8)(v - X3D_ANGLE_STEPS);
    return v;
}

static X3D_u8 X3D_neg_angle(X3D_u8 v)
{
    v = X3D_angle_index(v);
    if (v == 0) return 0;
    return (X3D_u8)(X3D_ANGLE_STEPS - v);
}

static void X3D_rotate_y(X3D_i16* px, X3D_i16* pz, X3D_u8 angle)
{
    X3D_u8 ai;
    X3D_i16 s;
    X3D_i16 c;
    X3D_i16 in_x;
    X3D_i16 in_z;
    X3D_i16 out_x;
    X3D_i16 out_z;

    ai = X3D_angle_index(angle);
    s = X3D_sin_q6[(__safe_index X3D_u8)ai];
    c = X3D_cos_q6[(__safe_index X3D_u8)ai];
    in_x = X3D_clamp_i16(*px, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    in_z = X3D_clamp_i16(*pz, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    out_x = (X3D_i16)(((in_x * c) + (in_z * s)) >> 6);
    out_z = (X3D_i16)(((in_z * c) - (in_x * s)) >> 6);
    *px = out_x;
    *pz = out_z;
}

static void X3D_rotate_x(X3D_i16* py, X3D_i16* pz, X3D_u8 angle)
{
    X3D_u8 ai;
    X3D_i16 s;
    X3D_i16 c;
    X3D_i16 in_y;
    X3D_i16 in_z;
    X3D_i16 out_y;
    X3D_i16 out_z;

    ai = X3D_angle_index(angle);
    s = X3D_sin_q6[(__safe_index X3D_u8)ai];
    c = X3D_cos_q6[(__safe_index X3D_u8)ai];
    in_y = X3D_clamp_i16(*py, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    in_z = X3D_clamp_i16(*pz, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    out_y = (X3D_i16)(((in_y * c) - (in_z * s)) >> 6);
    out_z = (X3D_i16)(((in_y * s) + (in_z * c)) >> 6);
    *py = out_y;
    *pz = out_z;
}

static void X3D_rotate_z(X3D_i16* px, X3D_i16* py, X3D_u8 angle)
{
    X3D_u8 ai;
    X3D_i16 s;
    X3D_i16 c;
    X3D_i16 in_x;
    X3D_i16 in_y;
    X3D_i16 out_x;
    X3D_i16 out_y;

    ai = X3D_angle_index(angle);
    s = X3D_sin_q6[(__safe_index X3D_u8)ai];
    c = X3D_cos_q6[(__safe_index X3D_u8)ai];
    in_x = X3D_clamp_i16(*px, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    in_y = X3D_clamp_i16(*py, (X3D_i16)(0 - X3D_TRANSFORM_LIMIT), X3D_TRANSFORM_LIMIT);
    out_x = (X3D_i16)(((in_x * c) - (in_y * s)) >> 6);
    out_y = (X3D_i16)(((in_x * s) + (in_y * c)) >> 6);
    *px = out_x;
    *py = out_y;
}

static X3D_u8 X3D_project_camera_space(X3D_i16 vx, X3D_i16 vy, X3D_i16 vz, X3D_u8* sx, X3D_u8* sy)
{
    X3D_u8 iz;
    X3D_i16 px;
    X3D_i16 py;
    X3D_i16 ox;
    X3D_i16 oy;

    if (vz < X3D_NEAR_Z) return 0;
    if (vz > X3D_FAR_Z) return 0;

    px = X3D_clamp_i16(vx, (X3D_i16)(0 - X3D_PROJECT_LIMIT), X3D_PROJECT_LIMIT);
    py = X3D_clamp_i16(vy, (X3D_i16)(0 - X3D_PROJECT_LIMIT), X3D_PROJECT_LIMIT);
    iz = X3D_inv_depth[(__safe_index X3D_u8)((X3D_u8)vz)];

    ox = (X3D_i16)((px * (X3D_i16)iz) >> 5);
    oy = (X3D_i16)((py * (X3D_i16)iz) >> 5);

    *sx = X3D_clamp_screen((X3D_i16)(X3D_CENTER_X + ox), (X3D_u8)(X3D_SCREEN_W - 1));
    *sy = X3D_clamp_screen((X3D_i16)(X3D_CENTER_Y - oy), (X3D_u8)(X3D_SCREEN_H - 1));
    return 1;
}

static X3D_u8 X3D_project_world(X3D_i16 wx, X3D_i16 wy, X3D_i16 wz, X3D_u8* sx, X3D_u8* sy)
{
    X3D_i16 vx;
    X3D_i16 vy;
    X3D_i16 vz;

    vx = (X3D_i16)(wx - X3D_cam_x);
    vy = (X3D_i16)(wy - X3D_cam_y);
    vz = (X3D_i16)(wz - X3D_cam_z);

    X3D_rotate_y(&vx, &vz, X3D_neg_angle(X3D_cam_yaw));
    X3D_rotate_x(&vy, &vz, X3D_neg_angle(X3D_cam_pitch));
    X3D_rotate_z(&vx, &vy, X3D_neg_angle(X3D_cam_roll));

    return X3D_project_camera_space(vx, vy, vz, sx, sy);
}

X3D_u8 X3D_ProjectPoint(X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8* sx, X3D_u8* sy)
{
    return X3D_project_world(x, y, z, sx, sy);
}

void X3D_RotatePoint(X3D_i16* x, X3D_i16* y, X3D_i16* z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz)
{
    if (x == 0) return;
    if (y == 0) return;
    if (z == 0) return;

    X3D_rotate_y(x, z, ry);
    X3D_rotate_x(y, z, rx);
    X3D_rotate_z(x, y, rz);
}

void X3D_PutBgTile(X3D_u8 x, X3D_u8 y, X3D_u8 tile)
{
    if (x >= 32) return;
    if (y >= 32) return;
    if (X3D_bgq_count >= X3D_BG_QUEUE_LIMIT) return;
    X3D_bgq_x[(__safe_index X3D_u8)X3D_bgq_count] = x;
    X3D_bgq_y[(__safe_index X3D_u8)X3D_bgq_count] = y;
    X3D_bgq_tile[(__safe_index X3D_u8)X3D_bgq_count] = tile;
    X3D_bgq_count = (X3D_u8)(X3D_bgq_count + 1);
}

void X3D_SetPalette(X3D_u8 bgp)
{
    X3D_reg_bgp = bgp;
}

static void X3D_flush_bg_queue()
{
    X3D_u8 i;

    i = 0;
    while (i < X3D_bgq_count)
    {
        X3D_u16 off;

        off = (X3D_u16)(((X3D_u16)X3D_bgq_y[(__safe_index X3D_u8)i] << 5) + (X3D_u16)X3D_bgq_x[(__safe_index X3D_u8)i]);
        off = (X3D_u16)(off + 0x9800);
        X3D_bgq_addr_hi = (X3D_u8)(off >> 8);
        X3D_bgq_addr_lo = (X3D_u8)off;
        X3D_bgq_value = X3D_bgq_tile[(__safe_index X3D_u8)i];
        X3D_put_bg_tile_safe_asm();
        i = (X3D_u8)(i + 1);
    }
    X3D_bgq_count = 0;
}

static X3D_i16 X3D_scene_object_depth(const X3D_Object* obj)
{
    X3D_i16 vx;
    X3D_i16 vy;
    X3D_i16 vz;

    vx = (X3D_i16)(obj->x - X3D_cam_x);
    vy = (X3D_i16)(obj->y - X3D_cam_y);
    vz = (X3D_i16)(obj->z - X3D_cam_z);

    X3D_rotate_y(&vx, &vz, X3D_neg_angle(X3D_cam_yaw));
    X3D_rotate_x(&vy, &vz, X3D_neg_angle(X3D_cam_pitch));
    X3D_rotate_z(&vx, &vy, X3D_neg_angle(X3D_cam_roll));

    return vz;
}

static X3D_i16 X3D_abs_i16(X3D_i16 v)
{
    if (v < 0) return (X3D_i16)(0 - v);
    return v;
}

static void X3D_clear_occlusion_mask()
{
    X3D_clear_occlusion_mask_asm();
}

static void X3D_clear_dirty_tables()
{
    X3D_u8 i;

    i = 0;
    while (i < 240)
    {
        X3D_dirty_tiles[(__safe_index X3D_u8)i] = 0;
        X3D_prev_dirty_tiles[(__safe_index X3D_u8)i] = 0;
        i = (X3D_u8)(i + 1);
    }
}

static void X3D_mark_tile_dirty(X3D_u8 tx, X3D_u8 ty)
{
    X3D_u8 idx;

    if (tx >= X3D_TILE_W) return;
    if (ty >= X3D_TILE_H) return;
    idx = (X3D_u8)((X3D_u8)(tx * X3D_TILE_H) + ty);
    X3D_dirty_tiles[(__safe_index X3D_u8)idx] = 1;
}

static void X3D_mark_rect_dirty(X3D_u8 x0, X3D_u8 y0, X3D_u8 x1, X3D_u8 y1)
{
    X3D_u8 tx;
    X3D_u8 ty;
    X3D_u8 tx0;
    X3D_u8 tx1;
    X3D_u8 ty0;
    X3D_u8 ty1;

    if (X3D_dirty_transfer_enabled == 0) return;
    if (x0 >= X3D_SCREEN_W) x0 = (X3D_u8)(X3D_SCREEN_W - 1);
    if (x1 >= X3D_SCREEN_W) x1 = (X3D_u8)(X3D_SCREEN_W - 1);
    if (y0 >= X3D_SCREEN_H) y0 = (X3D_u8)(X3D_SCREEN_H - 1);
    if (y1 >= X3D_SCREEN_H) y1 = (X3D_u8)(X3D_SCREEN_H - 1);
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

    tx0 = (X3D_u8)(x0 >> 3);
    tx1 = (X3D_u8)(x1 >> 3);
    ty0 = (X3D_u8)(y0 >> 3);
    ty1 = (X3D_u8)(y1 >> 3);

    tx = tx0;
    while (tx <= tx1)
    {
        ty = ty0;
        while (ty <= ty1)
        {
            X3D_mark_tile_dirty(tx, ty);
            if (ty == ty1) break;
            ty = (X3D_u8)(ty + 1);
        }
        if (tx == tx1) break;
        tx = (X3D_u8)(tx + 1);
    }
}

static void X3D_mark_line_dirty(X3D_u8 x0, X3D_u8 y0, X3D_u8 x1, X3D_u8 y1)
{
    X3D_mark_rect_dirty(x0, y0, x1, y1);
}

static X3D_u16 X3D_mask_offset(X3D_u8 x, X3D_u8 y)
{
    return (X3D_u16)(((X3D_u16)y << 4) + (X3D_u16)(x >> 3));
}

static X3D_u8 X3D_mask_get(X3D_u8 x, X3D_u8 y)
{
    X3D_u16 ofs;
    X3D_u8 bit;

    if (x >= X3D_SCREEN_W) return 1;
    if (y >= X3D_SCREEN_H) return 1;

    ofs = X3D_mask_offset(x, y);
    bit = X3D_bit_mask[(__safe_index X3D_u8)(x & 7)];
    if ((X3D_occlusion_mask[(__safe_index X3D_u16)ofs] & bit) != 0) return 1;
    return 0;
}

static void X3D_mask_set(X3D_u8 x, X3D_u8 y)
{
    X3D_u16 ofs;
    X3D_u8 bit;

    if (x >= X3D_SCREEN_W) return;
    if (y >= X3D_SCREEN_H) return;

    ofs = X3D_mask_offset(x, y);
    bit = X3D_bit_mask[(__safe_index X3D_u8)(x & 7)];
    X3D_occlusion_mask[(__safe_index X3D_u16)ofs] = (X3D_u8)(X3D_occlusion_mask[(__safe_index X3D_u16)ofs] | bit);
}

static void X3D_stage_clear_pixel(X3D_u8 x, X3D_u8 y)
{
    X3D_u16 ofs;
    X3D_u8 bit;

    if (x >= X3D_SCREEN_W) return;
    if (y >= X3D_SCREEN_H) return;

    ofs = (X3D_u16)((((X3D_u16)(x >> 3)) << 8) + (((X3D_u16)(y >> 3)) << 3) + (X3D_u16)(y & 7));
    bit = X3D_bit_mask[(__safe_index X3D_u8)(x & 7)];
    X3D_stage[(__safe_index X3D_u16)ofs] = (X3D_u8)(X3D_stage[(__safe_index X3D_u16)ofs] & (X3D_u8)(0xFF ^ bit));
}

static void X3D_stage_clear_span(X3D_u8 y, X3D_u8 min_x, X3D_u8 max_x)
{
    X3D_u8 x;
    X3D_u16 ofs;

    if (y >= X3D_SCREEN_H) return;
    X3D_mark_rect_dirty(min_x, y, max_x, y);

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        X3D_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (X3D_u8)(x + 1);
    }

    while ((X3D_u8)(x + 7) <= max_x)
    {
        ofs = (X3D_u16)((((X3D_u16)(x >> 3)) << 8) + (((X3D_u16)(y >> 3)) << 3) + (X3D_u16)(y & 7));
        X3D_stage[(__safe_index X3D_u16)ofs] = 0;
        x = (X3D_u8)(x + 8);
    }

    while (x <= max_x)
    {
        X3D_stage_clear_pixel(x, y);
        if (x == max_x) return;
        x = (X3D_u8)(x + 1);
    }
}

static void X3D_mask_set_span(X3D_u8 y, X3D_u8 min_x, X3D_u8 max_x)
{
    X3D_u8 x;
    X3D_u16 ofs;

    if (y >= X3D_SCREEN_H) return;

    x = min_x;
    while ((x <= max_x) && ((x & 7) != 0))
    {
        X3D_mask_set(x, y);
        if (x == max_x) return;
        x = (X3D_u8)(x + 1);
    }

    while ((X3D_u8)(x + 7) <= max_x)
    {
        ofs = X3D_mask_offset(x, y);
        X3D_occlusion_mask[(__safe_index X3D_u16)ofs] = 0xFF;
        x = (X3D_u8)(x + 8);
    }

    while (x <= max_x)
    {
        X3D_mask_set(x, y);
        if (x == max_x) return;
        x = (X3D_u8)(x + 1);
    }
}

static void X3D_line_stage_masked_c(X3D_u8 x0, X3D_u8 y0, X3D_u8 x1, X3D_u8 y1)
{
    X3D_u8 mx;
    X3D_u8 my;
    X3D_u8 q0x;
    X3D_u8 q0y;
    X3D_u8 q1x;
    X3D_u8 q1y;

    mx = (X3D_u8)(((X3D_u16)x0 + (X3D_u16)x1) >> 1);
    my = (X3D_u8)(((X3D_u16)y0 + (X3D_u16)y1) >> 1);
    q0x = (X3D_u8)(((X3D_u16)x0 + (X3D_u16)mx) >> 1);
    q0y = (X3D_u8)(((X3D_u16)y0 + (X3D_u16)my) >> 1);
    q1x = (X3D_u8)(((X3D_u16)x1 + (X3D_u16)mx) >> 1);
    q1y = (X3D_u8)(((X3D_u16)y1 + (X3D_u16)my) >> 1);

    if (X3D_mask_get(x0, y0) &&
        X3D_mask_get(q0x, q0y) &&
        X3D_mask_get(mx, my) &&
        X3D_mask_get(q1x, q1y) &&
        X3D_mask_get(x1, y1))
    {
        return;
    }

    X3D_line_x0 = x0;
    X3D_line_y0 = y0;
    X3D_line_x1 = x1;
    X3D_line_y1 = y1;
    X3D_line_stage_asm();
}

static X3D_i16 X3D_screen_delta(X3D_u8 a, X3D_u8 b)
{
    return (X3D_i16)((X3D_i16)a - (X3D_i16)b);
}

static X3D_i16 X3D_edge_area(X3D_i16 ax, X3D_i16 ay, X3D_i16 bx, X3D_i16 by, X3D_i16 px, X3D_i16 py)
{
    return (X3D_i16)(((bx - ax) * (py - ay)) - ((by - ay) * (px - ax)));
}

static void X3D_mark_triangle(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by, X3D_u8 cx, X3D_u8 cy)
{
    X3D_u8 min_x;
    X3D_u8 max_x;
    X3D_u8 min_y;
    X3D_u8 max_y;
    X3D_i16 area;
    X3D_u8 y;

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

    area = X3D_edge_area((X3D_i16)ax, (X3D_i16)ay, (X3D_i16)bx, (X3D_i16)by, (X3D_i16)cx, (X3D_i16)cy);
    if (area == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        X3D_mask_set_span(y, min_x, max_x);
        if (y == max_y) break;
        y = (X3D_u8)(y + 1);
    }
}

static void X3D_clear_triangle(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by, X3D_u8 cx, X3D_u8 cy)
{
    X3D_u8 min_x;
    X3D_u8 max_x;
    X3D_u8 min_y;
    X3D_u8 max_y;
    X3D_u8 y;

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

    if (X3D_edge_area((X3D_i16)ax, (X3D_i16)ay, (X3D_i16)bx, (X3D_i16)by, (X3D_i16)cx, (X3D_i16)cy) == 0) return;

    y = min_y;
    while (y <= max_y)
    {
        X3D_u8 hits;
        X3D_i16 span_min;
        X3D_i16 span_max;
        X3D_i16 dx;
        X3D_i16 dy;
        X3D_i16 ix;

        hits = 0;
        span_min = 127;
        span_max = 0;

        if (ay != by)
        {
            if (((y >= ay) && (y <= by)) || ((y >= by) && (y <= ay)))
            {
                dx = (X3D_i16)((X3D_i16)bx - (X3D_i16)ax);
                dy = (X3D_i16)((X3D_i16)by - (X3D_i16)ay);
                ix = (X3D_i16)((X3D_i16)ax + ((((X3D_i16)y - (X3D_i16)ay) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (X3D_u8)(hits + 1);
            }
        }

        if (by != cy)
        {
            if (((y >= by) && (y <= cy)) || ((y >= cy) && (y <= by)))
            {
                dx = (X3D_i16)((X3D_i16)cx - (X3D_i16)bx);
                dy = (X3D_i16)((X3D_i16)cy - (X3D_i16)by);
                ix = (X3D_i16)((X3D_i16)bx + ((((X3D_i16)y - (X3D_i16)by) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (X3D_u8)(hits + 1);
            }
        }

        if (cy != ay)
        {
            if (((y >= cy) && (y <= ay)) || ((y >= ay) && (y <= cy)))
            {
                dx = (X3D_i16)((X3D_i16)ax - (X3D_i16)cx);
                dy = (X3D_i16)((X3D_i16)ay - (X3D_i16)cy);
                ix = (X3D_i16)((X3D_i16)cx + ((((X3D_i16)y - (X3D_i16)cy) * dx) / dy));
                if (ix < span_min) span_min = ix;
                if (ix > span_max) span_max = ix;
                hits = (X3D_u8)(hits + 1);
            }
        }

        if (hits >= 2)
        {
            if (span_min < (X3D_i16)min_x) span_min = (X3D_i16)min_x;
            if (span_max > (X3D_i16)max_x) span_max = (X3D_i16)max_x;
            if (span_min < 0) span_min = 0;
            if (span_max > (X3D_i16)(X3D_SCREEN_W - 1)) span_max = (X3D_i16)(X3D_SCREEN_W - 1);
            if (span_min <= span_max)
            {
                X3D_stage_clear_span(y, (X3D_u8)span_min, (X3D_u8)span_max);
            }
        }

        if (y == max_y) break;
        y = (X3D_u8)(y + 1);
    }
}

static void X3D_build_face_visibility(const X3D_Model* model, X3D_u8 count, X3D_u8 face_count)
{
    X3D_u8 i;

    i = 0;
    while (i < face_count)
    {
        const X3D_Face* f;
        X3D_i16 abx;
        X3D_i16 aby;
        X3D_i16 acx;
        X3D_i16 acy;
        X3D_i16 area;

        X3D_face_visible[(__safe_index X3D_u8)i] = 0;
        f = &model->faces[(__safe_index X3D_u8)i];

        if ((f->a < count) && (f->b < count) && (f->c < count))
        {
            if (X3D_screen_visible[(__safe_index X3D_u8)f->a] &&
                X3D_screen_visible[(__safe_index X3D_u8)f->b] &&
                X3D_screen_visible[(__safe_index X3D_u8)f->c])
            {
                abx = X3D_screen_delta(X3D_screen_x[(__safe_index X3D_u8)f->b], X3D_screen_x[(__safe_index X3D_u8)f->a]);
                aby = X3D_screen_delta(X3D_screen_y[(__safe_index X3D_u8)f->b], X3D_screen_y[(__safe_index X3D_u8)f->a]);
                acx = X3D_screen_delta(X3D_screen_x[(__safe_index X3D_u8)f->c], X3D_screen_x[(__safe_index X3D_u8)f->a]);
                acy = X3D_screen_delta(X3D_screen_y[(__safe_index X3D_u8)f->c], X3D_screen_y[(__safe_index X3D_u8)f->a]);
                area = (X3D_i16)((abx * acy) - (aby * acx));
                if (area > 0)
                {
                    X3D_face_visible[(__safe_index X3D_u8)i] = 1;
                }
            }
        }
        i = (X3D_u8)(i + 1);
    }
}

static X3D_u8 X3D_is_edge_visible(const X3D_Model* model, X3D_u8 edge_index, X3D_u8 face_count)
{
    const X3D_EdgeFaces* ef;
    X3D_u8 any_face;

    if ((model->flags & X3D_MODEL_HIDDEN_LINES) == 0) return 1;
    if (model->faces == 0) return 1;
    if (model->edge_faces == 0) return 1;

    ef = &model->edge_faces[(__safe_index X3D_u8)edge_index];
    any_face = 0;

    if (ef->f0 != X3D_FACE_NONE)
    {
        any_face = 1;
        if (ef->f0 >= face_count) return 1;
        if (X3D_face_visible[(__safe_index X3D_u8)ef->f0]) return 1;
    }

    if (ef->f1 != X3D_FACE_NONE)
    {
        any_face = 1;
        if (ef->f1 >= face_count) return 1;
        if (X3D_face_visible[(__safe_index X3D_u8)ef->f1]) return 1;
    }

    if (any_face == 0) return 1;
    return 0;
}

static void X3D_mark_model_occluder(const X3D_Model* model)
{
    X3D_u8 count;
    X3D_u8 face_count;
    X3D_u8 i;

    if (model == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & X3D_MODEL_HIDDEN_LINES) == 0) return;

    count = model->vertex_count;
    if (count > X3D_MODEL_VERTEX_LIMIT) count = X3D_MODEL_VERTEX_LIMIT;
    face_count = model->face_count;
    if (face_count > X3D_MODEL_FACE_LIMIT) face_count = X3D_MODEL_FACE_LIMIT;

    X3D_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const X3D_Face* f;
        f = &model->faces[(__safe_index X3D_u8)i];
        if (X3D_face_visible[(__safe_index X3D_u8)i])
        {
            X3D_mark_triangle(
                X3D_screen_x[(__safe_index X3D_u8)f->a],
                X3D_screen_y[(__safe_index X3D_u8)f->a],
                X3D_screen_x[(__safe_index X3D_u8)f->b],
                X3D_screen_y[(__safe_index X3D_u8)f->b],
                X3D_screen_x[(__safe_index X3D_u8)f->c],
                X3D_screen_y[(__safe_index X3D_u8)f->c]);
        }
        i = (X3D_u8)(i + 1);
    }
}

void X3D_wait_vblank_start()
{
    if ((X3D_reg_lcdc & 0x80) == 0) return;
    while (X3D_reg_ly >= 144) { }
    while (X3D_reg_ly < 144) { }
}

#pragma fixed_bank 1
#pragma fixed_order 92
void X3D_clear_vram_asm()
{
    __asm {
x3dcv_enter:
        LD_HL_IMM X3D_vram_tiles
        XOR_A
        LD_D_IMM 24

x3dcv_page:
        LD_C_IMM 0

x3dcv_loop:
        LDI_HL_A
        DEC_C
        JR_NZ x3dcv_loop
        DEC_D
        JR_NZ x3dcv_page
        RET
    }
}

#pragma fixed_order 94
void X3D_fill_bg_map_asm()
{
    __asm {
x3dfb_enter:
        LD_HL_IMM X3D_bg_map_9800
        LD_A_IMM 0x80
        LD_D_IMM 4

x3dfb_page:
        LD_C_IMM 0

x3dfb_loop:
        LDI_HL_A
        DEC_C
        JR_NZ x3dfb_loop
        DEC_D
        JR_NZ x3dfb_page
        RET
    }
}

#pragma fixed_order 96
void X3D_clear_occlusion_mask_asm()
{
    __asm {
x3dco_enter:
        LD_HL_IMM X3D_occlusion_mask
        XOR_A
        LD_D_IMM 8

x3dco_page:
        LD_C_IMM 0

x3dco_loop:
        LDI_HL_A
        DEC_C
        JR_NZ x3dco_loop
        DEC_D
        JR_NZ x3dco_page
        RET
    }
}

#pragma fixed_order 98
void X3D_put_bg_tile_safe_asm()
{
    __asm {
x3dbg_enter:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ x3dbg_enter
        LD_A_MEM X3D_bgq_addr_hi
        LD_H_A
        LD_A_MEM X3D_bgq_addr_lo
        LD_L_A
        LD_A_MEM X3D_bgq_value
        LD_HL_A
        RET
    }
}

#pragma fixed_order 100
void X3D_clear_stage_asm()
{
    __asm {
x3dcs_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD0
        LD_C_IMM 16

x3dcs_outer:
        LD_L_IMM 0
        LD_B_IMM 0x78

x3dcs_inner:
        LDI_HL_A
        DEC_B
        JR_NZ x3dcs_inner
        INC_H
        DEC_C
        JR_NZ x3dcs_outer
        RET
    }
}

#pragma fixed_order 110
void X3D_plot_stage_asm()
{
    __asm {
x3dps_enter:
        LD_A_MEM X3D_plot_x
        CP_IMM 0x80
        JP_NC x3dps_ret

        LD_A_MEM X3D_plot_y
        CP_IMM 0x78
        JP_NC x3dps_ret

        LD_A_MEM X3D_plot_x
        OR_A
        RRA
        OR_A
        RRA
        OR_A
        RRA
        LD_MEM_A X3D_plot_tx

        LD_A_MEM X3D_plot_tx
        LD_B_A
        LD_A_IMM 0xD0
        ADD_B
        LD_H_A

        LD_A_MEM X3D_plot_y
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

        LD_A_MEM X3D_plot_y
        AND_IMM 7
        ADD_B
        LD_L_A

        LD_A_MEM X3D_plot_x
        AND_IMM 7
        LD_E_A
        LD_D_IMM 0
        PUSH_HL
        LD_HL_IMM X3D_bit_mask
        ADD_HL_DE
        LD_A_HL
        LD_B_A
        POP_HL

        LD_A_HL
        OR_B
        LD_HL_A

x3dps_ret:
        RET
    }
}

#pragma fixed_order 120
void X3D_line_stage_asm()
{
    __asm {
x3dls_enter:
        LD_A_MEM X3D_line_x0
        LD_B_A
        LD_A_MEM X3D_line_x1
        CP_B
        JP_C x3dls_x_reverse
        SUB_B
        LD_MEM_A X3D_line_dx
        LD_A_IMM 1
        LD_MEM_A X3D_line_sx
        JP x3dls_y_start

x3dls_x_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A X3D_line_dx
        LD_A_IMM 255
        LD_MEM_A X3D_line_sx

x3dls_y_start:
        LD_A_MEM X3D_line_y0
        LD_B_A
        LD_A_MEM X3D_line_y1
        CP_B
        JP_C x3dls_y_reverse
        SUB_B
        LD_MEM_A X3D_line_dy
        LD_A_IMM 1
        LD_MEM_A X3D_line_sy
        JP x3dls_branch

x3dls_y_reverse:
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A X3D_line_dy
        LD_A_IMM 255
        LD_MEM_A X3D_line_sy

x3dls_branch:
        LD_A_MEM X3D_line_dx
        LD_B_A
        LD_A_MEM X3D_line_dy
        CP_B
        JP_C x3dls_shallow
        JP_Z x3dls_shallow
        JP x3dls_steep

x3dls_shallow:
        LD_A_MEM X3D_line_x0
        LD_MEM_A X3D_line_x
        LD_A_MEM X3D_line_y0
        LD_MEM_A X3D_line_y
        LD_A_MEM X3D_line_dx
        OR_A
        RRA
        LD_MEM_A X3D_line_err

x3dls_shallow_loop:
        LD_A_MEM X3D_line_x
        LD_MEM_A X3D_plot_x
        LD_A_MEM X3D_line_y
        LD_MEM_A X3D_plot_y
        CALL X3D_plot_stage_asm

        LD_A_MEM X3D_line_x
        LD_B_A
        LD_A_MEM X3D_line_x1
        CP_B
        JP_Z x3dls_done

        LD_A_MEM X3D_line_dy
        LD_B_A
        LD_A_MEM X3D_line_err
        CP_B
        JP_NC x3dls_shallow_skip_bridge

        LD_A_MEM X3D_line_y
        LD_B_A
        LD_A_MEM X3D_line_sy
        ADD_B
        LD_MEM_A X3D_line_y

        LD_A_MEM X3D_line_x
        LD_MEM_A X3D_plot_x
        LD_A_MEM X3D_line_y
        LD_MEM_A X3D_plot_y
        CALL X3D_plot_stage_asm

        LD_A_MEM X3D_line_dx
        LD_B_A
        LD_A_MEM X3D_line_err
        ADD_B
        LD_MEM_A X3D_line_err

x3dls_shallow_skip_bridge:
        LD_A_MEM X3D_line_err
        LD_B_A
        LD_A_MEM X3D_line_dy
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A X3D_line_err

        LD_A_MEM X3D_line_x
        LD_B_A
        LD_A_MEM X3D_line_sx
        ADD_B
        LD_MEM_A X3D_line_x
        JP x3dls_shallow_loop

x3dls_steep:
        LD_A_MEM X3D_line_x0
        LD_MEM_A X3D_line_x
        LD_A_MEM X3D_line_y0
        LD_MEM_A X3D_line_y
        LD_A_MEM X3D_line_dy
        OR_A
        RRA
        LD_MEM_A X3D_line_err

x3dls_steep_loop:
        LD_A_MEM X3D_line_x
        LD_MEM_A X3D_plot_x
        LD_A_MEM X3D_line_y
        LD_MEM_A X3D_plot_y
        CALL X3D_plot_stage_asm

        LD_A_MEM X3D_line_y
        LD_B_A
        LD_A_MEM X3D_line_y1
        CP_B
        JP_Z x3dls_done

        LD_A_MEM X3D_line_dx
        LD_B_A
        LD_A_MEM X3D_line_err
        CP_B
        JP_NC x3dls_steep_skip_bridge

        LD_A_MEM X3D_line_x
        LD_B_A
        LD_A_MEM X3D_line_sx
        ADD_B
        LD_MEM_A X3D_line_x

        LD_A_MEM X3D_line_x
        LD_MEM_A X3D_plot_x
        LD_A_MEM X3D_line_y
        LD_MEM_A X3D_plot_y
        CALL X3D_plot_stage_asm

        LD_A_MEM X3D_line_dy
        LD_B_A
        LD_A_MEM X3D_line_err
        ADD_B
        LD_MEM_A X3D_line_err

x3dls_steep_skip_bridge:
        LD_A_MEM X3D_line_err
        LD_B_A
        LD_A_MEM X3D_line_dx
        LD_C_A
        LD_A_B
        SUB_C
        LD_MEM_A X3D_line_err

        LD_A_MEM X3D_line_y
        LD_B_A
        LD_A_MEM X3D_line_sy
        ADD_B
        LD_MEM_A X3D_line_y
        JP x3dls_steep_loop

x3dls_done:
        RET
    }
}

#pragma fixed_order 130
void X3D_transfer_stage_asm()
{
    __asm {
x3dtf_enter:
        LD_HL_IMM 0x8900
        LD_DE_IMM X3D_stage
        LD_B_IMM 16

        INC_L
x3dtf_row:
        PUSH_BC

x3dtf_col:
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

x3dtf_wait:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ x3dtf_wait

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
        CP_IMM 0x78
        JR_C x3dtf_col

        LD_E_IMM 0
        INC_D
        POP_BC
        DEC_B
        JR_NZ x3dtf_row
        RET
    }
}

#pragma fixed_order 132
void X3D_transfer_dirty_tile_asm()
{
    __asm {
x3ddt_enter:
        LD_A_MEM X3D_dirty_src_hi
        LD_H_A
        LD_A_MEM X3D_dirty_src_lo
        LD_L_A
        LD_A_MEM X3D_dirty_dst_hi
        LD_D_A
        LD_A_MEM X3D_dirty_dst_lo
        LD_E_A
        LD_B_IMM 8

x3ddt_loop:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ x3ddt_loop

        LD_A_HL
        LD_DE_A
        XOR_A
        LD_HL_A
        INC_HL
        INC_DE
        INC_DE
        DEC_B
        JR_NZ x3ddt_loop
        RET
    }
}

#pragma fixed_order 133
void X3D_clear_stage_aux_asm()
{
    __asm {
x3dac_enter:
        LD_A_IMM 0
        LD_H_IMM 0xD5
        LD_C_IMM 6

x3dac_row:
        LD_L_IMM 0x18
        LD_B_IMM 0x38

x3dac_col:
        LDI_HL_A
        DEC_B
        JR_NZ x3dac_col
        INC_H
        DEC_C
        JR_NZ x3dac_row
        RET
    }
}

#pragma fixed_order 134
void X3D_transfer_stage_aux_asm()
{
    __asm {
x3dta_enter:
        LD_HL_IMM 0x90A0
        LD_DE_IMM X3D_stage_aux
        LD_B_IMM 6

        INC_L
x3dta_row:
        PUSH_BC

x3dta_col:
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

x3dta_wait:
        LDH_A_MEM 65
        AND_IMM 0x02
        JR_NZ x3dta_wait

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
        CP_IMM 0x50
        JR_C x3dta_col

        LD_E_IMM 0x18
        INC_D
        LD_A_L
        ADD_A_IMM 0x40
        LD_L_A
        LD_A_H
        ADC_IMM 0
        LD_H_A
        POP_BC
        DEC_B
        JR_NZ x3dta_row
        RET
    }
}
#pragma fixed_order -1
#pragma fixed_bank -1

void X3D_Init()
{
    X3D_u8 row;
    X3D_u8 col;
    X3D_u8 tid;
    X3D_u16 off;

    X3D_wait_vblank_start();
    X3D_reg_lcdc = 0;

    X3D_clear_vram_asm();
    X3D_fill_bg_map_asm();

    row = 0;
    tid = 0x90;
    while (row < X3D_TILE_H)
    {
        off = (X3D_u16)(0x23 + ((X3D_u16)row << 5));
        col = 0;
        while (col < X3D_TILE_W)
        {
            X3D_bg_map_9800[(X3D_u16)(off + (X3D_u16)col)] = tid;
            tid = (X3D_u8)(tid + X3D_TILE_H);
            col = (X3D_u8)(col + 1);
        }
        row = (X3D_u8)(row + 1);
        tid = (X3D_u8)(0x90 + row);
    }

    X3D_reg_scx = 8;
    X3D_reg_scy = 0;
    X3D_reg_bgp = 0xB4;
    X3D_reg_lcdc = 0x81;
    X3D_bgq_count = 0;
    X3D_aux_transfer_enabled = 0;
    X3D_dirty_transfer_enabled = 0;

    X3D_SetCamera(0, 0, 0, 0, 0, 0);
    X3D_clear_stage_asm();
    X3D_clear_stage_aux_asm();
    X3D_clear_dirty_tables();
}

void X3D_BeginFrame()
{
    /* The X-style stage transfer clears D000 as it uploads, so BeginFrame only
       resets draw state. X3D_Init supplies the first clean buffer. */
    X3D_occlusion_active = 0;
}

void X3D_SetCamera(X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 pitch, X3D_u8 yaw, X3D_u8 roll)
{
    X3D_cam_x = x;
    X3D_cam_y = y;
    X3D_cam_z = z;
    X3D_cam_pitch = pitch;
    X3D_cam_yaw = yaw;
    X3D_cam_roll = roll;
}

void X3D_DrawLine2D(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by)
{
    X3D_mark_line_dirty(ax, ay, bx, by);

    if (X3D_occlusion_active)
    {
        X3D_line_stage_masked_c(ax, ay, bx, by);
        return;
    }

    X3D_line_x0 = ax;
    X3D_line_y0 = ay;
    X3D_line_x1 = bx;
    X3D_line_y1 = by;
    X3D_line_stage_asm();
}

void X3D_DrawScene(X3D_Object* objects, X3D_u8 count)
{
    X3D_u8 scene_count;
    X3D_u8 i;

    if (objects == 0) return;

    scene_count = count;
    if (scene_count > X3D_SCENE_OBJECT_LIMIT) scene_count = X3D_SCENE_OBJECT_LIMIT;
    if (scene_count == 0) return;

    i = 0;
    while (i < scene_count)
    {
        X3D_scene_order[(__safe_index X3D_u8)i] = i;
        X3D_scene_depth[(__safe_index X3D_u8)i] = X3D_scene_object_depth(&objects[(__safe_index X3D_u8)i]);
        i = (X3D_u8)(i + 1);
    }

    i = 0;
    while (i < scene_count)
    {
        X3D_u8 j;
        j = (X3D_u8)(i + 1);
        while (j < scene_count)
        {
            X3D_u8 oi;
            X3D_u8 oj;
            oi = X3D_scene_order[(__safe_index X3D_u8)i];
            oj = X3D_scene_order[(__safe_index X3D_u8)j];

            if (X3D_scene_depth[(__safe_index X3D_u8)oj] < X3D_scene_depth[(__safe_index X3D_u8)oi])
            {
                X3D_scene_order[(__safe_index X3D_u8)i] = oj;
                X3D_scene_order[(__safe_index X3D_u8)j] = oi;
            }
            j = (X3D_u8)(j + 1);
        }
        i = (X3D_u8)(i + 1);
    }

    X3D_clear_occlusion_mask();
    X3D_occlusion_active = 0;

    i = 0;
    while (i < scene_count)
    {
        X3D_Object* obj;
        obj = &objects[(__safe_index X3D_u8)X3D_scene_order[(__safe_index X3D_u8)i]];
        if ((obj->visible != 0) && (obj->model != 0))
        {
            X3D_occlusion_active = i == 0 ? 0 : 1;
            X3D_DrawModelScaled(obj->model, obj->x, obj->y, obj->z, obj->rx, obj->ry, obj->rz, obj->scale_q8);
            if ((X3D_u8)(i + 1) < scene_count)
            {
                X3D_mark_model_occluder(obj->model);
            }
        }
        i = (X3D_u8)(i + 1);
    }

    X3D_occlusion_active = 0;
}

void X3D_DrawLine3D(X3D_i16 ax, X3D_i16 ay, X3D_i16 az, X3D_i16 bx, X3D_i16 by, X3D_i16 bz)
{
    X3D_u8 sx0;
    X3D_u8 sy0;
    X3D_u8 sx1;
    X3D_u8 sy1;

    if (X3D_project_world(ax, ay, az, &sx0, &sy0) == 0) return;
    if (X3D_project_world(bx, by, bz, &sx1, &sy1) == 0) return;
    X3D_DrawLine2D(sx0, sy0, sx1, sy1);
}

void X3D_DrawModel(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz)
{
    X3D_DrawModelScaled(model, x, y, z, rx, ry, rz, 256);
}

void X3D_EraseModelFaces(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz, X3D_i16 scale_q8)
{
    X3D_u8 i;
    X3D_u8 count;
    X3D_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->faces == 0) return;
    if ((model->flags & X3D_MODEL_HIDDEN_LINES) == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > X3D_MODEL_VERTEX_LIMIT) count = X3D_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const X3D_Vec3* v;
        X3D_i16 vx;
        X3D_i16 vy;
        X3D_i16 vz;
        X3D_u8 sx;
        X3D_u8 sy;

        v = &model->vertices[(__safe_index X3D_u8)i];
        vx = (X3D_i16)((v->x * scale_q8) >> 8);
        vy = (X3D_i16)((v->y * scale_q8) >> 8);
        vz = (X3D_i16)((v->z * scale_q8) >> 8);

        X3D_rotate_y(&vx, &vz, ry);
        X3D_rotate_x(&vy, &vz, rx);
        X3D_rotate_z(&vx, &vy, rz);

        vx = (X3D_i16)(vx + x);
        vy = (X3D_i16)(vy + y);
        vz = (X3D_i16)(vz + z);

        if (X3D_project_world(vx, vy, vz, &sx, &sy))
        {
            X3D_screen_x[(__safe_index X3D_u8)i] = sx;
            X3D_screen_y[(__safe_index X3D_u8)i] = sy;
            X3D_screen_visible[(__safe_index X3D_u8)i] = 1;
        }
        else
        {
            X3D_screen_visible[(__safe_index X3D_u8)i] = 0;
        }
        i = (X3D_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > X3D_MODEL_FACE_LIMIT) face_count = X3D_MODEL_FACE_LIMIT;
    X3D_build_face_visibility(model, count, face_count);

    i = 0;
    while (i < face_count)
    {
        const X3D_Face* f;
        f = &model->faces[(__safe_index X3D_u8)i];
        if (X3D_face_visible[(__safe_index X3D_u8)i] &&
            X3D_screen_visible[(__safe_index X3D_u8)f->a] &&
            X3D_screen_visible[(__safe_index X3D_u8)f->b] &&
            X3D_screen_visible[(__safe_index X3D_u8)f->c])
        {
            X3D_clear_triangle(
                X3D_screen_x[(__safe_index X3D_u8)f->a],
                X3D_screen_y[(__safe_index X3D_u8)f->a],
                X3D_screen_x[(__safe_index X3D_u8)f->b],
                X3D_screen_y[(__safe_index X3D_u8)f->b],
                X3D_screen_x[(__safe_index X3D_u8)f->c],
                X3D_screen_y[(__safe_index X3D_u8)f->c]);
        }
        i = (X3D_u8)(i + 1);
    }
}

void X3D_EraseTriangle2D(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by, X3D_u8 cx, X3D_u8 cy)
{
    X3D_clear_triangle(ax, ay, bx, by, cx, cy);
}

void X3D_EraseSpan2D(X3D_u8 y, X3D_u8 x0, X3D_u8 x1)
{
    X3D_u8 tx;

    if (x0 > x1)
    {
        tx = x0;
        x0 = x1;
        x1 = tx;
    }
    X3D_stage_clear_span(y, x0, x1);
}

void X3D_DrawModelScaled(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz, X3D_i16 scale_q8)
{
    X3D_u8 i;
    X3D_u8 count;
    X3D_u8 edge_count;
    X3D_u8 face_count;

    if (model == 0) return;
    if (model->vertices == 0) return;
    if (model->edges == 0) return;
    if (scale_q8 <= 0) scale_q8 = 256;

    count = model->vertex_count;
    if (count > X3D_MODEL_VERTEX_LIMIT) count = X3D_MODEL_VERTEX_LIMIT;

    i = 0;
    while (i < count)
    {
        const X3D_Vec3* v;
        X3D_i16 vx;
        X3D_i16 vy;
        X3D_i16 vz;
        X3D_u8 sx;
        X3D_u8 sy;

        v = &model->vertices[(__safe_index X3D_u8)i];
        vx = (X3D_i16)((v->x * scale_q8) >> 8);
        vy = (X3D_i16)((v->y * scale_q8) >> 8);
        vz = (X3D_i16)((v->z * scale_q8) >> 8);

        X3D_rotate_y(&vx, &vz, ry);
        X3D_rotate_x(&vy, &vz, rx);
        X3D_rotate_z(&vx, &vy, rz);

        vx = (X3D_i16)(vx + x);
        vy = (X3D_i16)(vy + y);
        vz = (X3D_i16)(vz + z);

        if (X3D_project_world(vx, vy, vz, &sx, &sy))
        {
            X3D_screen_x[(__safe_index X3D_u8)i] = sx;
            X3D_screen_y[(__safe_index X3D_u8)i] = sy;
            X3D_screen_visible[(__safe_index X3D_u8)i] = 1;
        }
        else
        {
            X3D_screen_visible[(__safe_index X3D_u8)i] = 0;
        }
        i = (X3D_u8)(i + 1);
    }

    face_count = model->face_count;
    if (face_count > X3D_MODEL_FACE_LIMIT) face_count = X3D_MODEL_FACE_LIMIT;
    if ((model->flags & X3D_MODEL_HIDDEN_LINES) && (model->faces != 0) && (model->edge_faces != 0))
    {
        X3D_build_face_visibility(model, count, face_count);
    }

    edge_count = model->edge_count;
    i = 0;
    while (i < edge_count)
    {
        const X3D_Edge* e;
        e = &model->edges[(__safe_index X3D_u8)i];
        if ((e->a < count) && (e->b < count))
        {
            if (X3D_screen_visible[(__safe_index X3D_u8)e->a] &&
                X3D_screen_visible[(__safe_index X3D_u8)e->b] &&
                X3D_is_edge_visible(model, i, face_count))
            {
                X3D_DrawLine2D(
                    X3D_screen_x[(__safe_index X3D_u8)e->a],
                    X3D_screen_y[(__safe_index X3D_u8)e->a],
                    X3D_screen_x[(__safe_index X3D_u8)e->b],
                    X3D_screen_y[(__safe_index X3D_u8)e->b]);
            }
        }
        i = (X3D_u8)(i + 1);
    }
}

void X3D_DrawIndexedEdges(const X3D_Vec3* vertices, X3D_u8 vertex_count, const X3D_u8* edges, X3D_u8 edge_count, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 scale)
{
    X3D_u8 i;
    X3D_u8 count;

    if (vertices == 0) return;
    if (edges == 0) return;

    count = vertex_count;
    if (count > X3D_MODEL_VERTEX_LIMIT) count = X3D_MODEL_VERTEX_LIMIT;
    if (scale == 0) scale = 1;

    i = 0;
    while (i < count)
    {
        const X3D_Vec3* v;
        X3D_i16 vx;
        X3D_i16 vy;
        X3D_i16 vz;
        X3D_u8 sx;
        X3D_u8 sy;

        v = &vertices[(__safe_index X3D_u8)i];
        vx = (X3D_i16)(x + (X3D_i16)(v->x * scale));
        vy = (X3D_i16)(y + (X3D_i16)(v->y * scale));
        vz = (X3D_i16)(z + (X3D_i16)(v->z * scale));

        if (X3D_project_world(vx, vy, vz, &sx, &sy))
        {
            X3D_screen_x[(__safe_index X3D_u8)i] = sx;
            X3D_screen_y[(__safe_index X3D_u8)i] = sy;
            X3D_screen_visible[(__safe_index X3D_u8)i] = 1;
        }
        else
        {
            X3D_screen_visible[(__safe_index X3D_u8)i] = 0;
        }

        i = (X3D_u8)(i + 1);
    }

    i = 0;
    while (i < edge_count)
    {
        X3D_u8 a;
        X3D_u8 b;

        a = edges[(__safe_index X3D_u8)((X3D_u8)(i << 1))];
        b = edges[(__safe_index X3D_u8)((X3D_u8)((i << 1) + 1))];
        if ((a < count) && (b < count) &&
            X3D_screen_visible[(__safe_index X3D_u8)a] &&
            X3D_screen_visible[(__safe_index X3D_u8)b])
        {
            X3D_DrawLine2D(
                X3D_screen_x[(__safe_index X3D_u8)a],
                X3D_screen_y[(__safe_index X3D_u8)a],
                X3D_screen_x[(__safe_index X3D_u8)b],
                X3D_screen_y[(__safe_index X3D_u8)b]);
        }

        i = (X3D_u8)(i + 1);
    }
}

void X3D_SetAuxTransfer(X3D_u8 flag)
{
    X3D_aux_transfer_enabled = flag;
}

static void X3D_transfer_dirty_stage()
{
    X3D_u8 tx;
    X3D_u8 ty;
    X3D_u8 idx;
    X3D_u16 dst;

    idx = 0;
    tx = 0;
    while (tx < X3D_TILE_W)
    {
        ty = 0;
        while (ty < X3D_TILE_H)
        {
            X3D_u8 cur;
            X3D_u8 prev;

            cur = X3D_dirty_tiles[(__safe_index X3D_u8)idx];
            prev = X3D_prev_dirty_tiles[(__safe_index X3D_u8)idx];
            if ((cur != 0) || (prev != 0))
            {
                X3D_dirty_src_hi = (X3D_u8)(0xD0 + tx);
                X3D_dirty_src_lo = (X3D_u8)(ty << 3);
                dst = (X3D_u16)(0x8901 + ((X3D_u16)idx << 4));
                X3D_dirty_dst_hi = (X3D_u8)(dst >> 8);
                X3D_dirty_dst_lo = (X3D_u8)(dst & 0xFF);
                X3D_transfer_dirty_tile_asm();
                X3D_prev_dirty_tiles[(__safe_index X3D_u8)idx] = cur;
                X3D_dirty_tiles[(__safe_index X3D_u8)idx] = 0;
            }

            idx = (X3D_u8)(idx + 1);
            ty = (X3D_u8)(ty + 1);
        }
        tx = (X3D_u8)(tx + 1);
    }
}

void X3D_SetDirtyTransfer(X3D_u8 flag)
{
    X3D_dirty_transfer_enabled = flag;
    X3D_clear_dirty_tables();
    X3D_clear_stage_asm();
}

void X3D_TransferMainNow()
{
    X3D_transfer_stage_asm();
}

void X3D_TransferAuxNow()
{
    X3D_transfer_stage_aux_asm();
}

void X3D_EndFrame()
{
    X3D_wait_vblank_start();
    X3D_flush_bg_queue();
    if (X3D_aux_transfer_enabled)
    {
        X3D_transfer_stage_aux_asm();
    }
    if (X3D_dirty_transfer_enabled)
    {
        X3D_transfer_dirty_stage();
    }
    else
    {
        X3D_transfer_stage_asm();
    }
}


