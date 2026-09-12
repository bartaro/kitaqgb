# X3D DMGワイヤーフレーム描画ライブラリ

`x3d.h` / `x3d.c` は、KITAQGBでX式に近いDMG向け1bppワイヤーフレーム描画を使うためのライブラリです。

## 特徴

- 画面は `128x120` のワイヤーフレーム面として扱います。
- 描画先はWRAMの `D000` ステージです。
- `X3D_EndFrame()` がVBlankを待ち、STATを確認しながら `D000 -> 8900` へ転送します。
- 線分描画、ステージクリア、VRAM転送はインラインアセンブラです。
- BGマップは `0x9800+0x23` 起点で、タイルIDを横方向に15ずつ進めるX式配置です。
- 補助転送 `D518 -> 90A0` は `X3D_SetAuxTransfer(1)` で明示的に有効化できます。

## ビルド例

```powershell
kitaqgb lib/x3d.c examples/x3d_demo.c -I lib -o examples/x3d_demo.gb --profile=dev --stack-bank=fixed --rst-disable --rom-title=X3DDEMO
```

または:

```powershell
.\examples\x3d_demo_build.ps1
```

## 基本フロー

```c
#include "x3d.h"

void main()
{
    X3D_Init();

    while (1)
    {
        X3D_BeginFrame();
        X3D_SetCamera(0, 0, 0, 0, 0, 0);
        X3D_DrawLine3D(-20, 0, 100, 20, 0, 100);
        X3D_EndFrame();
    }
}
```

## モデル描画

`X3D_Model` には頂点、辺、面、辺に対応する面番号を渡します。`X3D_MODEL_HIDDEN_LINES` を指定すると、面の向きに基づいて隠線を抑制します。

```c
X3D_DrawModelScaled(&model, 0, 0, 120, rx, ry, rz, 256);
```

`X3D_DrawIndexedEdges()` は、X式の「頂点を一括投影して、エッジ列だけ描く」入口として使えます。ROM固有のオブジェクトコマンドやdemo/state machineには依存しません。

## 注意

- `x3d.c` と `wire3d.c` は別レンダラです。同じROMへ同時リンクする用途は想定していません。
- `X3D_EndFrame()` はVRAM転送を所有します。ゲーム側で同じフレーム中に大量のVRAM書き込みをする場合は、呼び順を固定してください。
- 補助転送はデフォルト無効です。必要な描画経路だけで有効化してください。
