# ZX0 相容素材壓縮

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | **繁體中文** | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

[API 與範例](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html#module-zx0)

PC 壓縮器與 GB 解壓縮器是 KITAQ 獨立撰寫的實作，支援 ZX0 v2 順向串流。KITAQ 實作採用 MIT 授權條款，著作權為 Copyright (c) 2026 DAISUKE OBA。

ZX0 格式與原始壓縮演算法由 [Einar Saukas](https://github.com/einar-saukas/ZX0) 設計。這項格式致謝與 KITAQ 實作的著作權、授權分別表述；另請參閱 [LICENSE](../../LICENSE) 與 [LICENSE.ja](../../LICENSE.ja)。

從儲存庫根目錄建置 PC 工具：

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

獨立工具使用 .NET Framework 4.x，接受 1～65535 位元組的壓縮輸入。壓縮採用限制搜尋次數的雜湊鏈，不保證得到最小壓縮大小。輸出為一般 ZX0 v2 資料，不附加 KITAQ 封裝。此介面不支援逆向串流、前綴字典或 ZX0 v1。原始素材的權利仍屬於各自作者。

編碼後的輸出必須符合目標 API 的 65535 位元組大小參數上限。自動選擇容器的九位元組標頭也計入此上限。大型素材須分割，且仍須遵守目標硬體更小的 RAM 容量與 bank 視窗限制。空的 raw C 標頭會保留一個佔位位元組，但邏輯 `_SIZE` 為零；不支援空的純 ZX0 串流。

若要比較未壓縮資料、個數／值 RLE 與 ZX0，並選取最小酬載，請執行：

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

自動模式會附加九位元組的 KQA1 標頭，並回報選用的編碼方式。它比較酬載大小；同大小時依 raw、RLE、ZX0 的順序優先選取。這是 KITAQ 素材容器，並非純 ZX0 串流。標頭依序為 `KQA1`、一位元組編碼值（0 raw、1 RLE、2 ZX0）、小端序 u16 原始大小、小端序 u16 酬載大小，接著立即存放酬載。此容器請用 `asset_decompress` 展開。`--format=raw` 與 `--format=rle` 僅輸出所選酬載；RLE 以個數零作為結束標記。

目標程式須引入 `zx0.h`，並編譯 `lib/zx0.c`。`zx0_decompress` 接受輸出位址、輸出容量、壓縮來源與壓縮大小。請同時檢查回傳位元組數與 `zx0_error`。發生錯誤時可能已寫入部分輸出，因此失敗後不要顯示或使用結果。來源與目的緩衝區不可重疊，也不可跨越目前映射的 CPU bank 視窗邊界。函式使用共用工作區，不可從中斷重入。

緩衝區也不可超出 CPU 位址範圍而繞回起點。

`zx0_decompress_vram` 僅在 LCD 關閉時寫入目前選取的 GB VRAM bank，並保留顯示、bank 與中斷設定。請在場景初始化時上傳，不要假設整份素材能在一次 VBlank 內處理完畢。
