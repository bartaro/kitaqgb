# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | **简体中文** | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

由 **DAISUKE OBA** 创作的 Game Boy / Game Boy Color 分数挑战游戏。操纵白蛇吃石榴、增长身体，同时避开地雷和自己的身体。

- **游戏介绍与正式版 ROM 下载：** [itch.io](https://bartaro.itch.io/harapeko-shirohebi)
- **程序解说：** [简体中文 HTML](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-zh-CN.html) · [English HTML](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-en.html) · [日本語 HTML](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-ja.html)
- **许可证：** [MIT](LICENSE)，Copyright © 2026 DAISUKE OBA。适用于本目录中的游戏源码、随附原创图像、字体数据、音乐、音效和文档。再分发时请保留许可证声明。KITAQGB 及其依赖项仍适用[仓库许可证](../../LICENSE)中的声明，包括[原创 ASCII 字体声明](../../licenses/fonts/ASCII-font-MIT.txt)。

HTML 解说包含程序流程图、白蛇身体跟随算法、库的使用示例和源码文件导航。点击上面的链接，即可在浏览器中阅读 KITAQ Docs 上的指南。HTML 和样式表保存在 [kitaq-docs 仓库](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi)。

## 在 Windows 上构建

使用 Rust 1.85 或更高版本，可为 Windows、Linux、macOS ARM 和 macOS Intel 构建编译器及全部辅助工具。原生程序运行不需要 .NET，素材处理工具也不需要 Python 或 Pillow。 PowerShell 示例构建使用根目录的 Windows 程序。在 Linux/macOS 上使用原生编译器并传入相同的 C 输入和选项，或使用 PowerShell 7。CHR 图形与 C 源码是不同输入；font.chr 保留原示例字体。

在仓库根目录运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

也可以在游戏目录中运行：

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

默认输出为 `out/shirohebi.gb`，并生成 `out/shirohebi.map` 和 `out/build_manifest.json`。使用 `-OutputDirectory C:\build\shirohebi` 可指定其他输出目录。构建成功后会删除临时编译目录；指定 `-KeepBuildFiles` 或构建失败时则保留，便于排查。输出文件不纳入 Git 管理。

脚本将分割的游戏源码合并，与本仓库的库一起编译，确认中断处理代码位于固定 ROM 存储体，然后安装 VBlank 向量并更新卡带校验和。生成的 ROM 使用 **MBC5，容量 64 KiB，带有 8 KiB 电池备份 RAM**，可在 DMG 和 CGB 模式下运行。请勿单独编译各个源码片段，也不要省略向量和校验和处理。

本应用目录不附带预编译的可执行文件或 ROM。编译器位于仓库根目录，正式游戏 ROM 请从 itch.io 下载。

## 操作

| 画面 | 操作 |
| --- | --- |
| 标题 | START 开始。上／下／SELECT 选择 MUSIC 或 SOUND，左／右／A 切换所选项。 |
| 游戏中 | 左／右相对于蛇头朝向转弯；按住上加速；START 暂停。 |
| 暂停 | START 继续；SELECT 打开返回标题的确认画面。 |
| 确认画面 | 左／右／SELECT 选择，A 确定，B／START 取消。 |
| 姓名输入 | 上／下选择 A–Z 或句点；左／右／SELECT 移动光标；A 前进到下一字符，在第三个字符处结束；START 也可结束输入。 |
| 重试 | 左／右／SELECT 选择 YES／NO，A／START 确定。 |

在标题画面同时按 SELECT+START 可打开成绩清除确认。画面切换后，需要先松开所有按键，才能接受新的操作。内部状态转换和实现细节请参阅 HTML 解说。
