#pragma once

// Select the viewport at build time before including this header or source.
// Default: 128x120 with consuming uploads and optional dirty/auxiliary transfer.
// Height 96 retains the HUD and yaw-mask profile with BeginFrame clearing.
#ifndef WIRE3D_DMG_HEIGHT
#define WIRE3D_DMG_HEIGHT 120
#endif
#if WIRE3D_DMG_HEIGHT != 96 && WIRE3D_DMG_HEIGHT != 120
#error WIRE3D_DMG_HEIGHT must be 96 or 120
#endif


#ifndef KITAQGB_WIRE3D_DMG_H
#define KITAQGB_WIRE3D_DMG_H

#if WIRE3D_DMG_HEIGHT == 96
// Monochrome 128x96 wireframe renderer with global, non-reentrant state.
// It takes over LCD tile data/map and reserves WRAM 0xD000..0xDFFF for staging.
// Angles wrap every 16 steps; scale 256 is unity. Scene occlusion is a coarse
// face-bounding-box/five-line-sample heuristic, not per-pixel depth testing.
#else
// Monochrome 128x120 staged renderer with global non-reentrant state.
// Reserve WRAM 0xD000..0xDFFF and the renderer's tile/map VRAM regions.
// BeginFrame does not clear pixels: uploads consume the stage. Auxiliary source
// strips alias the main stage, and dirty transfer relies on current/previous flags.
#endif
#define WIRE3D_DMG_SCREEN_W 128
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_SCREEN_H 96
#else
#define WIRE3D_DMG_SCREEN_H 120
#endif
#define WIRE3D_DMG_MODEL_VERTEX_LIMIT 24
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_MODEL_EDGE_LIMIT 16
#endif
#define WIRE3D_DMG_MODEL_FACE_LIMIT 16
#define WIRE3D_DMG_SCENE_OBJECT_LIMIT 8
#define WIRE3D_DMG_FACE_NONE 255
#define WIRE3D_DMG_MODEL_HIDDEN_LINES 1
#if WIRE3D_DMG_HEIGHT == 96
#define WIRE3D_DMG_EDGE_MASK_BINS_16 16
#else
#define WIRE3D_DMG_ANGLE_STEPS 16
#endif

typedef s8 w3ddmg_i8;
typedef s16 w3ddmg_i16;
typedef u8 w3ddmg_u8;
typedef u16 w3ddmg_u16;

typedef struct {
    w3ddmg_i16 x;
    w3ddmg_i16 y;
    w3ddmg_i16 z;
} Wire3DDMG_Vec3;

typedef struct {
    w3ddmg_u8 a;
    w3ddmg_u8 b;
} Wire3DDMG_Edge;

typedef struct {
    w3ddmg_u8 a;
    w3ddmg_u8 b;
    w3ddmg_u8 c;
} Wire3DDMG_Face;

typedef struct {
    w3ddmg_u8 f0;
    w3ddmg_u8 f1;
} Wire3DDMG_EdgeFaces;

#if WIRE3D_DMG_HEIGHT == 96
// Model tables are borrowed for drawing, never copied or bank-switched.
// Counts are capped at 24 vertices, 16 edges and 16 faces; each supplied table
// must contain the referenced entries. Triangle winding with positive projected
// area is front-facing. FACE_NONE denotes a missing adjacent face.
#else
// Model tables are borrowed and must remain readable in the active bank.
// At most 24 vertices and 16 faces are processed; the model edge count is not
// limited to 16. Supply all declared edge and adjacency entries. Positive
// projected triangle area is front-facing; FACE_NONE means missing adjacency.
#endif
typedef struct {
    const Wire3DDMG_Vec3* vertices;
    const Wire3DDMG_Edge* edges;
    const Wire3DDMG_Face* faces;
    const Wire3DDMG_EdgeFaces* edge_faces;
#if WIRE3D_DMG_HEIGHT == 96
    /* Keep edge mask tables in RAM for pointer-indirect access on current KITAQGB. */
    const w3ddmg_u16* edge_masks;
#endif
    w3ddmg_u8 vertex_count;
    w3ddmg_u8 edge_count;
    w3ddmg_u8 face_count;
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_u8 edge_mask_count;
#endif
    w3ddmg_u8 flags;
} Wire3DDMG_Model;

