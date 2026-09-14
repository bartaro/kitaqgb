# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **繁體中文**

**[編譯器繁體中文手冊](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html)** · **[程式庫繁體中文手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)**

## 名稱由來

KITAQGB 以 NORCAL 為起點，分支開發而成。NORCAL 是與 Zachtronics 相關的 NES C 編譯器；本專案保留原作者 Keith Holman 的著作權聲明。

NORCAL的名稱源自北加利福尼亞（Northern California）。受到這種以地名命名的方式啟發，作者以自己出生並成長的北九州市為基礎，將專案命名為KITAQGB。KITAQ + GB把日本福岡縣北九州市的暱稱**北九（キタキュー，Kitakyū）**與Game Boy結合起來。KITAQ讀作日語「キタキュー」；英語發音提示為 **kee-tah-KYOO**，音標為 **/ˌkiːtɑːˈkjuː/**。最後的Q與英語字母Q同音。KITAQGB中的G和B分別按照字母名稱發音，整個名稱讀作 **kee-tah-KYOO jee bee**。

KITAQGB這個名稱有兩層意義。**Kernel-Informed Toolchain for AI-Quality Game Boy Development**表達了工具鏈應理解目標硬體，同時支援程式設計者與生成式AI的目標。

另一層意義是 **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**，意指「把孩子的想像，化為Game Boy森林裡真實冒險的工具」。這寄託了一份創作心願：讓小小的點子、塗鴉及借助AI製作的原型，成為真正可以遊玩的冒險。

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

<!-- development-prompt:zh-TW:start -->
## 遊戲開發提示詞

填寫需求後，將完整提示詞交給 AI。內容涵蓋實作、模擬器測試、SARAKURA 分析，以及修正後的重新驗證。

[閱讀 HTML 手冊中的參考範例](https://bartaro.github.io/kitaq-docs/zh-TW/kitaqgb.html#loop-prompts)

<details>
<summary>展開完整提示詞</summary>

### 使用 KITAQGB、KOKURA 與 SARAKURA 開發遊戲

填寫需求後，將本文完整交給 AI。命令假設 `kitaqgb`、`kitaqfc`、`kokura`、`kurosaki`、`sarakura`、`kitaq-docs` 儲存庫與 `game-gb` 或 `game-fc` 專案位於同一個上層目錄。請從該目錄執行，並依實際環境調整路徑。

#### 需求

- 遊戲名稱：&lt;填寫&gt;
- 類型與核心玩法：&lt;填寫&gt;
- 操作方式及成功、失敗條件：&lt;填寫&gt;
- 必備畫面、關卡、敵人與道具：&lt;填寫&gt;
- 美術風格、背景音樂與音效：&lt;填寫，並註明提供素材的路徑&gt;
- 存檔、通訊、周邊設備及其他需求：&lt;填寫，或無&gt;
- 專案目錄：&lt;填寫&gt;
- 再散布條件：&lt;例如，自行撰寫的程式與原創素材可採 MIT 授權公開&gt;

- 目標機型：&lt;初代 Game Boy / 同時支援 GB 與 CGB / CGB 專用&gt;
- 效能目標：&lt;例如，一般遊玩時每秒更新遊戲邏輯 60 次；註明高負載場景可接受的表現&gt;

#### 請執行的工作

請以 KITAQGB 及其函式庫實作遊戲。使用 KOKURA 執行與除錯，使用 SARAKURA 整理診斷並比較修正前後的結果。

持續重複以下流程，直到符合驗收標準：具體化規格 → 實作小幅變更 → 建置 → 輸入操作並觀察 → 調查原因 → 修正 → 在相同條件下重新驗證。不可只因提出計畫、提供程式碼或編譯成功，就認定工作完成。

##### 確認環境與驗收標準

1. 閱讀工作目錄的指示、各工具 README、HTML 手冊，以及所用函式庫的標頭檔和實作。記錄執行檔路徑及版本或 SHA-256；以實際 `--help` 輸出確認命令，以原始碼確認 API。
2. 為輸入、畫面、聲音、遊戲流程和更新頻率訂出可判定的驗收標準。例如，按下再放開 START 後開始遊戲；碰撞減少一條命；暫停時指定聲音靜音，繼續後恢復播放。
3. 只針對重要的模糊需求提問，一般可復原的實作決策請自主處理。不得自行降低需求或驗收標準。
4. 先用隨附的小範例串接編譯器、模擬器與 SARAKURA。這只能確認工具銜接，不能代表所需遊戲已完成。

##### 先完成可玩的最小流程

- 使用 KITAQGB 的 C 方言與 `void main()`。不要假定桌面 C 或 GBDK API 能直接使用。除了宣告，也要把必要的 `.c` 實作納入建置；確認初始化順序、單位、正負號、範圍、緩衝區生命週期及 ROM 分頁。
- 規劃 VRAM/OAM 更新、VBlank、中斷、堆疊、ROM/WRAM 分頁，以及圖塊與精靈數量限制。傳輸佇列的總容量、剩餘容量，與實體 VRAM 的容量、可用空間是不同概念。
- DMG 遊戲不得依賴 CGB 專用功能。若支援兩者，請分別驗證各硬體模式。
- 英文字母、數字與符號使用提供的原創 `ascii.c` 字型，並確認字元與圖塊的對應關係。

- 先串起開機、標題畫面、可操控角色、成功或失敗與重新開始，再擴充內容。
- 保留可編輯的圖形、音樂、音效原始檔及產生步驟，並確認建置確實讀取匯出的資料。
- 原始碼註解使用英文，進度報告使用繁體中文。SARAKURA 的標準報告維持英文。

##### 對應每次建置與執行結果

以 `out/iter-001` 等目錄區分每輪輸出。記錄命令、結束碼，以及原始碼、素材、工具、ROM、中繼資料的雜湊值。建置失敗後，不可執行殘留的舊 ROM。配置映射、原始碼映射與除錯資訊必須和 ROM 來自同一次建置。

以下是基本的 DMG 檢查範例。請準備 `main.c` 和必要的函式庫實作檔，並依遊戲調整選項及輸入序列。

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


`--hardware dmg` 選用初代 GB。測試 CGB 或雙機型支援時，應讓 ROM 標頭與模擬器機型設定一致。範例輸入在兩段放開按鈕的時間之間按一次 START。執行 300 影格不代表測試完整個遊戲。

##### 檢查畫面、聲音、狀態與效能

- 保存輸入情境，區分按下、按住與放開。走過規格中的全部路徑：開機、開始、移動、動作、碰撞、捲動、關卡切換、遊戲結束、重新開始、暫停，以及適用的存檔與通訊。
- 保留關鍵影格 PNG、輸入資料、執行報告、診斷 JSONL、WAV 和必要的狀態、記憶體觀測。確認實際到達的影格數與停止原因。務必開啟圖片查看；一張截圖無法證明移動或輸入反應。將計數器、座標與狀態變化和預期值比對，也要檢查畫面邊緣、圖塊與屬性邊界及精靈密集情境。
- 檢查音樂、音效、同時發聲、斷音、暫停與恢復。僅產生 WAV 不能證明聲音正確。無法試聽時，請區分已執行的波形、數值檢查與尚未確認的聽感。
- 測量高負載場景的目標 CPU 工作量、遊戲更新與傳輸量；FC 還要計入 NMI 工作。主機上模擬器的執行速度不等於遊戲更新頻率，也不能證明實機速度。使用 `--allow-unimplemented` 後能繼續執行，不代表未實作功能已受支援。

##### 分析、修正並重新驗證

- 將待測 ROM 的建置中繼資料與該次執行的診斷 JSONL 交給 SARAKURA。CPU 追蹤或一般執行報告不能取代它。`--frames` 指定分析條件；SARAKURA 不會執行 ROM，也不會自動修改原始碼。
- 閱讀 `report.html`、`ai_diagnostics.json`、`repair_prompt.md`、`retest_plan.json`，並與重現步驟、畫面、聲音及原始碼核對。區分推測的位置、原因與已確認事實，也要區分正常等待迴圈與當機。逐項判讀警告，記錄未支援事件與分析限制。不可透過隱藏警告或縮短測試來取得通過結果。
- 將問題縮減成最小重現案例，修正原因後重新建置。若根源在編譯器或模擬器，應與遊戲程式問題分開確認，並為工具修正加入回歸驗證。
- 重新驗證時，保持輸入、亂數種子、機型與影像制式、Mapper、觀測影格及診斷設定一致。新 ROM 使用相符的中繼資料；程式或 RAM 配置變更後，不可直接沿用即時存檔。

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


診斷差異應搭配操作、畫面與聲音的驗收結果判斷。相同失敗反覆發生時，請重新檢視證據與假設，不要漫無根據地繼續修改。

##### 完成條件與交付內容

以交付的原始碼與設定建置最終 ROM，再執行所有必要情境。僅使用無敵狀態、自動測試輸入或另一種 Mapper，無法驗證最終版本的正常遊玩。提供需求與測試對照表，說明剩餘警告的原因，明列未驗證、未支援項目。未做實機測試時，請標註「實機未驗證」。

交付原始碼、工具與函式庫識別資訊、可編輯素材、可重現的建置與測試指令稿、ROM、最終驗證證據，以及說明安裝、操作與已知限制的 README。視需要附上重播資料和測試驅動程式。只在明確授權範圍內公開或對外傳送檔案。驗證後刪除不必要的中間建置與暫存追蹤，但保留原始碼、素材、最終成果及必要的回歸證據。

若環境或權限阻礙必要檢查，請回報確切的重現步驟與所需處理，不得標記為完成。

</details>
<!-- development-prompt:zh-TW:end -->

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
