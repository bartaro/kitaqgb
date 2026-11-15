# KITAQGB

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | **繁體中文** | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

[編譯器手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html) · [程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)

## 目標平台

KITAQGB 可產生與 Game Boy 相容、與 Game Boy Color 相容，以及明確使用 CGB 專屬功能的自製 ROM。常見輸出副檔名如下：

```text
*.gb
*.gbc
```

產生的 ROM 應先在模擬器測試；條件允許時，也請使用實機或適合的燒錄卡驗證。時序、中斷、VRAM／OAM 存取、音訊、通訊線與記憶體區塊切換尤其需要留意硬體細節。

## 安裝

下載並解壓縮儲存庫 ZIP。執行隨附的編譯器與輔助工具不需要 Rust、.NET、Python 或 Pillow。

Windows 程式位於儲存庫根目錄；Linux 和 macOS 程式位於下表所列的 bin/ 平台資料夾。請保留 lib/ 和授權聲明。在 Linux/macOS 下載後執行 chmod +x，並將資料夾加入 PATH，或使用完整路徑執行。

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

## 快速開始

建立小型範本專案：

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

編譯程式：

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

若要使用發行設定：

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

請在慣用的 Game Boy／Game Boy Color 模擬器中執行 ROM。需要配套的除錯與觀察功能時，可使用 KOKURA CLI。

## 從原始碼建置

使用 Rust 1.85 或更新版本，可為 Windows、Linux、macOS ARM 和 macOS Intel 建置編譯器及所有輔助工具。原生程式執行不需要 .NET，素材處理工具也不需要 Python 或 Pillow。

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

[Rust 原生編譯器與輔助工具](tools/README.zh-TW.md)

## 常用命令列選項

```text
-o <file>                  輸出 ROM 的路徑
-I <dir>                   標頭檔搜尋目錄
--profile=dev              開發設定
--profile=release          發行設定
--fast-build / --fast      開發期間使用快速建置
--cache                    啟用建置快取
--no-cache                 停用建置快取
--disasm                   輸出反組譯結果
--no-disasm                不輸出反組譯結果
--diag-json <file>         將診斷資料寫入 JSON
--machine-readable         優先採用方便工具讀取的輸出
--deps-out <file>          輸出相依資訊
--debug-output <dir>       除錯及輔助輸出目錄
--strict                   將指定類型的警告視為錯誤
--permissive               放寬指定的診斷條件
--stack-bank=fixed|wramx1  選擇堆疊的記憶體區塊模型
--stack-top=<addr>         指定堆疊頂端位址
--stack-reserve=<bytes>    保留堆疊空間
```

目前版本支援的完整選項，請以執行檔的說明為準：

```powershell
.\kitaqgb.exe --help
```

## 搭配 KOKURA CLI

KOKURA CLI 是與 KITAQGB 配套的模擬器與除錯器。典型開發流程如下：

1. 撰寫或產生 C 遊戲程式碼。
2. 使用 KITAQGB 編譯。
3. 在 KOKURA CLI 中執行產生的 ROM。
4. 記錄診斷、追蹤、符號、時序觀察與模擬器報告。
5. 根據結果進行下一輪修改或除錯。

進行 AI 輔助開發時，可將編譯器錯誤、模擬器報告或執行追蹤整理成範圍明確的修正工作。

## 儲存庫結構

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

## KITAQGB 提供哪些功能

KITAQGB 包含 C 編譯器與 Game Boy 自製軟體支援程式庫。在 NORCAL 的基礎上，專案擴充了周邊工具流程，以支援運用現代工具與 AI 輔助的遊戲製作。

主要功能包括：

- 將 C 原始碼編譯為 Game Boy ROM；
- 支援 Game Boy 與 Game Boy Color 自製軟體開發；
- 提供便於 AI 處理的診斷資料與可重現的建置報告；
- 為 LR35902 類目標產生 ROM、標頭與底層程式碼；
- 提供調色盤、圖塊、捲動、鏡頭、音訊、通訊，以及 RPG／ADV／SLG 支援程式庫；
- 配合 KOKURA CLI 進行模擬器測試、執行追蹤與除錯。

KITAQGB **不包含商業 ROM、Nintendo BIOS、Nintendo SDK、專有素材或 Nintendo 官方開發資料**。

## 使用隨附程式庫

`lib/` 提供可重複使用的 C 支援程式碼。將所需的程式庫原始碼與遊戲原始碼一起編譯，並加上 `-I lib`，讓編譯器能找到標頭檔。

以下範例結合音訊、調色盤、捲動、鏡頭與物理功能。多行指令採用 **cmd.exe** 的續行符號：

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

序列通訊專案範例：

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

RPG／ADV／SLG 功能範例：

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

