# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA将PNG转换为Game Boy图块、调色板和地图。作者：DAISUKE OBA。原创代码采用MIT许可证。

## 构建与运行

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## macOS可执行文件

Apple Silicon和Intel Mac的CLI版本：[下载与启动说明](MACOS.md)。
解压对应文件，在终端进入解压后的文件夹，运行`./gbhua --help`。
下方示例中的`./target/release/gbhua`请改为`./gbhua`。无需安装Rust或Python。
最低系统构建设置为macOS 11.0，运行测试使用macOS 15。未进行Apple Developer ID签名或公证。

## 操作流程

依次导入PNG、查看简短JSON报告、验证、预览、导出。成功时输出一个JSON对象。向AI提供文件路径和报告，不必粘贴完整项目，以减少令牌用量。

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

图片宽高必须为8的倍数（8..2040）。--size使用最近邻缩放。DMG使用4级明暗；CGB近似为RGB555，每图块4色、最多8个调色板。透明区域与白色合成。相同图块共享，超过256种图块会拒绝导入。请检查警告和预览。

完整项目使用.gbh保存。旧JSON项目只需将扩展名改为.gbh，无需转换内容；不提供旧扩展名加载器。GBTD/GBR/GBTB与GBMB/GBM用于交换，不保留所有项目字段。GBMB导出还会生成同名.gbr，请一起保存。

覆盖已有输出必须加--force。输入和输出路径必须不同。退出码：0成功，1验证失败，2参数/I/O/转换错误。不包含图像生成API或网络上传。依赖保留原许可证，请查看LICENSE、THIRD_PARTY_NOTICES.md和DEPENDENCIES.json。

## GUI / Python

本地GUI和Python绑定共享转换核心。独立CLI包不含GUI。在完整本地工作区运行gbhua_gui，Python使用import gbhua。GUI提供分组菜单、保存对话框、DMG/CGB对比、7种绘图尺寸、16字节输入、撤销/重做，以及地图缩放和滚动。

