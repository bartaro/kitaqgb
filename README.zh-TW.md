# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **繁體中文**

**[編譯器繁體中文手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html)** · **[程式庫繁體中文手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)**

## 名稱由來

KITAQGB 以 NORCAL 為起點，分支開發而成。NORCAL 是與 Zachtronics 相關的 NES C 編譯器；本專案保留原作者 Keith Holman 的著作權聲明。

**KITAQGB** 這個名稱有兩層意思：

- **Kernel-Informed Toolchain for AI-Quality Game Boy Development**，表達工具鏈應掌握目標硬體特性，並支援人工撰寫與生成式 AI 輔助開發流程的理念。
- **KITAQ + GB**，將**日本福岡縣北九州市**的暱稱與 **Game Boy** 結合。**KITAQ** 代表北九州的暱稱 **北九（キタキュー，Kitakyū）**。

以英語標示發音時，**KITAQ** 讀作 **「kee-tah-KYOO」**，國際音標為 **/ˌkiːtɑːˈkjuː/**，接近日語的 **キタキュー**。最後的 **Q** 讀英文字母 Q 的名稱。**KITAQGB** 的讀法是 **「kee-tah-KYOO jee bee」**，G 與 B 分別依英文字母名稱發音。

## 專案狀態：公開預覽版

KITAQGB 與 KOKURA 目前以公開預覽版開發工具的形式提供。

可用於實驗、範例專案、AI 輔助遊戲開發、編譯器研究、模擬器除錯及開發流程驗證。專案仍持續開發，API、CLI 選項、輸出格式、診斷內容與行為都可能隨版本變動。

預覽版可能有錯誤、尚未完成的功能，或不相容的變更。用於正式專案或公開發行前，請仔細驗證產生的程式碼、模擬器行為、時序診斷與報告。

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB 是用於 Game Boy 與 Game Boy Color 自製軟體開發的開放原始碼 C 工具鏈。開發流程著重診斷能力與 AI 輔助：撰寫小型 C 程式，編譯成 `.gb` 或 `.gbc` ROM，再根據模擬器的回饋逐步改善遊戲。

KITAQGB 與 Nintendo 並無隸屬關係，也未獲 Nintendo 認可、贊助或核准。Game Boy 與 Game Boy Color 為 Nintendo 的商標。

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

## 目標平台

KITAQGB 可產生與 Game Boy 相容、與 Game Boy Color 相容，以及明確使用 CGB 專屬功能的自製 ROM。常見輸出副檔名如下：

```text
*.gb
*.gbc
```

產生的 ROM 應先在模擬器測試；條件允許時，也請使用實機或適合的燒錄卡驗證。時序、中斷、VRAM／OAM 存取、音訊、通訊線與記憶體區塊切換尤其需要留意硬體細節。

## 儲存庫結構

編譯器原始碼集中在同名子目錄 `kitaqgb/`。已建置的 Release 程式與執行階段設定檔位於根目錄，程式庫和範例則各自存放在獨立目錄。

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

隨附編譯器須在 Windows 與 .NET Framework 4.8 環境執行。請下載整個儲存庫的 ZIP，將程式、設定、程式庫與授權聲明保存在一起。自行建置還需要 .NET Framework 4.8 Developer Pack 與 Visual Studio Build Tools。請在儲存庫根目錄執行：

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Release 建置會將程式與設定檔複製到根目錄。Debug 版本保留在 `kitaqgb/bin/Debug`，不會覆寫發行用的 Release 編譯器。發行內容不包含建置快取或 PDB 檔。建置輸入與 SHA-256 請見[二進位檔建置紀錄](BINARY_BUILD.json)。

## 開發環境需求

主要建置環境為 Windows、.NET Framework 4.8 目標參考檔，以及包含 MSBuild 的 Visual Studio 或 Visual Studio Build Tools。專案採用傳統 C# 專案格式，目標為 `.NET Framework v4.8`。

其他作業系統可能可以透過 Mono／MSBuild 建置，實際情況取決於安裝的參考組件；主要支援的建置方式仍是 Windows 搭配 MSBuild。

## 建置 KITAQGB

在儲存庫根目錄執行：

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

建置成功後，專案會將執行檔複製到根目錄：

```text
kitaqgb.exe
```

接著可查看命令列說明：

```powershell
.\kitaqgb.exe --help
```

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

程式庫分類與注意事項請見 [lib/README.zh-TW.md](lib/README.zh-TW.md)。

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

## 開發理念

KITAQGB 並非以通用現代 C 編譯器為目標，而是專為記憶體有限、採用記憶體分區且對時序敏感的 8 位元遊戲平台設計。

專案重視可預測的生成程式碼、清楚的診斷、小型且可重現的範例，以及方便人與 AI 閱讀的建置和除錯報告。需要時保留底層控制，也在適當之處提供易用的高階程式庫，讓 Game Boy 開發更容易入門，同時仍能掌握硬體特性。

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

## 參與開發

提交變更前，請遵循以下原則：

- 不加入受著作權保護的 ROM、BIOS、從商業遊戲擷取的素材或官方 SDK 資料。
- 除非有明確的發行需求，否則不將 `bin/`、`obj/`、`target/`、`dist/`、`*.exe`、`*.dll`、`*.pdb` 等建置產物提交至原始碼中。
- 回報編譯器或程式碼生成錯誤時，優先提供小型、可重現的測試案例。
- 新增支援程式庫時，記錄建置指令與所需的硬體暫存器宣告。
- 讓診斷內容清楚具體，方便開發者與 AI 程式設計工具採取後續行動。

## 本版狀態

本儲存庫為 KITAQGB 首次公開發行而準備。隨著專案發展，介面、支援程式庫、診斷及周邊工具整合仍可能調整。

## 建置與首次使用

在 Windows 使用 .NET Framework 4.8 Developer Pack 與 Visual Studio Build Tools 提供的 MSBuild。請於 Developer PowerShell 執行：

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## 手冊與授權

- [繁體中文編譯器手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html)／[繁體中文程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)
- [可供離線閱讀的手冊原始檔](https://github.com/bartaro/kitaq-docs)
- [授權條款](LICENSE)／[日文參考譯文](LICENSE.ja)
