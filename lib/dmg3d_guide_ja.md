# DMG3D DMGワイヤーフレーム描画ライブラリ

**wire3d_dmg** は、ゲームボーイ向けのモノクロ描画ライブラリです。この文書の旧API名は互換入口で使用できます。新規開発は [wire3d_dmgガイド](wire3d_dmg_guide_ja.md) を参照してください。

`dmg3d.h` / `dmg3d.c` は、KITAQGBでDMG向け1bppワイヤーフレーム描画を使うためのライブラリです。

## 特徴

- 画面は `128x120` のワイヤーフレーム面として扱います。
- 描画先はWRAMの `D000` ステージです。
- `DMG3D_EndFrame()` がVBlankを待ち、STATを確認しながら `D000 -> 8900` へ転送します。
- 線分描画、ステージクリア、VRAM転送はインラインアセンブラです。
- BGマップは `0x9800+0x23` 起点で、タイルIDを横方向に15ずつ進める列優先配置です。
- 補助転送 `D518 -> 90A0` は `DMG3D_SetAuxTransfer(1)` で明示的に有効化できます。

## ビルド例

```powershell
kitaqgb lib/dmg3d.c examples/dmg3d_demo.c -I lib -o examples/dmg3d_demo.gb --profile=dev --stack-bank=fixed --rst-disable --rom-title=DMG3DDEMO
```

または:

```powershell
.\examples\dmg3d_demo_build.ps1
```

## 基本フロー

```c
#include "dmg3d.h"

void main()
{
    DMG3D_Init();

    while (1)
    {
        DMG3D_BeginFrame();
        DMG3D_SetCamera(0, 0, 0, 0, 0, 0);
        DMG3D_DrawLine3D(-20, 0, 100, 20, 0, 100);
        DMG3D_EndFrame();
    }
}
```

## モデル描画

`DMG3D_Model` には頂点、辺、面、辺に対応する面番号を渡します。`DMG3D_MODEL_HIDDEN_LINES` を指定すると、面の向きに基づいて隠線を抑制します。

```c
DMG3D_DrawModelScaled(&model, 0, 0, 120, rx, ry, rz, 256);
```

`DMG3D_DrawIndexedEdges()` は、頂点を一括投影して辺のインデックス列を描く入口として使えます。ROM固有のオブジェクトコマンドやdemo/state machineには依存しません。

## 注意

- `dmg3d.c` と `wire3d.c` は同じ実装の互換入口です。同じROMへ同時リンクする用途は想定していません。
- `DMG3D_EndFrame()` はVRAM転送を所有します。ゲーム側で同じフレーム中に大量のVRAM書き込みをする場合は、呼び順を固定してください。
- 補助転送はデフォルト無効です。必要な描画経路だけで有効化してください。