typedef struct {
    const Wire3DDMG_Model* model;
    w3ddmg_i16 x;
    w3ddmg_i16 y;
    w3ddmg_i16 z;
#if WIRE3D_DMG_HEIGHT == 96
    w3ddmg_i8 rx;
    w3ddmg_i8 ry;
    w3ddmg_i8 rz;
#else
    w3ddmg_u8 rx;
    w3ddmg_u8 ry;
    w3ddmg_u8 rz;
#endif
    w3ddmg_i16 scale_q8;
    w3ddmg_u8 visible;
} Wire3DDMG_Object;

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
void Wire3DDMG_Init();
#if WIRE3D_DMG_HEIGHT == 96
// Clear staged pixels and occlusion mask and disable occlusion filtering.
// This leaves the displayed VRAM frame and pending BG tile queue unchanged.
#else
// Disable occlusion filtering only. Stage clearing normally happens during
// transfer, and Init supplies the first clean stage. This does not discard an
// unsubmitted frame, clear the mask, or reset pending BG writes/dirty flags.
#endif
void Wire3DDMG_BeginFrame();
// Store camera position and angles for later draws; a full turn is 16 angle
// steps. No existing drawing is reprojected and no input range validation occurs.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 pitch, w3ddmg_i8 yaw, w3ddmg_i8 roll);
#else
void Wire3DDMG_SetCamera(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 pitch, w3ddmg_u8 yaw, w3ddmg_u8 roll);
#endif
// Draw an unscaled model by delegating to DrawModelScaled with scale 256.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz);
// Scale, rotate in Y/X/Z order, translate and project at most 24 vertices;
// then draw up to 16 edges filtered by yaw masks and optional front-facing faces.
// Nonpositive scale becomes unity (256). Null model/vertex/edge pointers are
// ignored; referenced tables must remain readable. Keep scale products and
// translations in signed 16-bit range. Depth-crossing edges are dropped, not clipped.
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8);
#else
void Wire3DDMG_DrawModel(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz);
// Scale up to 24 vertices, rotate Y/X/Z, translate and project, then traverse
// all edge_count entries (a byte count) with optional adjacent-face filtering.
// Face scratch is capped at 16; edge tables must contain all declared entries.
// Nonpositive scale means 256. Null model/vertex/edge pointers are ignored.
// Keep products/translations in signed 16-bit range; depth-crossing edges are dropped.
void Wire3DDMG_DrawModelScaled(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8);
#endif
// Project up to 24 vertices and erase front-facing triangle interiors from
// the stage for a model with hidden-line flags and face data. Nonpositive scale
// means 256 (unity). Up to 16 faces are considered; depth-crossing faces are
// skipped. This also replaces shared projection caches, but does not update VRAM.
#if WIRE3D_DMG_HEIGHT == 96
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz, w3ddmg_i16 scale_q8);
#else
void Wire3DDMG_EraseModelFaces(const Wire3DDMG_Model* model, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz, w3ddmg_i16 scale_q8);
#endif
// Erase a triangle from staged pixels using integer scanline intersections.
// Supply viewport coordinates. Degenerate triangles are ignored; the occlusion
// mask and VRAM are unchanged.
void Wire3DDMG_EraseTriangle2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by, w3ddmg_u8 cx, w3ddmg_u8 cy);
// Order the X endpoints and erase the inclusive staged span. Both endpoints
// must be in 0..127; invalid Y is ignored. The routine does not clip X for you.
void Wire3DDMG_EraseSpan2D(w3ddmg_u8 y, w3ddmg_u8 x0, w3ddmg_u8 x1);
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
void Wire3DDMG_DrawScene(Wire3DDMG_Object* objects, w3ddmg_u8 count);
#if WIRE3D_DMG_HEIGHT == 96
// Select a precomputed mask from the low four bits of object yaw; pitch,
// roll and camera orientation are ignored. Use 16, 8, 4 or 2 bins; other positive
// counts select entry zero (counts >=16 use the first 16). Missing data returns
// 0xFFFF. Keep the mask table readable through the current data-pointer mapping.
w3ddmg_u16 Wire3DDMG_SelectEdgeMask(const Wire3DDMG_Model* model, w3ddmg_i8 rx, w3ddmg_i8 ry, w3ddmg_i8 rz);
// Project a world point with the current camera. Return zero for rejected
// depth, leaving outputs unchanged; success writes clamped 128x96 coordinates.
// Output pointers must be valid. Screen clamping is not geometric clipping.
#else
// Project a world point with the current camera. Return zero for invalid
// depth without changing outputs; success writes clamped 128x120 coordinates.
// Both output pointers must be valid. Coordinate differences must fit s16.
#endif
w3ddmg_u8 Wire3DDMG_ProjectPoint(w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8* sx, w3ddmg_u8* sy);
#if WIRE3D_DMG_HEIGHT == 120
// Rotate in place in Y/X/Z order using 16 steps per full turn. A null
// component pointer makes the whole call a no-op; use distinct pointers.
// Each axis rotation clamps its inputs to +/-220.
void Wire3DDMG_RotatePoint(w3ddmg_i16* x, w3ddmg_i16* y, w3ddmg_i16* z, w3ddmg_u8 rx, w3ddmg_u8 ry, w3ddmg_u8 rz);
#endif
// Project both world endpoints and draw only when both pass the depth range.
// Do not intersect a crossing segment with near/far planes; projected endpoints
// are independently clamped to the screen edges.
void Wire3DDMG_DrawLine3D(w3ddmg_i16 ax, w3ddmg_i16 ay, w3ddmg_i16 az, w3ddmg_i16 bx, w3ddmg_i16 by, w3ddmg_i16 bz);
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
void Wire3DDMG_DrawLine2D(w3ddmg_u8 ax, w3ddmg_u8 ay, w3ddmg_u8 bx, w3ddmg_u8 by);
#if WIRE3D_DMG_HEIGHT == 120
// Draw byte-pair edge indices after integer scaling and translation; scale
// zero means one, and there is no object rotation or face filtering. Project at
// most 24 vertices. Supply at most 128 edge pairs because byte offsets wrap
// after 255. Invalid vertex IDs/depth-rejected endpoints are skipped; null arrays
// are ignored. Keep coordinate products/sums representable as s16.
void Wire3DDMG_DrawIndexedEdges(const Wire3DDMG_Vec3* vertices, w3ddmg_u8 vertex_count, const w3ddmg_u8* edges, w3ddmg_u8 edge_count, w3ddmg_i16 x, w3ddmg_i16 y, w3ddmg_i16 z, w3ddmg_u8 scale);
#endif
// Queue a tile write to the 32x32 map at 0x9800 for EndFrame. Invalid map
// coordinates and submissions beyond 48 pending writes are silently ignored.
// This queue is separate from the general vram library queue.
void Wire3DDMG_PutBgTile(w3ddmg_u8 x, w3ddmg_u8 y, w3ddmg_u8 tile);
// Write the raw DMG BGP palette encoding immediately; no frame synchronization occurs.
void Wire3DDMG_SetPalette(w3ddmg_u8 bgp);
#if WIRE3D_DMG_HEIGHT == 96
// Wait for the next VBlank, flush pending BG map writes, then copy the stage
// to VRAM while polling access windows. Copying consumes the staged pixels.
// The transfer may extend beyond VBlank; this is not an atomic frame swap.
#else
// Store the EndFrame auxiliary-transfer gate; any nonzero value enables it.
// No data is copied or cleared here. Auxiliary source bytes alias the main stage.
void Wire3DDMG_SetAuxTransfer(w3ddmg_u8 flag);
// Store the dirty-transfer gate and clear both dirty histories and the main
// stage, even if the value is unchanged. VRAM is retained; enabling this over an
// old image can leave unmarked old tiles, so arrange a clean displayed baseline.
void Wire3DDMG_SetDirtyTransfer(w3ddmg_u8 flag);
// Upload and consume the full main stage immediately, polling STAT but not
// waiting for the next VBlank. Ignore transfer gates and leave dirty histories
// and BG tile queue unchanged; the caller coordinates the frame lifecycle.
void Wire3DDMG_TransferMainNow();
// Upload and consume auxiliary source strips immediately, independent of the
// auxiliary gate. No VBlank-start wait or dirty-history/BG-queue update occurs.
// The source overlaps the main stage and must be coordinated with its upload.
void Wire3DDMG_TransferAuxNow();
// Wait for the next VBlank and flush BG writes, then optionally consume the
// auxiliary strips before uploading either dirty tiles or the entire main stage.
// Auxiliary/main source overlap makes that ordering significant. STAT polling
// continues across access windows; the update need not fit one VBlank.
#endif
void Wire3DDMG_EndFrame();

#endif
#if WIRE3D_DMG_HEIGHT == 120

#endif
