// Minimal wire3d_dmg 128x120 scene: a centered cross projected from two world-space segments.
// Build from the repository root; see lib/wire3d_dmg_guide.md.
#define WIRE3D_DMG_HEIGHT 120
#include "wire3d_dmg.h"

// Uploading consumes the stage; draw both segments again for every displayed frame.
// Dirty transfer tracks the tiles that contain the cross. Auxiliary transfer stays off.
void main()
{
    Wire3DDMG_Init();
    Wire3DDMG_SetDirtyTransfer(1);
    while (1) {
        Wire3DDMG_BeginFrame();
        Wire3DDMG_DrawLine3D(-24, 0, 100, 24, 0, 100);
        Wire3DDMG_DrawLine3D(0, -24, 100, 0, 24, 100);
        Wire3DDMG_EndFrame();
    }
}
