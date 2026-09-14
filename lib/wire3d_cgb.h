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

// Pack the low five bits of each channel into the CGB RGB15 format.
// Each argument is evaluated once; out-of-range components wrap to five bits.
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

// Initialize the CGB-only 128x96 renderer: enable double speed, turn the
// LCD off at VBlank, clear both tile banks, load HUD/generated tiles, and
// install a 16x12 column-major viewport. Start with blank tile bank 1, scroll
// zero, default BG palettes, color 2 and a zero camera. Reset queues and
// clear the bank-2 stage. Call BeginFrame before drawing to initialize
// occlusion state. Initialization replaces LCD/tile/map state globally.
void Wire3DCGB_Init();
// Initialize the CGB-only 160x144 tile allocator. Clear both tile banks
// and map 9800, set scroll/palettes/color/camera, reset tile counts and
// lookup/span state, then enable unsigned BG tile addressing. HUD tiles
// are not loaded. Reserve its fixed WRAM lookup, payload and address lists
// and keep the same WRAM bank mapped through full-screen rendering.
void Wire3DCGB_InitFullScreen();
// Initialize a CGB tile-map renderer with HUD and 48 precomputed outline
// tiles. Clear tile VRAM/maps, reset scroll/palettes/camera and select normal
// mode. This omits normal pixel-viewport map layout and generated cube stamps;
// use the FastMap APIs for this mode. Clear the bank-2 stage and mark fast
// map preparation pending. Full-runtime build required.
void Wire3DCGB_InitFastMapLite();
// On CGB, return if KEY1 already reports double speed; otherwise request
// the speed switch and call the STOP helper. This API assumes CGB hardware
// and does not provide a monochrome fallback.
void Wire3DCGB_EnableDoubleSpeed();
// Write SCX/SCY immediately, without waiting for VBlank or changing projected
// coordinates. The caller chooses when a visible scroll change is acceptable.
void Wire3DCGB_SetScreenOffset(w3dcgb_u8 scx, w3dcgb_u8 scy);
// Set BG palette zero to the four supplied RGB15 colors. Palettes 1..3
// share color0 and repeat their respective nonzero color in entries 1..3,
// allowing monochrome stamp tiles to be recolored by attributes. Supply a
// valid palette-access window, normally LCD-off setup or VBlank.
void Wire3DCGB_SetPaletteRGB15(w3dcgb_u16 color0, w3dcgb_u16 color1, w3dcgb_u16 color2, w3dcgb_u16 color3);
// Store color & 3. Use the named line colors 1..3: normal lines OR their
// bitplanes, so intersecting colors combine; full-screen fast lines replace
// the pixel color. Line color zero is treated as 3, not as an eraser.
// Normal-mode point drawing separately supports color zero for erasure.
void Wire3DCGB_SetLineColor(w3dcgb_u8 color);
// Return the currently stored two-bit line color without changing drawing state.
w3dcgb_u8 Wire3DCGB_GetLineColor();
// For the 128x96 renderer, wait for VBlank, disable the LCD, clear the stage
// and frame tile ranges in both banks, reset map attributes, then enable LCDC
// 0x81. Clear dirty flags/counts but not sparse min/max history. This does
// not reset full_mode and is not a full-screen reinitializer.
void Wire3DCGB_ClearFrameTiles();
// In normal mode, select full-stage transfer, clear all staged pixels and
// the bitmap mask, reset mask bounds, and disable occlusion. In full-screen
// mode, reset that mode's allocator/spans instead. Pending BG writes remain queued.
// Full-runtime build required.
void Wire3DCGB_BeginFrame();
// Perform the same normal/full-screen frame reset as BeginFrame, while
// first marking both previous sparse-history ranges as the entire 192-tile
// viewport. This prevents a subsequent sparse upload from trusting stale banks.
void Wire3DCGB_BeginFrameFast();
// Select sparse transfer and clear the entire 128x96 stage, avoiding stale
// pixels when the dirty range expands. Reset mask bounds and disable
// occlusion; PAINTER_ONLY builds skip clearing the unused bitmap mask.
// Full-screen mode delegates to its own allocator reset.
void Wire3DCGB_BeginFrameSparse();
// Clear the entire bank-2 stage and mark the full 128x96 viewport dirty.
// Despite its name this is not a partial clear and does not preserve staged
// pixels. It neither clears occlusion nor selects a transfer mode.
void Wire3DCGB_ClearSparseStageFast();
// Mark an inclusive pixel rectangle for later transfer without drawing it.
// Supply ordered endpoints; values beyond 127/95 are clamped to the border.
// This tracks 128x96 tiles, not the full-screen tile allocator.
void Wire3DCGB_MarkDirtyRect2D(w3dcgb_u8 min_x, w3dcgb_u8 min_y, w3dcgb_u8 max_x, w3dcgb_u8 max_y);
// Store camera position and angles without changing existing pixels. Rotation
// helpers use the low four angle bits (16 steps per turn). Screen offsets are
// separate LCD registers and are not changed by this call.
void Wire3DCGB_SetCamera(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 pitch, w3dcgb_i8 yaw, w3dcgb_i8 roll);
// Draw a model at unity Q8 scale (256), using DrawModelScaled contracts.
void Wire3DCGB_DrawModel(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz);
// Project up to 24 vertices with positive Q8 scale; nonpositive scale means
// unity. Skip unity multiplication and zero-angle rotations. Intermediates
// must fit s16. Build up to 16 front-face flags when adjacency filtering is
// available, then draw depth-valid edges with valid vertex indices. The model
// edge count is not capped at 64. There is no near-plane clipping. Replace
// the shared projection cache and OR line colors into the normal 128x96 stage.
// This path does not mark sparse bounds; arrange a full upload or explicitly
// invalidate frame history before a sparse upload.
void Wire3DCGB_DrawModelScaled(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8);
// Draw a model at unity Q8 scale with a temporary color, restoring it afterward.
void Wire3DCGB_DrawModelColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_u8 color);
// Select color & 3 around DrawModelScaled and restore entry color afterward.
void Wire3DCGB_DrawModelScaledColor(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8, w3dcgb_u8 color);
// For a hidden-line model with vertices/faces, transform at most 24 vertices
// and erase front-facing triangles among at most 16 faces. Nonpositive scale
// means unity; this path still multiplies at unity and runs all three rotation
// helpers at zero angles. Reject invalid/depth-failed faces. Replace the shared
// projection cache and use the normal C stage clearer, which requires the
// caller or allocation to provide the correct WRAM mapping.
void Wire3DCGB_EraseModelFaces(const Wire3DCGB_Model* model, w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_i8 rx, w3dcgb_i8 ry, w3dcgb_i8 rz, w3dcgb_i16 scale_q8);
// Erase a triangle through the normal C stage clearer. Its clipped spans
// mark touched tiles dirty; caller/allocation must provide the stage WRAM mapping.
void Wire3DCGB_EraseTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy);
// Order X endpoints, reject an off-screen row or left edge, and clamp the
// right edge to 127. Mark the range dirty and use the ASM span clearer, which
// selects and restores WRAM bank 2.
void Wire3DCGB_EraseSpan2D(w3dcgb_u8 y, w3dcgb_u8 x0, w3dcgb_u8 x1);
/* Signed endpoints, clipped before conversion to the rasterizer's u8 ABI. */
// Clip a signed 2D line to the selected viewport, then draw with temporary
// color. Inputs outside +/-2047 are rejected. Fully in-bounds 128x96 lines
// use a fast path; other lines use outcodes and bounded integer intersections
// with an eight-iteration guard. Accepted normal-mode lines mark dirty bounds.
// This is screen clipping only, with no 3D projection or depth clipping.
void Wire3DCGB_DrawLineClipped2D(w3dcgb_i16 x0, w3dcgb_i16 y0,
                               w3dcgb_i16 x1, w3dcgb_i16 y1, w3dcgb_u8 color);
