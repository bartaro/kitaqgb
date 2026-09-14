# KITAQGB库

[English](README.md) | [日本語](README.ja.md) | **简体中文**

[打开KITAQGB库简体中文手册](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html)，查看各函数的说明和示例代码。

`wire3d_dmg` 是Game Boy单色线框渲染器。128×96请选择 `wire3d_dmg_96.c`，128×120请选择 `wire3d_dmg.c`，并使用 `Wire3DDMG_*` 函数。`wire3d` 和 `dmg3d` 分别保留为这两种分辨率的兼容入口。每个ROM只编译一个入口。彩色专用的 `wire3d_cgb` 仍是独立渲染器。

[渲染器指南](wire3d_dmg_guide.md) / [日文指南](wire3d_dmg_guide_ja.md)

本目录包含三类文件：

- 公开API：游戏中包含相应头文件，并与所需源码一起构建的可复用库。
- 辅助单元：可选支持代码、寄存器声明或占位源码，本身不构成独立的公开API。
- 参考文档：仅用于阅读，不链接进ROM。

## 文件分类

### 公开API

| 文件 | 用途 | 常规用法 |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 轴对齐包围盒（AABB）的2D物理、重力积分和迭代接触求解。 | 包含 `physics2d.h`，并编译 `physics2d.c`。 |
| `physics2d_circle.h` / `physics2d_circle.c` | 适合球类游戏的圆形物体2D物理。 | 包含头文件并编译源码。 |
| `physics3d.h` / `physics3d.c` | 带加速度、质量加权反弹和破坏标志的3D AABB物理，以及 `kq3d_dot_q8_8()`。 | 包含头文件并编译源码。 |
| `wire3d.h` / `wire3d.c` | 定点数3D线框渲染，使用WRAM暂存缓冲区，支持模型隐藏线与场景遮挡掩码。 | 包含 `wire3d.h`，并编译 `wire3d.c`。 |
| `dmg3d.h` / `dmg3d.c` | DMG暂存式线框渲染：D000处的128×120表面、固定顺序的内联汇编画线，以及受STAT状态控制的D000→8900传输。 | 需要1bpp暂存绘制流程时，包含头文件并编译源码。 |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB专用8MHz彩色线框渲染，包含2bpp WRAM缓冲区、隐藏线与场景遮挡、汇编裁剪绘制及HBlank DMA无撕裂显示。 | 构建CGB专用ROM，并加入头文件和源码。 |
| `system.h` / `system.c` | 初始化、帧计数、VBlank等待、协作式VBlank回调、DI/EI封装。 | 用于按帧推进的游戏循环。 |
| `input.h` / `input.c` | 每帧按钮状态：按住、刚按下、刚松开与重复输入。 | 用于菜单、动作、益智和策略游戏控制。 |
| `vram.h` / `vram.c` | 安排BG图块写入、矩形填充、地图块复制、memcpy与memset的VRAM命令队列。 | 游戏运行期间入队，在安全时段调用 `vram_flush()` 或 `vram_flush_now()`。 |
| `sprite.h` / `sprite.c` | OAM影子缓冲区、精灵分配、组合精灵、动画推进、OAM DMA刷新与扫描线溢出检查。 | 用于基于OBJ的显示。 |
| `fixed.h` / `fixed.c` | Q8.8定点数、`Vec2`、`KQRect`、clamp/min/max/lerp及基础矩形检测。 | 用于移动、物理、镜头和AI评分等。 |
| `scene.h` / `scene.c` | 轻量场景表及切换、更新、绘制分派，可组织标题、游戏、暂停等状态。 | 用于构建游戏状态流程。 |
| `entity.h` / `entity.c` | 固定数组对象池，最多容纳 `ENTITY_MAX` 个小型游戏实体。 | 回调接收实体ID，通过 `entity_get(id)` 取得实体数据。 |
| `danmaku.h` / `danmaku.c` | 96发定点弹幕池、32方向扇形弹、命中与擦弹事件，以及不受OAM数量限制的CGB BG图块合成。 | 包含头文件并编译源码，参阅 `danmaku_guide.md` 和完整游戏 `ressen_gbc`。 |
| `bank.h` / `bank.c` | 基于内建操作的远端数据、远指针、远调用与简单MBC存储体切换。 | 用于跨存储体访问的封装。 |
| `asset.h` / `asset.c` | 资源ID描述表及原始数据、图块加载。 | 包含头文件并编译源码；以后生成的 `assets.h/c/json` 也可采用此结构。 |
| `debug.h` / `debug.c` | ROM端轻量跟踪、断言和标记缓冲区，供KOKURA或其他模拟器查看。 | 将高开销的性能分析放在ROM之外。 |
| `chain.h` / `chain.c` | 蛇、绳索、列车和关节精灵等对象的坐标历史环形缓冲区。 | 用于分节对象沿历史位置移动。 |
| `cgb_tile.h` | CGB图块及属性内建操作的公开声明。 | 游戏使用 `__settile...` 等功能时包含。 |
| `cgb_palette.h` / `cgb_palette.c` | CGB BG/OBJ调色板的高层接口。 | 包含头文件并编译源码。 |
| `scroll.h` / `scroll.c` | 基于编译器内建操作的滚动与分屏表。 | 使用 `Scroll_*` 时加入。 |
| `raster.h` / `raster.c` | 分带光栅滚动及逐扫描线X方向变形配置。 | 包含 `raster.h`，同时编译 `raster.c` 和 `scroll.c`。 |
| `camera.h` / `camera.c` | 基于 `scroll.*` 的8.8定点镜头、简单全局接口、世界坐标与屏幕坐标转换。 | 包含头文件并编译源码。 |
| `audio.h` / `audio.c` | 共用Game Boy音频驱动，支持音乐、音效、声像、波形和淡变，音符编号覆盖0至67（`G6`），共68个。 | 加入需要音频的项目。 |
| `audio_vblank.h` / `audio_vblank.c` | VBlank IRQ BGM驱动，同样支持68个音符编号，提供16记录WRAM队列以播放跨存储体歌曲，并可挂接逐帧处理。 | 直接指针歌曲放在固定存储体0，或从其他存储体补充队列；用 `scripts/patch_gb_vblank_irq.ps1` 设置向量 `0x0040`。 |
| `link.h` / `link.c` | 通信线串行字节传输，以及协作式逻辑4人通信 `Link4_*`。 | 加入需要通信的项目。 |
| `link_packet.c` | `link.c` 上的可选包层，含 `Link4_*` 的逐对端收件箱。 | 需要包收发时才与 `link.c` 一起编译。 |
| `link_dmg07.h` / `link_dmg07.c` | 面向实体Nintendo DMG-07 Four Player Adapter的外部时钟轮询驱动。 | 与 `link_hwregs_gb.c` 一起编译；它与逻辑 `Link4_*` API独立。 |
| `rpg.h` | RPG/ADV/SLG共用声明和底层内建操作声明。 | 使用这一功能族时在游戏源码中包含。 |
| `rng.c` | `rng8`、`rng16`、`rand_range`、`weighted_choice`、`rng_seed`、`rng_next8`、`rng_next16`、`rng_range`、`rng_chance`。 | 使用 `rpg.h` 的随机函数时编译。 |
| `flags.c` | 2048个标志位的位集和任务状态存储。 | 使用 `rpg.h` 的标志、任务功能时编译。 |
| `rle.c` | RAM和远端ROM中的简单 `[数量][值]` RLE解码。 | 使用 `rpg.h` 的 `rle_decode*` 时编译。 |
| `text.c` | 图块字符串窗口、翻页等待、选项、直接XY文本、数值显示和清除/窗口别名。 | 使用 `rpg.h` 的文本功能时编译。 |
| `menu.c` | 竖排菜单、最小物品菜单和非阻塞菜单状态API。 | 使用 `rpg.h` 的菜单功能时编译。 |
| `script.c` | RPG/ADV流程的小型字节码执行器。 | 使用 `rpg.h` 的脚本功能时编译。 |
| `map.c` | 打包地图加载，含碰撞、触发器、镜头及可选16×16元图块支持。 | 使用 `rpg.h` 的地图功能时编译。 |
| `save.c` | MBC5方式的SRAM保存、读取、检查和清除，包含头部、版本、长度及校验和。 | 使用 `rpg.h` 的保存功能时编译。 |
| `slg_unit.c` | 策略游戏的移动与攻击范围。 | 使用 `rpg.h` 的战术单位功能时编译。 |
| `slg_path.c` | 广度优先寻路与移动代价洪泛填充。 | 使用 `rpg.h` 的战术寻路功能时编译。 |
| `slg.h` / `slg_board.c` | 棋盘、着法列表和撤销栈等通用棋类或战术系统功能。 | 包含 `slg.h`，编译 `slg_board.c`；游戏特有的评价逻辑另行实现。 |

