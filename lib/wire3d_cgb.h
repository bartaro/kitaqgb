#pragma once

#ifndef KITAQGB_WIRE3D_CGB_H
#define KITAQGB_WIRE3D_CGB_H

#define WIRE3DCGB_SCREEN_W 128
#define WIRE3DCGB_SCREEN_H 96
#define WIRE3DCGB_FULL_SCREEN_W 160
#define WIRE3DCGB_FULL_SCREEN_H 144
#define WIRE3DCGB_FULL_TILE_LIMIT 127
#define WIRE3DCGB_STAGE_BASE 0xD300
#define WIRE3DCGB_STAGE_WRAM_BANK 2
#define WIRE3DCGB_STAGE_BYTES 3072
#define WIRE3DCGB_MODEL_VERTEX_LIMIT 24
#define WIRE3DCGB_MODEL_FACE_LIMIT 16
#define WIRE3DCGB_MODEL_EDGE_LIMIT 64
#define WIRE3DCGB_SCENE_OBJECT_LIMIT 8
#define WIRE3DCGB_FACE_NONE 255
#define WIRE3DCGB_MODEL_HIDDEN_LINES 1
#define WIRE3DCGB_COLOR_LOW 1
#define WIRE3DCGB_COLOR_MAIN 2
#define WIRE3DCGB_COLOR_HIGH 3
#define WIRE3DCGB_COLOR_DEFAULT WIRE3DCGB_COLOR_MAIN

#define WIRE3DCGB_RGB15(r5, g5, b5) ((w3dcgb_u16)((w3dcgb_u16)((r5) & 31) | ((w3dcgb_u16)((g5) & 31) << 5) | ((w3dcgb_u16)((b5) & 31) << 10)))

typedef s8 w3dcgb_i8;
typedef s16 w3dcgb_i16;
typedef u8 w3dcgb_u8;
typedef u16 w3dcgb_u16;

typedef struct {
    w3dcgb_i16 x;
    w3dcgb_i16 y;
    w3dcgb_i16 z;
} Wire3DCGB_Vec3;

typedef struct {
    w3dcgb_u8 a;
    w3dcgb_u8 b;
} Wire3DCGB_Edge;

typedef struct {
    w3dcgb_u8 a;
    w3dcgb_u8 b;
    w3dcgb_u8 c;
} Wire3DCGB_Face;

typedef struct {
    w3dcgb_u8 f0;
    w3dcgb_u8 f1;
} Wire3DCGB_EdgeFaces;

typedef struct {
    const Wire3DCGB_Vec3* vertices;
    const Wire3DCGB_Edge* edges;
    const Wire3DCGB_Face* faces;
    const Wire3DCGB_EdgeFaces* edge_faces;
    w3dcgb_u8 vertex_count;
    w3dcgb_u8 edge_count;
    w3dcgb_u8 face_count;
    w3dcgb_u8 flags;
} Wire3DCGB_Model;

typedef struct {
    const Wire3DCGB_Model* model;
    w3dcgb_i16 x;
    w3dcgb_i16 y;
    w3dcgb_i16 z;
    w3dcgb_i8 rx;
    w3dcgb_i8 ry;
    w3dcgb_i8 rz;
    w3dcgb_i16 scale_q8;
    w3dcgb_u8 visible;
    w3dcgb_u8 color;
} Wire3DCGB_Object;

typedef struct {
    w3dcgb_i16 x;
    w3dcgb_i16 y;
    w3dcgb_i16 z;
    w3dcgb_i8 rx;
    w3dcgb_i8 ry;
    w3dcgb_i8 rz;
    w3dcgb_u8 color;
} Wire3DCGB_FastCube;

void Wire3DCGB_Init();
void Wire3DCGB_InitFullScreen();
void Wire3DCGB_InitFastMapLite();
void Wire3DCGB_EnableDoubleSpeed();
void Wire3DCGB_SetScreenOffset(w3dcgb_u8 scx, w3dcgb_u8 scy);
void Wire3DCGB_SetPaletteRGB15(w3dcgb_u16 color0, w3dcgb_u16 color1, w3dcgb_u16 color2, w3dcgb_u16 color3);
void Wire3DCGB_SetLineColor(w3dcgb_u8 color);
w3dcgb_u8 Wire3DCGB_GetLineColor();
void Wire3DCGB_ClearFrameTiles();
void Wire3DCGB_BeginFrame();
void Wire3DCGB_BeginFrameFast();
void Wire3DCGB_BeginFrameSparse();
void Wire3DCGB_ClearSparseStageFast();
void Wire3DCGB_MarkDirtyRect2D(w3dcgb_u8 min_x, w3dcgb_u8 min_y, w3dcgb_u8 max_x, w3dcgb_u8 max_y);
void Wire3DCGB_SetCamera(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 pitch, w3dcgb_i8 yaw, w3dcgb_i8 roll);
void Wire3DCGB_DrawModel(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz);
void Wire3DCGB_DrawModelScaled(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8);
void Wire3DCGB_DrawModelColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_u8 color);
void Wire3DCGB_DrawModelScaledColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8, w3dcgb_u8 color);
void Wire3DCGB_EraseModelFaces(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8);
void Wire3DCGB_EraseTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy);
void Wire3DCGB_EraseSpan2D(w3dcgb_u8 y, w3dcgb_u8 x0, w3dcgb_u8 x1);
/* Signed endpoints, clipped before conversion to the rasterizer's u8 ABI. */
void Wire3DCGB_DrawLineClipped2D(w3dcgb_i16 x0, w3dcgb_i16 y0,
                               w3dcgb_i16 x1, w3dcgb_i16 y1, w3dcgb_u8 color);
