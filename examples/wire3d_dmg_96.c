// Minimal wire3d_dmg 128x96 scene: a rotating cube without hidden-line filtering.
// Build from the repository root; see lib/wire3d_dmg_guide.md.
#define WIRE3D_DMG_HEIGHT 96
#include "wire3d_dmg.h"

__prg_rom Wire3DDMG_Vec3 cube_vertices[8] = {
    {-24,-24,-24}, {24,-24,-24}, {24,24,-24}, {-24,24,-24},
    {-24,-24,24}, {24,-24,24}, {24,24,24}, {-24,24,24}
};
__prg_rom Wire3DDMG_Edge cube_edges[12] = {
    {0,1}, {1,2}, {2,3}, {3,0}, {4,5}, {5,6},
    {6,7}, {7,4}, {0,4}, {1,5}, {2,6}, {3,7}
};
Wire3DDMG_Model cube;

// Initialize every model field, then redraw before each consuming stage upload.
// Angle zero through fifteen spans a full turn; no font or external asset is needed.
void main()
{
    w3ddmg_u8 frame = 0;
    cube.vertices = cube_vertices;
    cube.edges = cube_edges;
    cube.faces = 0;
    cube.edge_faces = 0;
    cube.edge_masks = 0;
    cube.vertex_count = 8;
    cube.edge_count = 12;
    cube.face_count = 0;
    cube.edge_mask_count = 0;
    cube.flags = 0;
    Wire3DDMG_Init();
    while (1) {
        Wire3DDMG_BeginFrame();
        Wire3DDMG_DrawModel(&cube, 0, 0, 104, 0, (w3ddmg_i8)(frame & 15), 0);
        Wire3DDMG_EndFrame();
        frame = (w3ddmg_u8)(frame + 1);
    }
}
