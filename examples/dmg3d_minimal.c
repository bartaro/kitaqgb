// Minimal DMG3D scene: a centered cross projected from two world-space segments.
// Build from the repository root using the command in lib/dmg3d_guide_ja.md.
#include "dmg3d.h"

// Uploading consumes the stage; draw both segments again for every displayed frame.
// Dirty transfer tracks the tiles that contain the cross. Auxiliary transfer stays off.
void main()
{
    DMG3D_Init();
    DMG3D_SetDirtyTransfer(1);
    while (1) {
        DMG3D_BeginFrame();
        DMG3D_DrawLine3D(-24, 0, 100, 24, 0, 100);
        DMG3D_DrawLine3D(0, -24, 100, 0, 24, 100);
        DMG3D_EndFrame();
    }
}