// Validate pointers, 1..24 vertices, at most 64 edges, every index and
// input coordinates/centers in +/-2047 before drawing. Fully visible geometry
// uses signed-byte scratch and the fast indexed path; other edges use the
// signed line clipper. That clipper may reject translated endpoints beyond
// +/-2047 even when individual inputs passed. Replace shared screen scratch
// and preserve the previous color through the called line APIs.
void Wire3DCGB_DrawEdgeListClipped2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
    const w3dcgb_i16* vx, const w3dcgb_i16* vy, w3dcgb_u8 vertex_count,
    w3dcgb_i16 cx, w3dcgb_i16 cy, w3dcgb_u8 color);
// Approximate value*48/z without a camera or screen-center offset. Negative
// or very small Z becomes 4. For Z above 255, shift depth and magnitude
// right together until depth fits a byte, then cap magnitude at 160.
// Return a signed offset; this helper does not reject near/far depths.
w3dcgb_i16 Wire3DCGB_ProjectAxis48(w3dcgb_i16 value, w3dcgb_i16 z);
// Mark current and both previous sparse ranges as all 192 normal-mode
// tiles. Pixel data is not cleared; subsequent uploads cannot trust old ranges.
void Wire3DCGB_InvalidateFrameHistory();
/* Startup-only: reserves both BG maps; queued HUD tiles are mirrored. */
// Enable map flipping once during 128x96 startup; ignore full-screen mode
// or an already-enabled setting. With LCD on, wait for VBlank, disable it,
// copy both banks of 9800 to 9C00, and assign viewport attributes to tile
// banks 0/1 respectively. Restore LCDC with map 9C00 selected and VBK zero.
// Both BG maps become reserved. Queued HUD tile numbers are mirrored.
void Wire3DCGB_EnableAtomicMaps();
// Order and clip the rectangle to 128x96. Full-height left strips of 16 or
// 24 pixels use bank-preserving ASM clears and dirty marking. Other rectangles
// use C row clears and require the correct stage WRAM mapping from the caller.
void Wire3DCGB_EraseRect2D(w3dcgb_u8 x0, w3dcgb_u8 y0, w3dcgb_u8 x1, w3dcgb_u8 y1);
// Clear the leftmost 24 stage columns through bank-preserving ASM. No dirty
// range is marked; sparse-transfer callers must include the cleared tiles.
void Wire3DCGB_EraseLeftGuard24Fast();
// Write a one-pixel color-2 border around the normal stage, preserving
// interior pixels and WRAM bank selection. Its visible color depends on the
// palette. No dirty range is marked; sparse callers must arrange its upload.
void Wire3DCGB_DrawWhiteBorderFast();
// Clear full-screen span history when full_mode is set; otherwise clear the
// 128x96 bitmap mask and its bounds. This does not erase displayed/staged
// pixels or change whether occlusion testing is enabled.
void Wire3DCGB_ClearOcclusionMask();
// Normalize active to zero or one and store it as the occlusion switch.
// No mask clearing or geometry marking occurs.
void Wire3DCGB_SetOcclusionActive(w3dcgb_u8 active);
// Mark a screen-space triangle without drawing pixels or enabling occlusion.
// Supply vertices inside the selected viewport. The active ASM adds a one-pixel
// X guard, limited to 0..127 in 128x96 mode or 0..159 in 160x144 mode.
// This function does not clip arbitrary out-of-bounds triangles.
void Wire3DCGB_MarkTriangle2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 cx, w3dcgb_u8 cy);
// Mark up to 29 silhouette rows in the 128x96 bitmap mask. Each packed byte
// uses its high nibble for center offset (nibble-7), low nibble for half-width;
// 0xFF skips a row. Place rows at y+y0+i, expand spans by two pixels, and clip
// to the viewport. profile must contain count readable bytes; null is ignored.
// This does not update full-screen spans or enable occlusion.
void Wire3DCGB_MarkPackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                     w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y);
// Erase up to 29 packed silhouette rows from both staged bitplanes. Format,
// placement, two-pixel guard and clipping match MarkPackedSilhouette2D; null
// is ignored. The ASM span writer selects/restores WRAM bank 2 but does not
// mark dirty tiles. In sparse mode the caller must ensure erased tiles are
// included in the upload range. This API targets the 128x96 stage.
void Wire3DCGB_ErasePackedSilhouette2D(const w3dcgb_u8* profile, w3dcgb_u8 count,
                                      w3dcgb_i8 y0, w3dcgb_u8 x, w3dcgb_u8 y);
