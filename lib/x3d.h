#pragma once

#ifndef KITAQGB_X3D_H
#define KITAQGB_X3D_H

#define X3D_SCREEN_W 128
#define X3D_SCREEN_H 120
#define X3D_MODEL_VERTEX_LIMIT 24
#define X3D_MODEL_FACE_LIMIT 16
#define X3D_SCENE_OBJECT_LIMIT 8
#define X3D_FACE_NONE 255
#define X3D_MODEL_HIDDEN_LINES 1
#define X3D_ANGLE_STEPS 16

typedef s8 X3D_i8;
typedef s16 X3D_i16;
typedef u8 X3D_u8;
typedef u16 X3D_u16;

typedef struct {
    X3D_i16 x;
    X3D_i16 y;
    X3D_i16 z;
} X3D_Vec3;

typedef struct {
    X3D_u8 a;
    X3D_u8 b;
} X3D_Edge;

typedef struct {
    X3D_u8 a;
    X3D_u8 b;
    X3D_u8 c;
} X3D_Face;

typedef struct {
    X3D_u8 f0;
    X3D_u8 f1;
} X3D_EdgeFaces;

typedef struct {
    const X3D_Vec3* vertices;
    const X3D_Edge* edges;
    const X3D_Face* faces;
    const X3D_EdgeFaces* edge_faces;
    X3D_u8 vertex_count;
    X3D_u8 edge_count;
    X3D_u8 face_count;
    X3D_u8 flags;
} X3D_Model;

typedef struct {
    const X3D_Model* model;
    X3D_i16 x;
    X3D_i16 y;
    X3D_i16 z;
    X3D_u8 rx;
    X3D_u8 ry;
    X3D_u8 rz;
    X3D_i16 scale_q8;
    X3D_u8 visible;
} X3D_Object;

void X3D_Init();
void X3D_BeginFrame();
void X3D_SetCamera(X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 pitch, X3D_u8 yaw, X3D_u8 roll);
void X3D_DrawModel(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz);
void X3D_DrawModelScaled(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz, X3D_i16 scale_q8);
void X3D_EraseModelFaces(const X3D_Model* model, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz, X3D_i16 scale_q8);
void X3D_EraseTriangle2D(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by, X3D_u8 cx, X3D_u8 cy);
void X3D_EraseSpan2D(X3D_u8 y, X3D_u8 x0, X3D_u8 x1);
void X3D_DrawScene(X3D_Object* objects, X3D_u8 count);
X3D_u8 X3D_ProjectPoint(X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8* sx, X3D_u8* sy);
void X3D_RotatePoint(X3D_i16* x, X3D_i16* y, X3D_i16* z, X3D_u8 rx, X3D_u8 ry, X3D_u8 rz);
void X3D_DrawLine3D(X3D_i16 ax, X3D_i16 ay, X3D_i16 az, X3D_i16 bx, X3D_i16 by, X3D_i16 bz);
void X3D_DrawLine2D(X3D_u8 ax, X3D_u8 ay, X3D_u8 bx, X3D_u8 by);
void X3D_DrawIndexedEdges(const X3D_Vec3* vertices, X3D_u8 vertex_count, const X3D_u8* edges, X3D_u8 edge_count, X3D_i16 x, X3D_i16 y, X3D_i16 z, X3D_u8 scale);
void X3D_PutBgTile(X3D_u8 x, X3D_u8 y, X3D_u8 tile);
void X3D_SetPalette(X3D_u8 bgp);
void X3D_SetAuxTransfer(X3D_u8 flag);
void X3D_SetDirtyTransfer(X3D_u8 flag);
void X3D_TransferMainNow();
void X3D_TransferAuxNow();
void X3D_EndFrame();

#endif