### 辅助单元

| 文件 | 用途 | 说明 |
| --- | --- | --- |
| `audio_hwregs_gb.c` | 最小APU及波形RAM寄存器声明。 | 项目未通过其他文件声明这些寄存器时才使用。 |
| `link_hwregs_gb.c` | 通信项目所需的最小 `SB`、`SC`、`IF`、`IE` 声明。 | 若其他文件已经声明串行寄存器，不要重复加入。 |
| `cgb_tile.c` | 有意保留为空的CGB图块功能编译单元。 | 公开接口在 `cgb_tile.h`；编译此文件没有妨碍，但不是运行逻辑所必需的。 |
| `math.c` | ROM正弦表 `MATH_SIN`。 | 尚未作为稳定公开API进行文档化，目前按项目辅助数据单元使用。 |

### 参考文档

| 文件 | 内容 |
| --- | --- |
| `README.md` | 本概览及构建说明的英文版。 |
| `wire3d_guide_ja.md` | 线框3D渲染器日文入门指南。 |
| `dmg3d_guide_ja.md` | DMG暂存式线框渲染器日文入门指南。 |
| `wire3d_cgb_guide.md` | CGB专用彩色线框渲染器入门指南。 |
| `physics_guide.html` | 物理库英文使用指南。 |
| `physics_guide_ja.html` | 物理库日文使用指南。 |

