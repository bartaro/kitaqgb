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
