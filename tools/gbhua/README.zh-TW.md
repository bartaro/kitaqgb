# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA將PNG轉換為Game Boy圖塊、調色盤與地圖。作者：DAISUKE OBA。原創程式碼採用MIT授權。

## 建置與執行

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## macOS執行檔

Apple Silicon和Intel Mac的CLI版本：[下載與啟動說明](MACOS.md)。
解壓縮對應檔案，在終端機進入解壓縮後的資料夾，執行`./gbhua --help`。
下方範例中的`./target/release/gbhua`請改為`./gbhua`。無須安裝Rust或Python。
最低系統建置設定為macOS 11.0，執行測試使用macOS 15。未進行Apple Developer ID簽署或公證。

## 操作流程

依序匯入PNG、查看精簡JSON報告、驗證、預覽、匯出。成功時輸出一個JSON物件。向AI提供檔案路徑與報告，不必貼上完整專案，可減少權杖用量。

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

圖片寬高必須為8的倍數（8..2040）。--size使用最近鄰縮放。DMG使用4階明暗；CGB近似為RGB555，每圖塊4色、最多8組調色盤。透明區域與白色合成。相同圖塊共用，超過256種圖塊時拒絕匯入。請檢查警告及預覽。

完整專案以.gbh儲存。舊JSON專案只需將副檔名改為.gbh，內容不必轉換；不提供舊副檔名載入器。GBTD/GBR/GBTB與GBMB/GBM是交換格式，不保留全部欄位。GBMB匯出也會產生同名.gbr，請一起保管。

覆寫既有輸出需加--force。輸入與輸出路徑必須不同。結束碼：0成功，1驗證失敗，2參數/I/O/轉換錯誤。不含影像生成API或網路上傳。相依套件保留原授權，請參閱LICENSE、THIRD_PARTY_NOTICES.md及DEPENDENCIES.json。

## GUI / Python

本機GUI與Python綁定共用轉換核心。獨立CLI套件不含GUI。在完整本機工作區執行gbhua_gui，Python使用import gbhua。GUI提供群組選單、儲存對話框、DMG/CGB比較、7種繪圖尺寸、16位元組輸入、復原/重做，以及地圖縮放和捲動。