## 构建用法

以下命令示范如何将应用程序源码与库模块一起构建。请自行准备命令中指定的应用程序文件。随附的入门程序可按 `../examples/build.ps1` 和HTML手册构建。短命令 `kitaqgb` 要求可执行文件已加入PATH。

把游戏源码与所需库源码一起编译：

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

线框3D项目需要一并编译渲染器源码：

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

随附的 `examples/wire3d_minimal.c` 初始化所有模型字段，并在128×96视口中旋转立方体，不需要外部图形或字体资源。

采用128×120视口的DMG项目可使用120行兼容入口 `dmg3d.*`：

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` 使用D000处的WRAM暂存区和从0x8900开始的图块上传，配置128×120可见线框表面。`DMG3D_BeginFrame()` 只重置遮挡状态；上传会消耗并清除像素。`DMG3D_EndFrame()` 先等待VBlank，再查询STAT完成传输，传输可能延续到VBlank之后。随附 `examples/dmg3d_minimal.c` 启用差分传输，每帧重画十字。辅助传输是独立操作，与主暂存区共享源存储空间。

CGB专用彩色项目使用独立的 `wire3d_cgb.*`：

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` 将CGB切换到双倍速，设置128×96、2bpp的BG线框表面，并安装默认四项BG调色板。普通与 `Fast` 两组帧API都采用无撕裂显示：通过HBlank DMA把 `0xD300–0xDEFF` 的3072字节送往未显示的VRAM图块存储体，再于VBlank切换显示，不需要逐帧更改LCDC。`Fast` 组省略普通帧结束时的BG队列检查。可通过 `Wire3DCGB_SetPaletteRGB15()`、`Wire3DCGB_SetLineColor()` 和 `Wire3DCGB_Draw*Color()` 指定颜色。

对于由CAD生成的分方向LOD，`Wire3DCGB_DrawMaskedModel2D()` 接受预投影的有符号顶点偏移及打包的可见边掩码。遍历边和汇编光栅化都在渲染器存储体4内完成，因此绘制一个模型时，不必为每条线进行跨存储体调用。

画面变化较少且使用OAM影子缓冲区的游戏，可在进入VBlank后依次调用 `sprite_flush_oam()` 和 `Wire3DCGB_EndFrameSparseNow()`。这会跳过最初等待下一次VBlank的步骤，但DMA和显示切换仍可能因修改范围及当前扫描线而等待，不能保证全部工作在同一次VBlank内完成。

