# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | **繁體中文** | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

這是由 **DAISUKE OBA** 製作的 Game Boy／Game Boy Color 挑戰高分遊戲。操縱白蛇吃石榴、讓身體變長，同時避開地雷與自己的身體。

- **遊戲介紹與官方 ROM 下載：** <https://bartaro.itch.io/harapeko-shirohebi>
- **程式解說：** [繁體中文 HTML](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-zh-TW.html)
- **授權：** [MIT](LICENSE)，著作權 © 2026 DAISUKE OBA。適用於本目錄中的遊戲原始碼、隨附的原創圖像、字形資料、音樂、音效與文件。重新散布時請保留授權聲明。KITAQGB 及其相依元件仍適用[儲存庫授權條款](../../LICENSE)中的聲明，包括[原始 ASCII 字型聲明](../../licenses/fonts/ASCII-font-MIT.txt)。

HTML 解說包含程式流程圖、白蛇身體的跟隨演算法、程式庫範例與原始碼導覽。透過上方連結即可在瀏覽器中直接閱讀 KITAQ Docs 頁面。HTML 與樣式表由 [kitaq-docs 儲存庫](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi)管理。

## 在 Windows 上建置

需要 Windows、.NET Framework 4.8、PowerShell，以及本儲存庫的完整簽出內容或 ZIP。請將根目錄的編譯器相關檔案與 `lib/` 保持在一起。圖像與音訊已以 C 陣列提供，不需要素材編輯器。

從儲存庫根目錄執行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

也可以從遊戲目錄執行：

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

預設輸出為 `out/shirohebi.gb`，並產生 `out/shirohebi.map` 與 `out/build_manifest.json`。可用 `-OutputDirectory C:\build\shirohebi` 指定其他位置。建置成功後會刪除暫存編譯目錄，除非指定 `-KeepBuildFiles`；建置失敗則保留以供診斷。輸出檔案已排除於 Git 追蹤之外。

指令碼會合併分割的遊戲原始碼、搭配儲存庫程式庫編譯、確認中斷函式位於固定 ROM，接著安裝 VBlank 向量並更新卡匣校驗和。產生的配置為 **64 KiB MBC5 ROM、8 KiB 電池備援 RAM**，可在 DMG 與 CGB 模式下遊玩。請勿單獨編譯分割檔案，也不要省略向量與校驗和處理。

本遊戲目錄**不含預先建置的執行檔或 ROM**。編譯器位於儲存庫根目錄；已發行的遊戲 ROM 請從 itch.io 下載。

## 操作方式

| 畫面 | 操作 |
|---|---|
| 標題 | START：開始。UP/DOWN/SELECT：選擇 MUSIC 或 SOUND。LEFT/RIGHT/A：切換所選選項的開關。 |
| 遊戲中 | LEFT/RIGHT：依白蛇目前朝向轉彎。按住 UP：加速。START：暫停。 |
| 暫停 | START：繼續。SELECT：開啟返回標題確認。 |
| 確認畫面 | LEFT/RIGHT/SELECT：選擇。A：確認。B/START：取消。 |
| 姓名輸入 | UP/DOWN：選擇 A–Z 或句點。LEFT/RIGHT/SELECT：移動游標。A：移到下一字元，在第三字元時完成。START：完成。 |
| 重試 | LEFT/RIGHT/SELECT：選擇 YES/NO。A/START：確認。 |

在標題畫面按 SELECT+START 可開啟成績刪除確認。切換畫面後，必須先放開所有按鍵才會接受新操作。狀態轉換與實作細節請見 HTML 解說。