// Sort at most eight objects near-to-far by camera-space origin depth,
// without changing the caller array. Clear the mask, draw visible models
// against earlier coverage, then add their faces as occluders. Model arrays
// and face indices must be valid. Color zero inherits the current color,
// possibly from the preceding object. Restore entry color and disable
// occlusion at the end. This model path targets the normal 128x96 stage.
void Wire3DCGB_DrawScene(Wire3DCGB_Object* objects, w3dcgb_u8 count);
// Draw at most three 2D cube approximations, sorted far-to-near by raw Z
// without reordering the caller array. Null/empty input is ignored. selected
// is an original array index. Camera/rotation are unused and color is not restored.
void Wire3DCGB_DrawFastCubes(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
// Draw small selection/hidden-mode indicators in color 3 and mark their
// normal-mode bounds dirty. selected is expected in 0..2; it is not checked.
// These indicators use lines, not font tiles, and the old color is not restored.
void Wire3DCGB_DrawFastStatus(w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
// Prepare the map, erase prior bounds/status cells, sort up to three cube
// stamps far-to-near by raw Z, then draw and upload numbers/attributes at
// VBlank. The caller array is not reordered. Non-null count zero clears
// old stamps; a null array is a complete no-op. selected & 3 chooses the
// status cell only, not a cube highlight. Requires loaded cube stamps, the
// full runtime and consistent FastMap WRAM mapping; it does not initialize them.
void Wire3DCGB_DrawFastCubeFrame(Wire3DCGB_FastCube* cubes, w3dcgb_u8 count, w3dcgb_u8 hidden_enabled, w3dcgb_u8 selected);
// Draw a six-edge projectile at an already-projected center using four
// depth size bands and four phase orientations. Reject centers outside
// X=10..117 or Y=10..85, mark the bounds dirty and leave the requested color
// selected. This uses the normal rasterizer and does not apply the camera.
void Wire3DCGB_DrawFastProjectile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_i16 z, w3dcgb_u8 phase, w3dcgb_u8 color);
// Mark FastMap prepared and clear tile numbers/attributes in its 16x12
// viewport. Shadow columns 16..31 are retained even though flush copies
// 32 columns per row. Requires the full runtime and a stable FastMap WRAM
// mapping; this does not initialize hardware or upload the map.
void Wire3DCGB_FastMapBegin();
// Mark FastMap prepared and clear only its 16x12 tile-number viewport.
// Keep attributes and columns 16..31. Requires the full runtime and the
// intended WRAM mapping; hardware initialization/upload is separate.
void Wire3DCGB_FastMapBeginTilesOnly();
// Add outline sides to a cell: bits 1/2/4/8 mean top/bottom/left/right.
// Same-color masks combine; another color replaces the old mask. Zero mask
// is ignored, zero color means MAIN, and color above 3 clamps to 3. Target
// the 16x12 shadow with stable WRAM mapping; full runtime required.
void Wire3DCGB_FastMapCell(w3dcgb_u8 tx, w3dcgb_u8 ty, w3dcgb_u8 color, w3dcgb_u8 mask);
// Order and clip byte tile coordinates to 16x12, then replace selected
// border cells. Side bits 1/2/4/8 mean top/bottom/left/right. Zero color
// means MAIN; values above 3 clamp to 3. Attributes and interior cells remain.
// Use at least two rows/columns for distinct corners; coincident corners
// are overwritten in sequence. Requires full runtime and stable WRAM mapping.
void Wire3DCGB_FastMapRect(w3dcgb_u8 tx0, w3dcgb_u8 ty0, w3dcgb_u8 tx1, w3dcgb_u8 ty1, w3dcgb_u8 color, w3dcgb_u8 sides);
// Wait for a fresh VBlank when LCD is enabled, then upload the first
// twelve complete map rows of tile numbers and attributes to map 9800.
// LCD-off callers proceed immediately. Requires the full runtime, stable
// WRAM mapping and enough safe time for both immediate GDMA transfers.
void Wire3DCGB_FastMapFlush();
// Wait for a fresh VBlank when LCD is enabled and upload only the first
// twelve complete rows of tile numbers to map 9800. Attributes remain.
// Requires the full runtime, stable WRAM mapping and a safe GDMA interval.
void Wire3DCGB_FastMapFlushTilesOnly();
// Project a world-space point through the current camera to the 128x96 viewport.
// Return zero for rejected depth and leave outputs unchanged; otherwise write
// clamped byte coordinates. Valid writable pointers are required. This is not
// the 160x144 full-screen projector. Absent from MINIMAL_RUNTIME builds.
w3dcgb_u8 Wire3DCGB_ProjectPoint(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_u8* sx, w3dcgb_u8* sy);
/* Fast path for games whose camera pitch, yaw, and roll stay at zero. */
// Subtract camera position and project to 128x96 while ignoring all camera angles.
// Use when camera pitch, yaw and roll are zero. Reject invalid depth without
// changing output pointers; successful coordinates are clamped to the viewport.
// Absent from MINIMAL_RUNTIME builds.
w3dcgb_u8 Wire3DCGB_ProjectPointNoRotation(w3dcgb_i16 x, w3dcgb_i16 y, w3dcgb_i16 z, w3dcgb_u8* sx, w3dcgb_u8* sy);
// Project both endpoints through the camera into the 128x96 viewport.
// Reject the whole line if either endpoint fails the depth test; do not clip
// near-plane crossings. DrawLine2D dispatches by mode, but projection remains
// 128x96. Normal lines accumulate color planes and do not mark sparse bounds;
// use a full upload or explicitly invalidate frame history for sparse upload.
void Wire3DCGB_DrawLine3D(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz);
// Temporarily select color & 3 for DrawLine3D, then restore the old color.
void Wire3DCGB_DrawLine3DColor(w3dcgb_i16 ax, w3dcgb_i16 ay, w3dcgb_i16 az, w3dcgb_i16 bx, w3dcgb_i16 by, w3dcgb_i16 bz, w3dcgb_u8 color);
// In normal mode, reject coordinates outside 128x96, mark sparse bounds
// and replace the staged pixel using the current color, including zero.
// Full-screen mode draws a coincident-endpoint fast line instead, so zero
// means color 3 and invalid coordinates latch that frame's overflow flag.
// Display transfer occurs later.
void Wire3DCGB_DrawPoint2D(w3dcgb_u8 x, w3dcgb_u8 y);
// Draw a six-pixel marker through the 128x96 plotter while preserving line
// color. Centers outside X=3..124 or Y=3..92 are ignored. The low five turn
// bits select left/straight/right nose and tail offsets; this is not a general
// 3D transform and does not dispatch to the full-screen renderer.
void Wire3DCGB_DrawTinyModel2D(w3dcgb_u8 x, w3dcgb_u8 y,
                               w3dcgb_u8 turn, w3dcgb_u8 color);
// Store byte endpoints and invoke the normal or full-screen line rasterizer.
// Provide coordinates inside the selected viewport; use DrawLineClipped2D
// for signed/off-screen geometry. Normal lines OR color planes and do not
// mark sparse bounds; use the clipped API or InvalidateFrameHistory before
// a sparse upload. Full-screen lines replace colors and use tile allocation.
// Both active line paths treat color zero as 3, not as an eraser.
void Wire3DCGB_DrawLine2D(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by);
// Set color & 3 around DrawLine2D and restore the previous color. Follow
// that function's coordinate, color-combination and sparse-range contracts;
// zero does not erase a line in either active renderer.
void Wire3DCGB_DrawLine2DColor(w3dcgb_u8 ax, w3dcgb_u8 ay, w3dcgb_u8 bx, w3dcgb_u8 by, w3dcgb_u8 color);
// Draw enabled model edges using signed-byte preprojected offsets around
// (x,y). Mask bits are least-significant-first, eight edges per byte. All
// pointers and vertex indices must be valid; edge_mask[0] is read even for
// zero edges. No counts are capped. Keep translated endpoints within the
// viewport before their byte conversion; negative values would wrap.
// Preserve the previous line color and dispatch by the active mode.
void Wire3DCGB_DrawMaskedModel2D(const Wire3DCGB_Model* model,
                                 const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                                 const w3dcgb_u8* edge_mask,
                                 w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color);
// Draw up to 64 indexed edges from signed-byte offsets around (cx,cy).
// Null arrays are ignored; indices must address readable vertices. Keep the
// translated endpoints in the viewport before byte conversion, which wraps.
// Dispatch by mode and preserve the old color; no vertex-count check occurs.
void Wire3DCGB_DrawEdgeList2D(const Wire3DCGB_Edge* edges, w3dcgb_u8 edge_count,
                              const w3dcgb_i8* vertex_x, const w3dcgb_i8* vertex_y,
                              w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 color);
// Draw up to 64 records of four bytes (x0,y0,x1,y1), ignoring a null list.
// Endpoints must lie in the selected viewport. Preserve the previous color
// and dispatch each line to the normal or full-screen rasterizer.
void Wire3DCGB_DrawLineList2DColor(const w3dcgb_u8* line_xy,
                                  w3dcgb_u8 line_count, w3dcgb_u8 color);
// Queue a tile-number write for a 32x32 BG map. Ignore invalid coordinates
// and submissions after 48 pending writes. EndFrame flushes this private queue;
// atomic-map mode mirrors writes to both BG maps. Attributes are not queued.
void Wire3DCGB_PutBgTile(w3dcgb_u8 x, w3dcgb_u8 y, w3dcgb_u8 tile);
// Normal mode flushes queued BG writes, transfers the full stage and
// presents the completed tile bank. Full-screen mode transfers/presents its
// allocated tiles and does not flush the HUD queue. Requires an enabled LCD
// and the transfer helpers' bank/interrupt contracts; may span multiple frames.
void Wire3DCGB_EndFrame();
// Transfer and present the complete selected-mode frame without flushing
// the BG queue. It does not consume queued HUD writes. Requires the same
// LCD/bank/interrupt conditions as EndFrame; full-runtime build required.
void Wire3DCGB_EndFrameFast();
// In normal mode, wait for a fresh VBlank and invoke EndFrameSparseNow.
// Full-screen mode uses its full transfer/presentation instead. Requires the
// full runtime and an enabled LCD; it is not guaranteed to complete in one frame.
void Wire3DCGB_EndFrameSparse();
// Flush queued BG writes, upload the sparse current/N-2 tile range to the
// back bank and present it. Skip the initial fresh-VBlank wait, but normal
// map mode still waits when restoring its blank HUD tile; DMA/presentation
// may also wait. Full-runtime full-screen mode delegates to its own path.
// Require an enabled LCD and keep SVBK/VBK stable while DMA is active.
void Wire3DCGB_EndFrameSparseNow();
// Return the current full-screen allocation count (up to 127). This is
// not a byte count or the number of visible/nonzero pixels. Full runtime only.
w3dcgb_u8 Wire3DCGB_GetFullScreenTileCount();
// Return the full-screen frame fault flag: the fast line path sets it for
// tile allocation exhaustion or an invalid coordinate. Once set, further
// fast-path plots are suppressed until BeginFrame resets it. This is not a
// general hardware status flag; the getter requires the full runtime.
w3dcgb_u8 Wire3DCGB_GetFullScreenOverflow();

#endif