CGB线条请使用颜色1、2、3。普通128×96模式叠加颜色位，因此1与2重叠得到3。颜色0不能擦除线条；请清空帧或使用专用擦除函数。普通 `Wire3DCGB_DrawLine2D` 和模型绘制不记录差分上传范围。需要同时记录范围的线条请用 `Wire3DCGB_DrawLineClipped2D`，或调用 `Wire3DCGB_InvalidateFrameHistory` 将整个视口纳入下一次差分上传。

160×144模式每帧最多分配127个图块。高速画线路径遇到分配失败或越界坐标时，会设置 `Wire3DCGB_GetFullScreenOverflow()` 并停止后续像素写入，直到下一帧重置。请保持顶点位于所选视口内。三角形掩码的右侧余量在128×96模式下止于X=127，在全屏模式下止于X=159。请遵守API中的WRAM存储体映射要求，尤其是全屏和FastMap功能。

[CGB三角形掩码边界回归测试](../tests/library/wire3d_cgb_mask_bounds.c)提供了检查两种视口的完整程序。

RPG/ADV/SLG功能的构建示例：

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

标准运行时基本功能检查的构建示例：

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

串行通信项目应在通信核心之前编译串行寄存器声明单元：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

由主机选择通信对端的协作式逻辑4人项目也使用这些文件。主机调用 `Link4_InitHost(slot_count)`，用 `Link4_SelectPeer()` 或 `Link4_SendPacketTo()` 选择对端；其他参与者调用 `Link4_InitPeer(local_slot, slot_count)`，与槽位0的主机通信。

也可以直接构建已指定槽位的对端包装示例：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

实体DMG-07项目使用专用轮询驱动：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

请持续调用 `LinkDmg07_Poll()`，因为适配器字节之间的间隔远短于一个视频帧。每次VBlank调用一次 `LinkDmg07_TickFrame()`，更新饱和式静默与握手超时计数。驱动始终使用外部时钟 `SC=$80`，对连接探测回复 `88 88 RATE 01`，只有物理玩家1可发送 `AA AA AA AA` 请求传输。

所有主机观察到 `CC CC CC CC` 后，每个四字节广播包都包含各物理槽位的一字节数据。适配器在提交后的下一个包才广播数据，因此驱动丢弃第一个未定义包，并提供收发序号。`LinkDmg07_RequestRestart()` 等待下一个包边界，发送对齐的 `FF FF FF FF`，收到适配器完整的全FF指示后停止。传输阶段静默超时也会自动预约此重启，同时保留当前四字节包内的位置；适配器时钟恢复后，可先完成当前包再进入恢复流程。

游戏代码随后包含所需头文件：

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## 使用说明

最后八个音符编号目前重用了前一八度的频率。支持68个编号不代表能发出68种不同音高。

