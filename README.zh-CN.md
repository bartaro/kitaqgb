# KITAQGB

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | **简体中文** | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

[KITAQGB 简体中文手册](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html) · [KITAQGB 库 简体中文手册](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html)

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

## 安装

下载并解压仓库 ZIP。运行附带的编译器和辅助工具不需要 Rust、.NET、Python 或 Pillow。

Windows 程序位于仓库根目录；Linux 和 macOS 程序位于下表所列的 bin/ 平台目录。请保留 lib/ 和许可声明。在 Linux/macOS 上下载后执行 chmod +x，并将目录加入 PATH，或使用完整路径运行。

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

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

库的分类和注意事项见 [库说明](lib/README.zh-CN.md)。

### 从源码构建

使用 Rust 1.85 或更高版本，可为 Windows、Linux、macOS ARM 和 macOS Intel 构建编译器及全部辅助工具。原生程序运行不需要 .NET，素材处理工具也不需要 Python 或 Pillow。

```sh
cargo build --locked --release
```

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

[Rust 原生编译器与辅助工具](tools/README.zh-CN.md)

## 3D 绘制

Wire3D耗时、88行配置与独立时钟

DMG的WIRE3D_DMG_HEIGHT可选88、96、120；CGB的WIRE3DCGB_HEIGHT可选88、96。默认仍为DMG 120、CGB 96。88行视口为128×88，中心Y=44。库与调用代码须使用相同配置，以wire3d_dmg_88.c / wire3d_cgb_88.c替代普通入口编译。DMG 88沿用96行的模型结构和16条边限制。CGB 160×144模式保持不变。

[验证结果与示例](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html#wire3d-feedback-20261009)

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

## 仓库结构

```text
Cargo.toml / Cargo.lock
src/                     # Rust compiler and native helper sources
kitaqgb.exe             # Windows x64 compiler
kitaqgb-*.exe           # Windows native helper tools
bin/                     # Linux and macOS executables
lib/                     # C libraries for console ROMs
tests/                   # Frozen reference fixtures and Rust tests
scripts/build.ps1
scripts/build.sh
```

## 开发理念

KITAQGB并不以通用现代C编译器为目标。它专门面向内存有限、使用存储体且对时序敏感的8位游戏平台。

项目重视可预测的生成代码、清晰的诊断、小而可复现的示例，以及便于人和AI阅读的构建与调试报告。在必要时提供底层控制，在适合的地方提供易用的上层库，让Game Boy开发更容易入门，同时不完全隐藏硬件。

## 游戏开发提示词

[手册](https://bartaro.github.io/kitaq-docs/zh-CN/loop-engineering.html#gb)

## 名称由来

KITAQGB最初是NORCAL的一个分支。NORCAL是与Zachtronics相关的NES C编译器，本项目保留了原作者Keith Holman的著作权声明。

NORCAL的名称源于北加利福尼亚（Northern California）。受这种以地域命名的思路启发，作者以自己出生并长大的北九州市为基础，将项目命名为KITAQGB。KITAQ + GB把日本福冈县北九州市的昵称**北九（キタキュー，Kitakyū）**与Game Boy结合起来。KITAQ读作日语“キタキュー”；英语读音提示为 **kee-tah-KYOO**，音标为 **/ˌkiːtɑːˈkjuː/**。末尾的Q与英语字母Q同音。KITAQGB中的G和B分别按字母名称发音，整个名称读作 **kee-tah-KYOO jee bee**。

KITAQGB这个名称包含两层含义。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**表达了工具链应理解目标硬件、同时支持程序员和生成式AI的目标。

另一层含义是 **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**，意为“把孩子的想象，化为Game Boy森林里的真正冒险的工具”。这寄托了一份创作愿望：让小小的点子、涂鸦和借助AI制作的原型，变成真正可以游玩的冒险。

## 参与开发

提交修改前，请遵守以下约定：

- 不加入受版权保护的ROM、BIOS、从商业游戏中提取的资源或官方SDK资料。
- 除非有明确的发行需要，不将 `bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` 等构建产物提交到源码中。
- 为编译器或代码生成错误优先提供小型、可复现的测试案例。
- 新增支持库时，记录构建命令和需要的硬件寄存器声明。
- 让诊断足够清晰，便于人和AI编程工具据此采取行动。

## 项目状态：公开预览

KITAQGB与KOKURA目前以公开预览版开发工具的形式提供。

它们可用于实验、示例项目、AI辅助游戏开发、编译器研究、模拟器调试和开发流程验证。项目仍在积极开发，API、CLI选项、输出格式、诊断和行为可能随版本变化。

预览版可能存在错误、未完成功能或不兼容的改动。用于正式项目或公开发行前，请仔细验证生成代码、模拟器行为、时序诊断及报告。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB是为Game Boy和Game Boy Color自制软件开发提供的开源C工具链。它面向便于诊断和AI辅助的开发过程：编写小型C程序，编译成 `.gb` 或 `.gbc` ROM，再利用模拟器的反馈改进游戏。

KITAQGB与Nintendo没有隶属关系，也未获得Nintendo的认可、赞助或批准。Game Boy和Game Boy Color是Nintendo的商标。

## 手册与许可证

- [简体中文编译器手册](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html) / [简体中文库手册](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html)
- [供离线阅读的手册源码](https://github.com/bartaro/kitaq-docs)
- [许可证](LICENSE) / [日文参考译文](LICENSE.ja)

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

[手册](https://bartaro.github.io/kitaq-docs/zh-CN/kitaqgb.html)
