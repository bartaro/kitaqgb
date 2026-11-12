# GBSEQ — 用户手册

[MIDI 文件与设备、演奏键盘、录音与 512 种音色](docs/MIDI_PERFORMANCE.md) · `K` · `import-midi` / `export-midi`

导出的ROM包含用于启动兼容性验证的48字节任天堂标志数据。GBSEQ的MIT许可证不授予任天堂标志或商标的独立使用权，GBSEQ与任天堂没有合作关系，也未获其认可。[英文／日文声明](docs/NINTENDO_NOTICE.md)。

在导出菜单中选择可编辑的 UGE（含自定义波形）或 MOD（GBT 固定音色）。每台主机单独导出一个文件，存入 `assets/music`。 [CLI: `export-uge` / `export-mod`](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ由DAISUKE OBA制作，提供plita桌面GUI和独立CLI，共享编辑模型、校验、KOKURA音源与导出功能。LOCAL 4CH、CABLE 8CH、DMG07 16CH分别支持1、2、4台Game Boy。每台CH1/CH2为脉冲波、CH3为波形、CH4为噪声。每首1–256小节，每小节16步（每通道最多4096步），BPM 30–300，可保存循环开关。 在「视图 → 语言」选择界面语言。此设置与乐曲分开保存。 `--language en|ja|ko|zh-CN|zh-TW|es|pt-BR|fr|de`

## 演奏输入与 WAV 导出

选择目标轨道，按 K 打开键盘。白键为 A/S/D/F/G/H/J，黑键为 W/E/T/Y/U，Z/X 切换八度。点击 IN 和 OUT 切换 MIDI 设备。步进输入每次前进 1、2 或 4 个十六分音符；录音从曲首开始录一遍。保护模式会跳过重叠输入，覆盖模式只替换重叠的区间。从文件菜单导入 MIDI，从导出菜单写出 MIDI。请用 .gbseq 保留 GB 专用音色。

每种声道有 128 个预设，可从下拉列表选择；用户音色可单独保存为 .gbpreset 并重新载入。和弦标记可分别为总混音和各轨道开启。整曲移调不会改变噪声轨。

WAV 遵循开始导出时的监听、静音与独奏设置。监听全部会混合当前模式的所有机器，P1–P4 只导出指定机器。一旦启用独奏，就只输出未静音的独奏轨。仅选中轨道进行编辑不会改变声音。所有轨道静音时，WAV 无声但长度不变。

`gbseq-cli render-wav song.gbseq mix.wav` · `gbseq-cli render-wav song.gbseq p2.wav --player 2`

[导出 VBlank 驱动和乐曲的 C 源码](docs/KITAQGB_VBLANK.md): `gbseq-cli export-vblank song.gbseq output` · `--strict` · `--driver-dir`.
记录采用 audio.h 的声道顺序。VBlank 驱动的 CH3 仅有三角波和锯齿波；如不允许近似，请使用 --strict。每台机器的源码单独编译。

按 **E** 或双击轨道标题即可打开该轨的钢琴卷帘编辑器；再按 E 返回总览。拖动音符右端可将它延长至下一音符之前或曲末。在 CH3 自定义波形图上可直接绘制 32 个采样点，Ctrl+Z 撤销一笔。

## 按声部导出播放 ROM 与监听

在导出菜单选择 **Playback ROMs (no SRAM)**，即可生成 P1.gb 至 P4.gb；每个 ROM 为 512 KiB，内含曲名和本机的四个声道，无需存档。按 START 可独立试听。联机时在各机停止状态按 SELECT，一台按 A 选 MASTER，其余按 B 选 PEER，再在主机按 START 播放或停止。四人适配器的主机接 1 号端口，与声部编号无关。在 GBSEQ 中可独立于编辑视图监听全部或单个声部，WAV 按当前监听选择导出。

`gbseq-cli export-player-roms song.gbseq output_folder`

## 安装与启动

从[bin/](bin/)选择Windows x86_64、Linux x86_64、macOS Apple Silicon或Intel包，完整解压，保留程序旁的assets与许可证。Windows GUI为gbseq.exe，Unix为./gbseq。以下CLI示例的gbseq-cli，在PowerShell中写为./gbseq-cli.exe，在Unix中为./gbseq-cli；加入PATH后可省略前缀。正常使用不需要编译器、另装plita或Python。

GUI需要桌面、兼容GPU驱动及试听用音频设备。Linux面向Ubuntu 24.04及更新的兼容系统，使用ALSA、X11/Wayland、libxkbcommon及libxkbcommon-x11与Vulkan/OpenGL运行库。macOS程序未签名、未公证；核对来源及哈希后，如有需要，通过系统“打开”或“隐私与安全性”仅允许此应用，不要关闭全局保护。CLI生成WAV无需显示器或声卡。Unix若丢失执行权限，使用chmod +x gbseq gbseq-cli。

## GUI编辑

选择连接模式及P1–P4。DMG07的ALL16可同时显示16通道。双击空白轨道创建音符，双击音符打开详细面板。拖动主体改变时间和音高，拖右边缘改变长度。每轨单音，拒绝重叠。小节数为1–256。滚轮和底部滚动条导航；F适应全曲，Home回开头，+/-缩放。播放时画面跟随位置。Space播放/停止，LOOP ON/OFF控制循环。首次播放先渲染整曲，长曲需要准备时间。

Ctrl/Shift点击切换选择，Ctrl+A选择当前轨道。左右按网格移动，上下半音，Shift+上下八度。Ctrl+D复制，Delete删除；G切换网格，Q对齐起点；Ctrl+Z/Y撤销/重做，Ctrl+C/V复制/粘贴。Ctrl+O打开，Ctrl+S保存，Ctrl+Shift+S另存为。替换未保存的编辑前会确认，上写前请保留原件。

## 音色与CH3波形

每音符可设置音量、左/右/中声像、脉冲占空比与包络、CH1扫频、CH3电平/预设及CH4噪声。AUDITION试听所选音符。CH3没有硬件包络。NEW USER WAVE编辑32个4位采样，拖动绘制0–15。可存64个用户波形，另有7个内置波形，分配到16个有效预设并选100/50/25%电平。每音符的WAVE PRESET使音色随时间变化。硬件改写波形会短暂关闭CH3，并非连续交叉淡化。

## CLI

track/player从1开始，start从0开始；length=1为十六分音符，16为4/4一小节。pitch 0–47对应C2–B5，velocity 0–15。players仅支持1、2、4。编辑必须给出--out，显式目标可以覆盖已有文件。不合法操作非零退出且不保存。transform还支持--shift、--quantize和--velocity。WAV默认混合所有启用的主机；--player N 只输出指定主机。导出可能替换现有输出。

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## LINK16与KITAQGB

LINK16 SAV导出V4格式128 KiB MBC5电池镜像：link16_local.sav；link16_cable.p1.sav/.p2.sav；或link16_dmg07.p1.sav至.p4.sav。使用匹配的LINK16 ROM版本及模拟器存档命名规则，替换前备份旧SAV。P1也保存其他玩家的镜像。SAV不能独立启动。每台仅有4个共享包络预设，多余取值可能被近似；加载后请试听。V4不保存CH1扫频。frames_per_step_override逐帧音效支持预览/WAV/KITAQGB，但BPM方式的LINK16导出会拒绝它。

KITAQGB按玩家导出C/音频流。使用物理通道ID CH1=0、CH2=1、CH3=2、CH4=3的兼容audio.h/audio.c。这不能把16通道合并到一台Game Boy。GBSEQ预览将步长量化为视频帧，因此部分BPM与分数时钟ROM的实际速度不同。模拟器渲染不等于真机测试。

## 数据与Python

.gbseq是JSON，不是MIDI；改扩展名不能转换格式。AI生成文件须validate，检查曲长、有效通道、唯一ID、音域和重叠。[详细规格](docs/README.md)为日文。可选Python 3.10+接口需要将python/加入PYTHONPATH，将GBSEQ_NATIVE_LIBRARY设为同包gbseq.dll/libgbseq.dylib/libgbseq.so的绝对路径。Python通道/玩家从0计数，直接改数据后须validate；player=None混合全部玩家。

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## 编译、排错与权利

使用Rust 1.93.1及系统C/C++工具。Linux编译需要libasound2-dev、libudev-dev、libxkbcommon-dev, libxkbcommon-x11-dev、libwayland-dev、libx11-dev、libxi-dev、libxcursor-dev及Mesa开发库。CI分别验证各OS的测试、音频/存档导出和GUI启动，并在verification中记录源提交。这些检查不测试实际声卡输出、物理输入或Game Boy真机。

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


启动失败检查GPU驱动和终端错误；缺字检查assets/fonts；无声检查设备和mute/solo；导出失败检查validate指出的音符。本体为MIT，Copyright (c) 2026 DAISUKE OBA。阅读[LICENSE](LICENSE)、[第三方声明](THIRD_PARTY_NOTICES.md)及[审核清单](LICENSE_AUDIT.json)，再分发时保留全部声明与字体许可证。第三方素材和导入音乐各自保留原有权利。