- `inv_mass_q8 == 0` 表示静态物体。
- `Wire3D_Init()` 使用128×96的BG线框表面、从 `0xD000` 开始的WRAM暂存区和从 `0x8900` 开始的图块数据。Wire3D项目请使用 `--stack-bank=fixed`。
- `Wire3D_BeginFrame()` 清空WRAM暂存区；`Wire3D_EndFrame()` 等待VBlank，再按STAT条件成块复制到VRAM。
- Wire3D角度分为16档，基本模型路径每个模型最多支持 `WIRE3D_MODEL_VERTEX_LIMIT` 个顶点。
- 多个带面的线框物体重叠时可用 `Wire3D_DrawScene()`。它先画近物体，累积可见面掩码，保守地跳过远处被遮挡的线条。
- `Wire3DCGB_Init()` 仅支持CGB，并使用KEY1/STOP切换双倍速。构建时指定 `--cgb=cgb_only`，不要在同一ROM混用 `wire3d_cgb.*` 和DMG兼容的 `wire3d.*`。
- 普通与 `Fast` 帧API都在非显示VRAM图块存储体传输完成前保留旧帧。不使用BG图块写队列的场景优先使用 `Fast`。
- 声明 `NR10..NR52` 和 `WAVE0..WAVE15` 的文件必须先于 `lib/audio.c` 编译。
- `lib/audio_hwregs_gb.c` 已提供这些声明，不要与其他重复声明相同音频寄存器的文件同时编译。
- `cgb_tile.h` 直接公开内建操作；`lib/cgb_tile.c` 仅是占位单元，普通构建可以省略。
- `cgb_palette.h` 的公开API使用 `cgb_*` 命名。
- 菜单或设置改变音乐、音效启用状态时，调用 `Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()`。
- `Audio_PlaySFX()` 记录当前可见ROM存储体。已明确知道音效数据所在存储体时，用 `Audio_PlaySFXBanked(bank, sfx, priority)`。
- 音乐流的 `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST` 使用以下通道编号：`0=CH1`、`1=CH2`、`2=CH4`、`3=CH3`。
- 自定义CH3波形时，将32个4位采样打包为16字节，传给 `Audio_LoadCustomWave()`。
- `Audio_FadeToMasterVolume()` 的淡变由 `Audio_Update()` 推进，淡变期间仍要每帧调用。
- `audio_vblank.c` 定义VBlank IRQ向量符号 `__kq_vblank_vector`。BGM事件为五字节：`delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`；休止、循环和结束使用 `AUDIO_VBLANK_REST`、`AUDIO_VBLANK_LOOP`、`AUDIO_VBLANK_END`。
- 直接指针VBlank歌曲必须放在固定存储体；队列模式可从其他存储体补充。链接 `lib/audio_vblank.c` 后运行 `scripts/patch_gb_vblank_irq.ps1 <rom> <map>`，使向量 `0x0040` 跳转至ISR并更新ROM校验和。
- 若未增加共享IRQ分派器，不要将 `audio_vblank.c` 与其他同样占用VBlank向量 `0x0040` 的库或游戏桩代码结合。
- `Scroll_SplitCommit()` 自动启用IE位 `0x01 | 0x02`，并使用编译器提供的VBlank/STAT处理程序播放分屏配置。
- 分屏功能在当前构建中保留向量 `0x0040` 和 `0x0048`，暂时不要与独立自定义VBlank/STAT桩代码混用。
- 声明 `SB`、`SC`、`IF`、`IE` 的文件应先于 `lib/link.c` / `lib/link_packet.c` 或 `lib/link_dmg07.c` 编译。
- `lib/link_hwregs_gb.c` 已提供这些声明，不要与其他重复声明相同串行寄存器的文件同时编译。
- 通信库不占用串行向量 `0x0058`。启用中断模式后，应从自己的IRQ桩代码或分派器调用 `Link_OnSerialIRQ()`。
- 包层只保留一层待收数据，适合由逐帧主循环及时驱动。
- `Link4_*` 模拟主机选择对端的协作式4人连接；每次只有一个对端在线路上活动，主机必须主动轮换。
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()` 提供逐对端收件箱，让主机轮询多个参与者时保留数据来源。
- `Link_ReadPacket()` 读取最近收到的一个包；4人流程请用 `Link4_ReadPacketFrom()`。
- `Link4_*` 不实现Nintendo DMG-07的电气行为或协议。实体配件使用 `link_dmg07.c`，不要在同一ROM中与 `link.c` 同时编译。
- DMG-07的 `GetConnectedMask()` 用位0~3表示物理玩家1~4。传输期间保留最后一次连接探测结果；成员情况只能在探测阶段刷新。
- 适配器未提供时钟时，待处理的DMG-07重启无法推进。恢复流量会被丢弃，不作为正常序列数据公开。若配件重新上电进入不同阶段，而非仅暂停时钟，应显式重新初始化驱动和会话。
- 物理库只计算线性位置和速度，不包含角动力学。
- 在Game Boy级硬件上应控制物体数量，例如保持8~24个活动物体。
- 按每个世界的玩法需要调整重力、最大速度和求解迭代次数。
- 台球类游戏优先使用 `physics2d_circle.*`，而非AABB库。
- 当前结构不需要另加 `random`、`collision`、`ui`、`tilemap`、`dialog`、`board_game`、`simple_physics` 库。对应使用 `rng`、`physics2d`、`text`/`menu`、`map`、`script`、`slg`、`physics2d`。
- `scene.c` 和 `entity.c` 避免在函数指针调用中使用指针大小的参数。当前KITAQGB的函数指针调用路径在无参数或使用单字节ID时最可靠。