void Wire3DCGB_DrawEdgeListClipped2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
    const w3dcgb_i16* vx, const w3dcgb_i16* vy, w3dcgb_u8 vertex_count,
    w3dcgb_i16 cx, w3dcgb_i16 cy, w3dcgb_u8 color);
w3dcgb_i16 Wire3DCGB_ProjectAxis48(w3dcgb_i16 value, w3dcgb_i16 z);
void Wire3DCGB_InvalidateFrameHistory();
/* Startup-only: reserves both BG maps; queued HUD tiles are mirrored. */
void Wire3DCGB_EnableAtomicMaps();
void Wire3DCGB_EraseRect2D(w3dcgb_u8 x0, w3dcgb_u8 y0, w3dcgb_u8 x1, w3dcgb_u8 y1);
void Wire3DCGB_EraseLeftGuard24Fast();
void Wire3DCGB_DrawWhiteBorderFast();
void Wire3DCGB_ClearOcclusionMask();
void Wire3DCGB_SetOcclusionActive(w3dcgb_u8 active);
void Wire3DCGB_MarkTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy);
void Wire3DCGB_MarkPackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                     w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y);
void Wire3DCGB_ErasePackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                      w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y);
void Wire3DCGB_DrawScene(Wire3DCGB_Object* objects, w3dcgb_u8 count);
void Wire3DCGB_DrawFastCubes(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
void Wire3DCGB_DrawFastStatus(w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
void Wire3DCGB_DrawFastCubeFrame(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
void Wire3DCGB_DrawFastProjectile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_i16 z, w3dcgb_u8 phase, w3dcgb_u8 color);
void Wire3DCGB_FastMapBegin();
void Wire3DCGB_FastMapBeginTilesOnly();
void Wire3DCGB_FastMapCell(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 color, w3dcgb_u8 mask);
void Wire3DCGB_FastMapRect(w3dcgb_u8 tx0, w3dcgb_u8 ty0, w3dcgb_u8 tx1, w3dcgb_u8 ty1, w3dcgb_u8 color, w3dcgb_u8 sides);
void Wire3DCGB_FastMapFlush();
void Wire3DCGB_FastMapFlushTilesOnly();
w3dcgb_u8 Wire3DCGB_ProjectPoint(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_u8* sx, w3dcgb_u8* sy);
/* Fast path for games whose camera pitch, yaw, and roll stay at zero. */
w3dcgb_u8 Wire3DCGB_ProjectPointNoRotation(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_u8* sx, w3dcgb_u8* sy);
void Wire3DCGB_DrawLine3D(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz);
void Wire3DCGB_DrawLine3DColor(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz, w3dcgb_u8 color);
void Wire3DCGB_DrawPoint2D(w3dcgb_u8 x, w3dcgb_u8 y);
void Wire3DCGB_DrawTinyModel2D(w3dcgb_u8 x, w3dcgb_u8 y,
                               w3dcgb_u8 turn, w3dcgb_u8 color);
void Wire3DCGB_DrawLine2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by);
void Wire3DCGB_DrawLine2DColor(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 color);
void Wire3DCGB_DrawMaskedModel2D(const Wire3DCGB_Model* model,
                                 const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                                 const w3dcgb_u8* edge_mask,
                                 w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color);
void Wire3DCGB_DrawEdgeList2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
                              const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                              w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color);
void Wire3DCGB_DrawLineList2DColor(const w3dcgb_u8* line_xy,
                                  w3dcgb_u8 line_count, w3dcgb_u8 color);
void Wire3DCGB_PutBgTile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 tile);
void Wire3DCGB_EndFrame();
void Wire3DCGB_EndFrameFast();
void Wire3DCGB_EndFrameSparse();
void Wire3DCGB_EndFrameSparseNow();
w3dcgb_u8 Wire3DCGB_GetFullScreenTileCount();
w3dcgb_u8 Wire3DCGB_GetFullScreenOverflow();

#endif
