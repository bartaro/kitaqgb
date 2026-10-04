# 原生輔助工具

[en](../README.en.md) · [ja](../README.ja.md) · [ko](../README.ko.md) · [zh-CN](../README.zh-CN.md) · [zh-TW](../README.zh-TW.md) · [fr](../README.fr.md) · [es](../README.es.md) · [de](../README.de.md)

所有輔助程式均以 Rust 實作，執行時不需要 .NET、Python 或 Pillow。使用 `cargo build --locked --release` 建置全部程式。Windows 指令名稱須加 `.exe`。

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 接受 1～65535 位元組並保留 C# 編碼器的輸出。支援 `raw`、個數/值 `rle` 與九位元組 `KQA1` 自動容器；大小相同依序選擇 raw、RLE、ZX0。`--decompress` 對純順向 ZX0 v2 串流進行有界解碼，不支援逆向串流及 v1。格式由 Einar Saukas 設計，KITAQ 實作採用 MIT 授權。

VBlank 工具檢查固定 bank 符號及 PUSH 模式後直接修改指定 ROM，並更新檢查碼。`--no-header-fix` 保留原檢查碼。模式搜尋不驗證整個中斷程式；修改前請備份。

```sh
sh tools/zx0/build.sh
```
