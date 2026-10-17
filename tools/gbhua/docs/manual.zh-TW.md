# GBHUA

## 下載與啟動

在[GUI下載](../GUI.md)選擇對應系統的壓縮檔並完整解壓縮。Windows開啟`GBHUA.exe`，保留旁邊的`assets`資料夾。Mac將`GBHUA.app`複製到「應用程式」，請區分Apple Silicon與Intel版本。執行GUI不需Rust或Python。程式尚未使用Developer ID簽署或公證。請從官方儲存庫下載；若macOS阻擋，確認來源後在「系統設定 > 隱私權與安全性」中僅允許此程式，不要關閉系統保護。

GUI標籤為英文，本頁為中文操作手冊。Mac的`Ctrl`也指Control鍵。下方建置命令從儲存庫根目錄開始。CLI預設仍不需要視窗或GPU。

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA將PNG轉換為Game Boy圖塊、調色盤與地圖。作者：DAISUKE OBA。原創程式碼採用MIT授權。

## 建置與執行

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

啟動後尚未編輯的文件不會顯示未儲存警告，實際編輯後仍會確認。Windows GUI預設不開啟主控台。需要診斷時執行 `cargo run -p gbhua --bin gbhua_gui --features gui-console`。

各欄標題和畫布頂端已對齊。將滑鼠停在開啟、儲存、復原/重做圖示上可查看名稱。用Graphic上方的尺寸按鈕選擇大小，用Map上方的符合視窗、減號、加號調整縮放。右上角可切換DMG/CGB。寬度小於1180px時，左欄使用Tiles / Palettes分頁切換。用滾輪或捲軸瀏覽調色盤和診斷資訊。Escape關閉選單。

1. 在左側選擇圖塊，在Graphic > 8x8選擇7種尺寸之一。大圖從選定ID起使用連續圖塊。選色並拖曳繪製，右鍵取色。

2. TOOLS > FILL填滿連通區域；TILES > FILL TILE填滿整幅圖。FLIP X/Y、ROTATE用於變換，EDIT的COPY/PASTE GRAPHIC用於複製。UNDO/REDO最多64步。

3. TOOLS > MAP放置圖塊，右鍵取得圖塊及調色盤。RECT放置3x3，FILL填滿區域，ATTR設定調色盤並切換X翻轉，COLL/EVT切換標記。VIEW選擇Fit/1x/2x/4x、格線及標記，以滾輪或捲軸移動。

4. 點選BYTECODE，Ctrl+A後Ctrl+V貼上16位元組。IMPORT套用至第一個8x8圖塊，EXPORT更新文字。方向鍵/Home/End移動游標，Shift選取，Ctrl+C/X複製/剪下，Backspace/Delete刪除相鄰字元。

5. FILE > SAVE及EXPORT都會詢問儲存位置。未儲存*時，開啟/匯入/關閉會確認：Yes儲存、No捨棄、Cancel繼續編輯。PNG匯入取代專案，可以復原。

6. SPRITES > NEW META建立複合精靈，以ADD/DUP/CLEAR FRAME和PREV/NEXT FRAME管理影格。METASPRITE中點選新增部件、右鍵移除。DMG / CGB比較DMG/CGB，不改變已儲存調色盤。

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

完整專案以.gbh儲存。舊JSON專案只需將副檔名改為.gbh，內容不必轉換；不提供舊副檔名載入器。GBTD/GBR/GBTB與GBMB/GBM是交換格式，不保留全部欄位。GBMB匯出也會產生同名.gbr，請一起保管。

## PNG / Python / CLI

圖片寬高必須為8的倍數（8..2040）。--size使用最近鄰縮放。DMG使用4階明暗；CGB近似為RGB555，每圖塊4色、最多8組調色盤。透明區域與白色合成。相同圖塊共用，超過256種圖塊時拒絕匯入。請檢查警告及預覽。

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: 繁體中文](../README.zh-TW.md)

## PNG匯入與疑難排解

使用`FILE > IMPORT PNG`或拖入PNG，先選擇DMG/CGB。匯入會取代目前專案，請先儲存。CGB自動調色盤會評估色彩誤差，保留少量高光與深色輪廓，而非只保留常見暗色。Game Boy色數限制仍存在。舊匯入遺失的光澤必須重新匯入原PNG；減色後的`.gbh`無法還原遺失的像素。`--size`僅為CLI選項。

選取圖塊，點擊BYTECODE內部，輸入恰好16位元組再按IMPORT。EXPORT僅更新文字。地圖顏色或方向異常時檢查調色盤與翻轉屬性。使用`.gbh`保留完整編輯資料。錯誤顯示於狀態列；取消對話框不會儲存。

macOS/Linux依序執行`python3 -m venv .venv`、`source .venv/bin/activate`、`python -m pip install maturin`、`python -m maturin develop --features python`。建置綁定需要Python 3.10以上及Rust。
