# Wire3D Guide

`wire3d.h` / `wire3d.c` は、KITAQGB で使う固定小数点ワイヤーフレーム 3D 描画ライブラリです。
利用側はモデルの頂点とエッジを渡し、`BeginFrame` から `EndFrame` までの間に描画要求を積みます。
複数オブジェクトを重ねる場合は `Wire3D_DrawScene()` を使うと、手前から描画し、手前オブジェクトの可視面マスクで奥の線を保守的にスキップできます。

## 最小利用例

```c
#include "wire3d.h"

__prg_rom Wire3D_Vec3 cube_vertices[8] = {
    { -24, -24, -24 }, { 24, -24, -24 }, { 24, 24, -24 }, { -24, 24, -24 },
    { -24, -24,  24 }, { 24, -24,  24 }, { 24, 24,  24 }, { -24, 24,  24 }
};

__prg_rom Wire3D_Edge cube_edges[12] = {
    { 0, 1 }, { 1, 2 }, { 2, 3 }, { 3, 0 },
    { 4, 5 }, { 5, 6 }, { 6, 7 }, { 7, 4 },
    { 0, 4 }, { 1, 5 }, { 2, 6 }, { 3, 7 }
};

Wire3D_Model cube;

void main(void)
{
    w3d_u8 frame = 0;

    cube.vertices = cube_vertices;
    cube.edges = cube_edges;
    cube.vertex_count = 8;
    cube.edge_count = 12;

    Wire3D_Init();

    while (1) {
        Wire3D_BeginFrame();
        Wire3D_DrawModel(&cube, 0, 0, 104, 0, (w3d_i8)(frame & 15), 0);
        Wire3D_EndFrame();
        frame = (w3d_u8)(frame + 1);
    }
}
```

## ビルド例

```powershell
kitaqgb lib/wire3d.c lib/physics3d.c examples/wire3d_cube_demo.c -I lib -o examples/wire3d_cube_demo.gb --profile=dev --stack-bank=fixed --rst-disable --rom-title=WIRE3DDEMO
```

同梱デモでは、A ボタンを押すと物理ボディ付きの表示オブジェクトが最大 6 個まで増えます。加速度、質量反発、強衝突時の破壊表示も `physics3d` 経由で動きます。

## 制約

- `Wire3D_Init()` は BG マップとタイルデータを Wire3D 用の 128x120 表示に初期化します。
- WRAM `0xD000` から始まるステージバッファと、VRAM `0x8900` から始まるタイルデータを使います。ビルド時は `--stack-bank=fixed` を指定してください。
- 1モデルあたりの頂点数は初期実装では `WIRE3D_MODEL_VERTEX_LIMIT` までです。
- 角度は 16 段階です。`0..15` の範囲が一周に対応します。
- 近すぎる頂点と遠すぎる頂点を含むエッジは描画されません。
- `Wire3D_DrawScene()` のオクルージョンは GB 上で回すため、可視三角面のスクリーン範囲を使った保守的なマスクです。正確な Z バッファではありません。
- `Wire3D_EndFrame()` は VBlank 開始を待ってから、WRAM ステージバッファを STAT gate 付きで VRAM へ転送し、転送済みのステージバイトを消します。

## 呼び出し順

```c
Wire3D_Init();

while (1) {
    Wire3D_BeginFrame();
    Wire3D_DrawModel(&model, 0, 0, 96, 0, angle, 0);
    Wire3D_EndFrame();
}
```