程式庫的分類與使用注意事項請參閱 [lib/README.md](lib/README.zh-TW.md)。

## 開發理念

KITAQGB 並非以通用現代 C 編譯器為目標，而是專為記憶體有限、採用記憶體分區且對時序敏感的 8 位元遊戲平台設計。

專案重視可預測的生成程式碼、清楚的診斷、小型且可重現的範例，以及方便人與 AI 閱讀的建置和除錯報告。需要時保留底層控制，也在適當之處提供易用的高階程式庫，讓 Game Boy 開發更容易入門，同時仍能掌握硬體特性。

## 遊戲開發提示詞

[手冊](https://bartaro.github.io/kitaq-docs/zh-TW/loop-engineering.html#gb)

## 名稱由來

KITAQGB 以 NORCAL 為起點，分支開發而成。NORCAL 是與 Zachtronics 相關的 NES C 編譯器；本專案保留原作者 Keith Holman 的著作權聲明。

NORCAL的名稱源自北加利福尼亞（Northern California）。受到這種以地名命名的方式啟發，作者以自己出生並成長的北九州市為基礎，將專案命名為KITAQGB。KITAQ + GB把日本福岡縣北九州市的暱稱**北九（キタキュー，Kitakyū）**與Game Boy結合起來。KITAQ讀作日語「キタキュー」；英語發音提示為 **kee-tah-KYOO**，音標為 **/ˌkiːtɑːˈkjuː/**。最後的Q與英語字母Q同音。KITAQGB中的G和B分別按照字母名稱發音，整個名稱讀作 **kee-tah-KYOO jee bee**。

KITAQGB這個名稱有兩層意義。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**表達了工具鏈應理解目標硬體，同時支援程式設計者與生成式AI的目標。

另一層意義是 **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**，意指「把孩子的想像，化為Game Boy森林裡真實冒險的工具」。這寄託了一份創作心願：讓小小的點子、塗鴉及借助AI製作的原型，成為真正可以遊玩的冒險。

## 參與開發

提交變更前，請遵循以下原則：

- 不加入受著作權保護的 ROM、BIOS、從商業遊戲擷取的素材或官方 SDK 資料。
- 除非有明確的發行需求，否則不將 `bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` 等建置產物提交至原始碼中。
- 回報編譯器或程式碼生成錯誤時，優先提供小型、可重現的測試案例。
- 新增支援程式庫時，記錄建置指令與所需的硬體暫存器宣告。
- 讓診斷內容清楚具體，方便開發者與 AI 程式設計工具採取後續行動。

## 專案狀態：公開預覽版

KITAQGB 與 KOKURA 目前以公開預覽版開發工具的形式提供。

可用於實驗、範例專案、AI 輔助遊戲開發、編譯器研究、模擬器除錯及開發流程驗證。專案仍持續開發，API、CLI 選項、輸出格式、診斷內容與行為都可能隨版本變動。

預覽版可能有錯誤、尚未完成的功能，或不相容的變更。用於正式專案或公開發行前，請仔細驗證產生的程式碼、模擬器行為、時序診斷與報告。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB 是用於 Game Boy 與 Game Boy Color 自製軟體開發的開放原始碼 C 工具鏈。開發流程著重診斷能力與 AI 輔助：撰寫小型 C 程式，編譯成 `.gb` 或 `.gbc` ROM，再根據模擬器的回饋逐步改善遊戲。

KITAQGB 與 Nintendo 並無隸屬關係，也未獲 Nintendo 認可、贊助或核准。Game Boy 與 Game Boy Color 為 Nintendo 的商標。

## 手冊與授權

- [編譯器手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html) · [程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)
- [可供離線閱讀的手冊原始檔](https://github.com/bartaro/kitaq-docs)
- [授權條款](LICENSE)／[日文參考譯文](LICENSE.ja)

## 商標與獨立性

KITAQGB 是獨立的開放原始碼自製軟體開發專案，與 Nintendo 並無隸屬關係，也未獲 Nintendo 認可、贊助或核准。Game Boy 與 Game Boy Color 為 Nintendo 的商標。

未具備所需權利時，請勿向本儲存庫加入 Nintendo 標誌、官方包裝圖像、官方字型、BIOS、商業 ROM 資料或專有遊戲素材。

## 授權條款

KITAQGB 採用 MIT 授權條款。作為基礎的 NORCAL 原始聲明如下：

```text
Copyright 2019 Keith Holman
```

KITAQGB 修改與新增部分的聲明如下：

```text
Copyright (c) 2026 DAISUKE OBA
```

散布軟體副本或其中實質部分時，須保留 NORCAL 原始著作權聲明與 MIT 授權聲明。詳見 [LICENSE](LICENSE) 與 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

[手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html)
