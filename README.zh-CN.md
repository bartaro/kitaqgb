# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **简体中文**

**[编译器简体中文手册](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html)** · **[库简体中文手册](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html)**

## 名称由来

KITAQGB最初是NORCAL的一个分支。NORCAL是与Zachtronics相关的NES C编译器，本项目保留了原作者Keith Holman的著作权声明。

NORCAL的名称源于北加利福尼亚（Northern California）。受这种以地域命名的思路启发，作者以自己出生并长大的北九州市为基础，将项目命名为KITAQGB。KITAQ + GB把日本福冈县北九州市的昵称**北九（キタキュー，Kitakyū）**与Game Boy结合起来。KITAQ读作日语“キタキュー”；英语读音提示为 **kee-tah-KYOO**，音标为 **/ˌkiːtɑːˈkjuː/**。末尾的Q与英语字母Q同音。KITAQGB中的G和B分别按字母名称发音，整个名称读作 **kee-tah-KYOO jee bee**。

KITAQGB这个名称包含两层含义。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**表达了工具链应理解目标硬件、同时支持程序员和生成式AI的目标。

另一层含义是 **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**，意为“把孩子的想象，化为Game Boy森林里的真正冒险的工具”。这寄托了一份创作愿望：让小小的点子、涂鸦和借助AI制作的原型，变成真正可以游玩的冒险。

## 项目状态：公开预览

KITAQGB与KOKURA目前以公开预览版开发工具的形式提供。

它们可用于实验、示例项目、AI辅助游戏开发、编译器研究、模拟器调试和开发流程验证。项目仍在积极开发，API、CLI选项、输出格式、诊断和行为可能随版本变化。

预览版可能存在错误、未完成功能或不兼容的改动。用于正式项目或公开发行前，请仔细验证生成代码、模拟器行为、时序诊断及报告。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB是为Game Boy和Game Boy Color自制软件开发提供的开源C工具链。它面向便于诊断和AI辅助的开发过程：编写小型C程序，编译成 `.gb` 或 `.gbc` ROM，再利用模拟器的反馈改进游戏。

KITAQGB与Nintendo没有隶属关系，也未获得Nintendo的认可、赞助或批准。Game Boy和Game Boy Color是Nintendo的商标。

## KITAQGB提供什么

KITAQGB由C编译器和Game Boy自制软件支持库组成。它在NORCAL的基础上扩展了配套工具流程，以支持使用现代工具和AI辅助的游戏制作。

项目重点包括：

- 将C源码编译为Game Boy ROM；
- 开发Game Boy和Game Boy Color自制软件；
- 提供便于AI处理的诊断和可复现构建报告；
- 为LR35902类目标生成ROM、头部及底层代码；
- 提供调色板、图块、滚动、镜头、音频、通信和RPG/ADV/SLG支持库；
- 配合KOKURA CLI进行模拟器测试、执行跟踪与调试。

KITAQGB **不包含商业ROM、Nintendo BIOS、Nintendo SDK、专有资源或Nintendo官方开发资料**。

## 目标平台

KITAQGB可生成Game Boy兼容软件、Game Boy Color兼容软件，以及明确使用CGB专有功能的软件的自制ROM。常见输出扩展名如下：

```text
*.gb
*.gbc
```

生成的ROM应在模拟器上测试，并在可行时使用实机或合适的烧录卡验证。时序、中断、VRAM/OAM访问、音频、通信线和存储体切换尤其需要关注硬件细节。

## 仓库结构

编译器源码放在同名子目录 `kitaqgb/` 中。已构建的Release程序及运行时配置放在根目录；库和示例各有独立目录。

```text
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

运行所附编译器需要Windows和.NET Framework 4.8。请下载仓库ZIP，将程序、配置、库和许可声明一起保存。重新构建还需要.NET Framework 4.8 Developer Pack及Visual Studio Build Tools。请在仓库根目录运行：

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Release构建会将程序和配置复制到根目录。Debug构建保留在 `kitaqgb/bin/Debug`，不会覆盖已发布的Release编译器。发布内容不包含构建缓存和PDB文件。输入资料及SHA-256见[二进制构建记录](BINARY_BUILD.json)。

## 开发环境要求

主要构建环境为Windows、.NET Framework 4.8目标引用文件，以及带MSBuild的Visual Studio或Visual Studio Build Tools。项目使用面向 `.NET Framework v4.8` 的传统C#项目格式。

其他系统可能可以通过Mono/MSBuild构建，具体取决于所安装的引用程序集；主要支持的构建路径仍是Windows与MSBuild。

## 构建KITAQGB

从仓库根目录运行：

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

构建成功后，项目会将可执行文件复制到根目录：

```text
kitaqgb.exe
```

随后可以检查命令行帮助：

```powershell
.\kitaqgb.exe --help
```

## 快速开始

创建一个小型模板项目：

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

编译：

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

如需使用发布配置：

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

在您常用的Game Boy/Game Boy Color模拟器中运行ROM；需要配套的调试与观察功能时，也可以使用KOKURA CLI。

## 使用附带库

`lib/` 中是可复用的C支持代码。将需要的库源码与游戏源码一起编译，并加上 `-I lib` 以查找头文件。

以下示例组合了音频、调色板、滚动、镜头和物理功能。下面的多行命令使用 **cmd.exe**的续行符：

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

串行通信项目示例：

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

RPG/ADV/SLG功能示例：

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

库的分类和注意事项见 [lib/README.zh-CN.md](lib/README.zh-CN.md)。

## 常用命令行选项

```text
-o <file>                  输出ROM路径
-I <dir>                   头文件搜索目录
--profile=dev              开发配置
--profile=release          发布配置
--fast-build / --fast      开发时采用快速构建
--cache                    启用构建缓存
--no-cache                 禁用构建缓存
--disasm                   输出反汇编结果
--no-disasm                不输出反汇编结果
--diag-json <file>         将诊断写入JSON
--machine-readable         优先采用便于工具读取的输出
--deps-out <file>          输出依赖信息
--debug-output <dir>       调试及辅助输出目录
--strict                   将指定类型的警告视为错误
--permissive               放宽指定诊断条件
--stack-bank=fixed|wramx1  选择栈的存储体模型
--stack-top=<addr>         指定栈顶地址
--stack-reserve=<bytes>    预留栈空间
```

当前构建支持的完整选项请以帮助为准：

```powershell
.\kitaqgb.exe --help
```

## 与KOKURA CLI配合

KOKURA CLI是与KITAQGB配套的模拟器和调试器。典型过程如下：

1. 编写或生成C游戏代码。
2. 用KITAQGB编译。
3. 在KOKURA CLI中运行生成的ROM。
4. 记录诊断、跟踪、符号、时序观察和模拟器报告。
5. 将结果用于下一轮代码修改或调试。

在AI辅助开发中，编译器错误、模拟器报告或运行跟踪都可以转化成范围明确的修复任务。

## 开发理念

KITAQGB并不以通用现代C编译器为目标。它专门面向内存有限、使用存储体且对时序敏感的8位游戏平台。

项目重视可预测的生成代码、清晰的诊断、小而可复现的示例，以及便于人和AI阅读的构建与调试报告。在必要时提供底层控制，在适合的地方提供易用的上层库，让Game Boy开发更容易入门，同时不完全隐藏硬件。

<!-- development-prompt:zh-CN:start -->
## 游戏开发提示词

填写需求后，将完整提示词交给 AI。内容涵盖实现、模拟器测试、SARAKURA 分析以及修复后的复测。

[阅读 HTML 手册中的参考示例](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html#loop-prompts)

<details>
<summary>展开完整提示词</summary>

### 使用 KITAQGB、KOKURA 和 SARAKURA 开发游戏

填写需求后，将本文完整交给 AI。命令假定 `kitaqgb`、`kitaqfc`、`kokura`、`kurosaki`、`sarakura`、`kitaq-docs` 仓库与 `game-gb` 或 `game-fc` 项目位于同一父目录。请从该父目录运行，并按实际环境调整路径。

#### 需求

- 游戏名称：&lt;填写&gt;
- 类型与核心玩法：&lt;填写&gt;
- 操作方式及成功、失败条件：&lt;填写&gt;
- 必需的界面、关卡、敌人和道具：&lt;填写&gt;
- 画面风格、背景音乐和音效：&lt;填写，并注明所提供素材的路径&gt;
- 存档、通信、外设及其他要求：&lt;填写，或无&gt;
- 项目目录：&lt;填写&gt;
- 再分发要求：&lt;例如，自编代码和原创素材可按 MIT 许可公开&gt;

- 目标机型：&lt;初代 Game Boy / GB 与 CGB 双兼容 / 仅 CGB&gt;
- 性能目标：&lt;例如，正常游玩时每秒更新游戏逻辑 60 次；注明高负载场景的可接受表现&gt;

#### 请执行的任务

请使用 KITAQGB 及其库实现游戏。使用 KOKURA 运行和调试，使用 SARAKURA 整理诊断并比较修复前后的结果。

不断重复以下过程，直到满足验收标准：明确规格 → 实现一个小改动 → 构建 → 输入操作并观察 → 调查原因 → 修复 → 在相同条件下复测。不能以写出计划、提供代码或编译成功作为完成依据。

##### 确认环境和验收标准

1. 阅读工作目录的说明、各工具的 README、HTML 手册，以及所用库的头文件和实现。记录可执行文件路径及版本或 SHA-256；以实际 `--help` 输出核对命令，以源码核对 API。
2. 为输入、画面、声音、游戏进程和更新频率制定可判断的验收标准。例如，按下并松开 START 后开始游戏；碰撞扣除一条生命；暂停时指定声音静音，恢复后继续播放。
3. 只就重要歧义提问，常规、可撤销的实现决策请自主推进。不得擅自降低需求或验收标准。
4. 先用一个随附的小示例走通编译器、模拟器和 SARAKURA。它只能证明工具之间能衔接，不能代表所需游戏已经完成。

##### 先实现一个可玩的最小流程

- 使用 KITAQGB 的 C 方言和 `void main()`。不要假定桌面 C 或 GBDK API 可以直接使用。除声明外，还要把所需 `.c` 实现纳入构建；检查初始化顺序、单位、符号、范围、缓冲区生命周期和 ROM 分库。
- 规划 VRAM/OAM 更新、VBlank、中断、栈、ROM/WRAM 分库和图块、精灵数量限制。传输队列的总容量、剩余容量与物理 VRAM 的容量、空闲空间不是同一概念。
- DMG 游戏不得依赖 CGB 专用功能。双兼容游戏必须分别测试两种硬件模式。
- 字母、数字和符号使用所提供的原创 `ascii.c` 字体，并检查字符与图块的对应关系。

- 先连通启动、标题界面、可控制角色、成功或失败及重新开始，再扩充内容。
- 保留可编辑的图形、音乐、音效源文件及生成步骤，确认构建实际读取了导出数据。
- 源码注释用英文，进度报告用简体中文。SARAKURA 的标准报告保持英文。

##### 将每次构建与运行对应起来

使用 `out/iter-001` 等目录区分每轮输出。记录命令、退出码及源码、素材、工具、ROM、元数据的哈希。构建失败后，不得运行遗留的旧 ROM。映射文件、源码映射和调试信息必须与 ROM 来自同一次构建。

下面是基本的 DMG 检查示例。请准备 `main.c` 和所需库实现文件，并按游戏调整选项和输入序列。

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg` 选择初代 GB。测试 CGB 或双兼容时，需匹配 ROM 头部和模拟器机型设置。示例输入在两个松开区间之间按一次 START。运行 300 帧不能代表完成整个游戏的测试。

##### 检查画面、声音、状态和性能

- 保存输入场景，区分按下、按住和松开。覆盖规格中的全部路径：启动、开始、移动、动作、碰撞、滚动、关卡切换、游戏结束、重新开始、暂停，以及适用的存档和通信。
- 保存关键帧 PNG、输入、运行报告、诊断 JSONL、WAV 和必要的状态、内存观测。检查实际到达帧数与停止原因。真正打开图像查看；一张截图不能证明运动或输入响应。将计数器、坐标和状态变化与预期值对照，同时检查画面边缘、图块和属性边界及精灵密集场景。
- 检查音乐、音效、同时发声、断音、暂停和恢复。仅生成 WAV 不能证明声音正确。无法试听时，应区分已完成的波形、数值检查与尚未确认的听感。
- 测量高负载场景的目标 CPU 工作量、游戏更新及传输量，FC 还需计入 NMI 工作。宿主机上模拟器的运行速度不等于游戏更新频率，也不是实机速度证明。使用 `--allow-unimplemented` 后能继续运行，不代表未实现功能已受支持。

##### 分析、修复并复测

- 向 SARAKURA 提交被测 ROM 的构建元数据和该次运行的诊断 JSONL。CPU 跟踪或普通运行报告不能替代它。`--frames` 指定分析条件；SARAKURA 不执行 ROM，也不自动修改源码。
- 阅读 `report.html`、`ai_diagnostics.json`、`repair_prompt.md`、`retest_plan.json`，与复现步骤、画面、声音和源码核对。区分推测的源码位置、原因与已确认事实，并区分正常等待循环与卡死。逐项判断警告，记录未支持事件和分析范围限制。不要通过过滤警告或缩短测试来获得通过结果。
- 把问题缩减为最小复现，修复原因后重新构建。若根因在编译器或模拟器，应与游戏代码问题分离，并为工具修复补充回归验证。
- 复测时保持输入、随机种子、机型和视频制式、Mapper、观测帧和诊断设置一致。新 ROM 使用对应元数据；代码或 RAM 布局变化后，不得盲目复用即时存档。

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


诊断差异应与操作、画面和声音的验收结果结合使用。相同失败反复出现时，应重审证据与假设，不要无依据地继续修改。

##### 完成条件与交付内容

用交付源码和设置构建最终 ROM，再执行全部必需场景。仅使用无敌状态、自动测试输入或另一种 Mapper，不能证明最终版本的正常游玩。提供需求与测试对应表，说明剩余警告的原因，明确未验证、未支持项目。未做实机测试时标注“实机未验证”。

交付源码、工具和库的标识信息、可编辑素材、可复现的构建与测试脚本、ROM、最终验证证据，以及说明安装、操作和已知限制的 README。按需附上回放和测试驱动程序。仅在明确授权范围内发布或向外部发送文件。验证后删除不必要的中间构建和临时跟踪，但保留源码、素材、最终成果及必要的回归证据。

若环境或权限阻碍必需检查，应报告准确的复现步骤和所需操作，不得标记为完成。

</details>
<!-- development-prompt:zh-CN:end -->

## 商标与独立性

KITAQGB是独立的开源自制软件开发项目，与Nintendo没有隶属关系，也未获得Nintendo的认可、赞助或批准。Game Boy和Game Boy Color是Nintendo的商标。

若无必要权利，请勿向本仓库加入Nintendo标志、官方包装图、官方字体、BIOS、商业ROM数据或专有游戏资源。

## 许可证

KITAQGB按MIT许可证发布。其基础NORCAL的原始声明为：

```text
Copyright 2019 Keith Holman
```

KITAQGB的修改及新增部分声明为：

```text
Copyright (c) 2026 DAISUKE OBA
```

分发软件副本或实质性部分时，必须保留NORCAL的原始著作权声明和MIT许可证声明。详见 [LICENSE](LICENSE) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 参与开发

提交修改前，请遵守以下约定：

- 不加入受版权保护的ROM、BIOS、从商业游戏中提取的资源或官方SDK资料。
- 除非有明确的发行需要，不将 `bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` 等构建产物提交到源码中。
- 为编译器或代码生成错误优先提供小型、可复现的测试案例。
- 新增支持库时，记录构建命令和需要的硬件寄存器声明。
- 让诊断足够清晰，便于人和AI编程工具据此采取行动。

## 本版状态

本仓库为KITAQGB的首次公开发行而准备。随着项目发展，接口、支持库、诊断及配套工具集成仍可能调整。

## 构建与首次使用

在Windows中使用.NET Framework 4.8 Developer Pack及Visual Studio Build Tools的MSBuild。请在Developer PowerShell中运行：

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## 手册与许可证

- [简体中文编译器手册](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html) / [简体中文库手册](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html)
- [供离线阅读的手册源码](https://github.com/bartaro/kitaq-docs)
- [许可证](LICENSE) / [日文参考译文](LICENSE.ja)
