# 原生辅助工具

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

所有辅助程序均以 Rust 实现，运行时不需要 .NET、Python 或 Pillow。使用 `cargo build --locked --release` 构建全部程序。Windows 命令名须加 `.exe`。

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 接受 1～65535 字节并保留 C# 编码器的输出。支持 `raw`、计数/值 `rle` 和九字节 `KQA1` 自动容器；大小相同依次选择 raw、RLE、ZX0。`--decompress` 对纯正向 ZX0 v2 流进行有界解码，不支持反向流及 v1。格式由 Einar Saukas 设计，KITAQ 实现采用 MIT 许可证。

VBlank 工具检查固定银行符号及 PUSH 模式后直接修改指定 ROM，并更新校验和。`--no-header-fix` 保留原校验和。模式搜索不验证整个中断程序；修改前请备份。

<!-- wire3d-feedback:start -->

Wire3D耗时、88行配置与独立时钟

DMG的WIRE3D_DMG_HEIGHT可选88、96、120；CGB的WIRE3DCGB_HEIGHT可选88、96。默认仍为DMG 120、CGB 96。88行视口为128×88，中心Y=44。库与调用代码须使用相同配置，以wire3d_dmg_88.c / wire3d_cgb_88.c替代普通入口编译。DMG 88沿用96行的模型结构和16条边限制。CGB 160×144模式保持不变。

[验证结果与示例](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html#wire3d-feedback-20261009)

Wire3D测量和回归验证脚本使用Python 3。模拟器检查需要KOKURA的Python桥接及C API DLL。PNG输出用Pillow为可选组件。

<!-- wire3d-feedback:end -->
