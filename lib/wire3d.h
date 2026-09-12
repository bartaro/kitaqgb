#pragma once

#ifndef KITAQGB_WIRE3D_H
#define KITAQGB_WIRE3D_H

#define WIRE3D_SCREEN_W 128
#define WIRE3D_SCREEN_H 96
#define WIRE3D_MODEL_VERTEX_LIMIT 24
#define WIRE3D_MODEL_EDGE_LIMIT 16
#define WIRE3D_MODEL_FACE_LIMIT 16
#define WIRE3D_SCENE_OBJECT_LIMIT 8
#define WIRE3D_FACE_NONE 255
#define WIRE3D_MODEL_HIDDEN_LINES 1
#define WIRE3D_EDGE_MASK_BINS_16 16

typedef s8 w3d_i8;
typedef s16 w3d_i16;
typedef u8 w3d_u8;
typedef u16 w3d_u16;

typedef struct {
    w3d_i16 x;
    w3d_i16 y;
    w3d_i16 z;
} Wire3D_Vec3;

typedef struct {
    w3d_u8 a;
    w3d_u8 b;
} Wire3D_Edge;

typedef struct {
    w3d_u8 a;
    w3d_u8 b;
    w3d_u8 c;
} Wire3D_Face;

typedef struct {
    w3d_u8 f0;
    w3d_u8 f1;
} Wire3D_EdgeFaces;

typedef struct {
    const Wire3D_Vec3* vertices;
    const Wire3D_Edge* edges;
    const Wire3D_Face* faces;
    const Wire3D_EdgeFaces* edge_faces;
    /* Keep edge mask tables in RAM for pointer-indirect access on current KITAQGB. */
    const w3d_u16* edge_masks;
    w3d_u8 vertex_count;
    w3d_u8 edge_count;
    w3d_u8 face_count;
    w3d_u8 edge_mask_count;
    w3d_u8 flags;
} Wire3D_Model;

typedef struct {
    const Wire3D_Model* model;
    w3d_i16 x;
    w3d_i16 y;
    w3d_i16 z;
    w3d_i8 rx;
    w3d_i8 ry;
    w3d_i8 rz;
    w3d_i16 scale_q8;
    w3d_u8 visible;
} Wire3D_Object;

void Wire3D_Init();
void Wire3D_BeginFrame();
void Wire3D_SetCamera(w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 pitch, w3d_i8 yaw, w3d_i8 roll);
void Wire3D_DrawModel(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz);
void Wire3D_DrawModelScaled(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz, w3d_i16 scale_q8);
void Wire3D_EraseModelFaces(const Wire3D_Model* model, w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz, w3d_i16 scale_q8);
void Wire3D_EraseTriangle2D(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by, w3d_u8 cx, w3d_u8 cy);
void Wire3D_EraseSpan2D(w3d_u8 y, w3d_u8 x0, w3d_u8 x1);
void Wire3D_DrawScene(Wire3D_Object* objects, w3d_u8 count);
w3d_u16 Wire3D_SelectEdgeMask(const Wire3D_Model* model, w3d_i8 rx, w3d_i8 ry, w3d_i8 rz);
w3d_u8 Wire3D_ProjectPoint(w3d_i16 x, w3d_i16 y, w3d_i16 z, w3d_u8* sx, w3d_u8* sy);
void Wire3D_DrawLine3D(w3d_i16 ax, w3d_i16 ay, w3d_i16 az, w3d_i16 bx, w3d_i16 by, w3d_i16 bz);
void Wire3D_DrawLine2D(w3d_u8 ax, w3d_u8 ay, w3d_u8 bx, w3d_u8 by);
void Wire3D_PutBgTile(w3d_u8 x, w3d_u8 y, w3d_u8 tile);
void Wire3D_SetPalette(w3d_u8 bgp);
void Wire3D_EndFrame();

#endif
