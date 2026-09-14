# Wire3D Guide

**wire3d_dmg** は、ゲームボーイ向けのモノクロ描画ライブラリです。この文書の旧API名は互換入口で使用できます。新規開発は [wire3d_dmgガイド](wire3d_dmg_guide_ja.md) を参照してください。

`wire3d.h` / `wire3d.c` は、KITAQGB で使う固定小数点ワイヤーフレーム 3D 描画ライブラリです。
利用側はモデルの頂点とエッジを渡し、`BeginFrame` から `EndFrame` までの間に描画要求を積みます。
複数オブジェクトを重ねる場合は `Wire3D_DrawScene()` を使うと、手前から描画し、手前の面の外接矩形と線上の5点を使って奥の線を省略します。厳密な画素単位の隠線処理ではなく、見える部分まで隠す場合もあります。

## 最小利用例

```c
// Minimal monochrome Wire3D scene: a rotating cube without hidden-line filtering.
// Build from the repository root using the command in lib/wire3d_guide_ja.md.
#include "wire3d.h"

__prg_rom Wire3D_Vec3 cube_vertices[8] = {
    {-24,-24,-24}, {24,-24,-24}, {24,24,-24}, {-24,24,-24},
    {-24,-24,24}, {24,-24,24}, {24,24,24}, {-24,24,24}
};
__prg_rom Wire3D_Edge cube_edges[12] = {
    {0,1}, {1,2}, {2,3}, {3,0}, {4,5}, {5,6},
    {6,7}, {7,4}, {0,4}, {1,5}, {2,6}, {3,7}
};
Wire3D_Model cube;

// Initialize every model field, then redraw before each consuming stage upload.
// Angle zero through fifteen spans a full turn; no font or external asset is needed.
void main()
{
    w3d_u8 frame = 0;
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
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

公開サンプル `examples/wire3d_minimal.c` は、モデルの全フィールドを初期化して立方体を回転表示します。外部の画像やフォントは不要です。上のコマンドはリポジトリのルートで実行してください。

## 制約

- `Wire3D_Init()` は BG マップとタイルデータを Wire3D 用の 128x96 表示に初期化します。
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
