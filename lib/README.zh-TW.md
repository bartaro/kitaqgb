# KITAQGB 程式庫



<!-- local-library-20261003:start -->
## 2026-10-03 本機程式庫差異與驗證

已將 C:/kitaqgb_project/lib 的原始碼與公開版 2de7f4b 比較。僅憑時間戳記無法確定哪個版本較新。本機版有新增功能，公開版也有其他新增功能與修正。以下說明針對保存的本機原始碼，與原有公開 API 的驗證紀錄分開。

physics2d.c 保留接觸 API，並透過 KQ2D_CACHE_SURFACE 提供快取接觸與速度限制實作。啟用時須預留 ROM bank 13。定義 KQ2D_EXTERNAL_SCALE_Q8、KQ2D_EXTERNAL_REST_COMPONENT 或 KQ2D_EXTERNAL_SURFACE 時，呼叫端須提供對應實作。下列 8,872 項測試使用未定義這些選項的預設路徑。

KQ3D_BODY_SPHERE=0x04 選擇球與盒的接觸處理。球半徑為 half_x。kq3d_overlap_sphere_aabb(sphere, box) 包含相切邊界；指標為 NULL 或半徑不大於 0 時回傳 0。沒有專用的球與球求解器。座標差、乘積與平方和須保持在各自的 16 位元範圍內。

帶有 KQ3D_BODY_KINEMATIC=0x02 的活動 body，即使逆質量為 0，也會在 kq3d_step 中更新 X/Y 速度和位置。Z 位置和速度不變。逆質量為 0 的 body 不會被接觸推移。kq3d_integrate_body 仍跳過靜態 body。break_speed 和 KQ3D_BODY_BROKEN 不會自動觸發破壞。

本機 bank_switch 寫入 kq_bank_current 與位址 0x2000。它與公開版的 __bankswitch 路徑不同，不更新編譯器的 bank 追蹤狀態。應從固定 bank 切換；需要維護編譯器還原狀態時使用 __bankswitch。

本機 cgb_bg_color/cgb_obj_color 及單色 RGB 包裝函式未啟用調色盤索引自動遞增。CGB 執行確認 0x1234 的低位元組被 0x12 覆寫。請使用公開版修正或 colors 陣列 API。DMG 不執行寫入。

本機 metasprite_draw 僅在全部完成時將使用數增加到 first_id+count。到達 40 個上限而提前回傳時不會更新。first_id=39、count=2 時回傳 1，槽位 39 啟用，使用數為 0。公開版在處理每個槽位時更新使用數。

本機版提供 vram_get_queue_used 和 vram_get_queue_free，但沒有公開版的 vram_get_queue_capacity。容量可使用 VRAM_QUEUE_MAX，預設 32。登記一個命令後，已用 1，空餘 31。

wire3d_dmg.h 僅在 96 行設定宣告 DrawRect2D。DrawBoxEdges2D 在兩種設定均有宣告，DrawIndexedEdges 僅在 120 行設定宣告。請選擇一個與設定對應的原始檔。本次未重新驗證這些函式的繪製結果。 保存的 lib 原始碼中未找到 DrawRect2D 或 DrawBoxEdges2D 的實作。這些僅為宣告，呼叫需要另外提供實作。

本機 audio_vblank 不含公開版的 AudioVBlank_QueueReset/QueueRefill/QueuePlay 和 audio_vblank_queue.inc。ZX0 也僅存在於公開版。這些屬於公開版新增功能，不計入已驗證的本機功能。

### 本次執行的驗證

2D 預設路徑：透過 KOKURA C API 自動選擇硬體，8,872 項全部通過。檢查了速度限制、接觸、有號 Q8 乘法與接觸時刻。這不是分別指定 DMG/CGB 的測試。

19 項物理邊界測試在 DMG 與 CGB 各執行一次，共 38 次全部通過。獨立的 3D 擴充與精靈/VRAM/調色盤重現測試也在兩種模式符合預期。調色盤和精靈結果是成功重現缺陷，不是確認正確運作。

這些結果是 C# 編譯器產生 ROM 的模擬器驗證，不證明 Rust 等價性、實機、音訊、繪製或所有 API 的正確性。原有圖片與測試仍是綁定到各紀錄原始碼指紋的歷史結果。

[原始碼指紋、輸入、預期值、實測值與重現步驟](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html#local-library-20261003-heading)
<!-- local-library-20261003:end -->


**[程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html)**

`wire3d_dmg` 是 Game Boy 單色線框繪圖器。128 × 96 選用 `wire3d_dmg_96.c`，128 × 120 選用 `wire3d_dmg.c`，並透過 `Wire3DDMG_*` 操作。`wire3d` 與 `dmg3d` 分別提供 96 列與 120 列設定的另一組入口。每個程式只編譯一個入口；`wire3d_cgb` 是彩色專用繪圖器。

[繪圖器指南](wire3d_dmg_guide.md)／[日文指南](wire3d_dmg_guide_ja.md)

本目錄分為三類檔案：

- 公開 API：遊戲可引入對應標頭檔，並與所需原始碼一起建置的通用程式庫。
- 輔助單元：選用的支援程式碼、暫存器宣告或佔位原始碼，本身不構成獨立的公開 API。
- 參考文件：供閱讀使用，不連結至 ROM。

## 檔案分類

### 公開 API

| 檔案 | 用途 | 一般使用方式 |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 軸對齊包圍盒（AABB）的 2D 物理、重力積分與反覆接觸求解。 | 引入 `physics2d.h`，並編譯 `physics2d.c`。 |
| `physics2d_circle.h` / `physics2d_circle.c` | 適合球類遊戲的圓形物體 2D 物理。 | 引入標頭檔並編譯原始碼。 |
| `physics3d.h` / `physics3d.c` | 具備加速度、質量加權反彈與破壞旗標的 3D AABB 物理，以及 `kq3d_dot_q8_8()`。 | 引入標頭檔並編譯原始碼。 |
| `wire3d.h` / `wire3d.c` | 使用定點數與 WRAM 暫存緩衝區的 3D 線框繪圖，支援模型隱藏線及場景遮蔽遮罩。 | 引入 `wire3d.h`，並編譯 `wire3d.c`。 |
| `dmg3d.h` / `dmg3d.c` | DMG 暫存式線框繪圖：D000 的 128×120 繪圖區、固定順序的內嵌組合語言畫線，以及依 STAT 狀態進行的 D000→8900 傳輸。 | 需要 1bpp 暫存繪圖流程時，引入標頭檔並編譯原始碼。 |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB 專用 8MHz 彩色線框繪圖，提供 2bpp WRAM 緩衝區、隱藏線與場景遮蔽、組合語言裁切繪圖，以及 HBlank DMA 無畫面撕裂顯示。 | 建置 CGB 專用 ROM，並加入標頭檔與原始碼。 |
| `system.h` / `system.c` | 初始化、影格計數、VBlank 等待、協作式 VBlank 回呼及 DI／EI 包裝函式。 | 用於逐影格推進的遊戲迴圈。 |
| `input.h` / `input.c` | 每影格按鍵狀態：按住、剛按下、剛放開與重複輸入。 | 用於選單、動作、益智與策略遊戲的操作。 |
| `vram.h` / `vram.c` | VRAM 指令佇列，可排程 BG 圖塊寫入、矩形填滿、地圖區塊複製、memcpy 與 memset。 | 遊戲執行時排入佇列，在安全時段呼叫 `vram_flush()` 或 `vram_flush_now()`。 |
| `sprite.h` / `sprite.c` | OAM 影子緩衝區、精靈配置、組合精靈、動畫推進、OAM DMA 更新與掃描線溢位檢查。 | 用於以 OBJ 為基礎的顯示。 |
| `sprite_order.h` / `sprite_order.c` | 提供四級選取優先權與循環 OAM 排序，依帶正負號的畫面座標裁切。最多接受 255 個候選，輸出不超過 40 個精靈，並隱藏未使用的槽位。 | 引入標頭並編譯原始碼；在主程式建立 OAM 影子緩衝區，於 VBlank 傳送。 [程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html#module-sprite_order). |
| `fixed.h` / `fixed.c` | Q8.8 定點數、`Vec2`、`KQRect`、clamp／min／max／lerp 與基本矩形檢測。 | 用於移動、物理、鏡頭、AI 評分等運算。 |
| `scene.h` / `scene.c` | 輕量場景表與切換、更新、繪製分派，可組織標題、遊戲、暫停等狀態。 | 用於安排遊戲狀態流程。 |
| `entity.h` / `entity.c` | 固定陣列物件池，最多容納 `ENTITY_MAX` 個小型遊戲實體。 | 回呼接收實體 ID，再透過 `entity_get(id)` 取得資料。 |
| `danmaku.h` / `danmaku.c` | 96 發定點數彈幕池、32 方向扇形彈、命中與擦彈事件，以及不受 OAM 數量限制的 CGB BG 圖塊合成。 | 引入標頭並編譯原始碼；請參閱 `danmaku_guide.md` 與 HTML 手冊中的完整範例。 [程式庫手冊](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html#module-danmaku) |
| `bank.h` / `bank.c` | 以內建操作實作的遠端資料、遠指標、遠端呼叫與簡單 MBC 記憶體區塊切換。 | 用作跨記憶體區塊存取的包裝層。 |
| `asset.h` / `asset.c` | 素材 ID 描述表，以及原始資料與圖塊載入。 | 引入標頭檔並編譯原始碼；日後產生的 `assets.h/c/json` 也可採用此結構。 |
| `debug.h` / `debug.c` | ROM 端輕量追蹤、斷言與標記緩衝區，可由 KOKURA 或其他模擬器讀取。 | 將較耗資源的效能分析留在 ROM 外執行。 |
| `chain.h` / `chain.c` | 提供蛇、繩索、列車與關節精靈用的座標歷史環形緩衝區，以及 ChainBody 身體跟隨功能。 | 引入 `chain.h` 並編譯 `chain.c`，依需求選擇歷史座標跟隨或以各節目前姿態運作的追隨方式。 |
| `cgb_tile.h` | CGB 圖塊與屬性內建操作的公開宣告。 | 遊戲使用 `__settile...` 等功能時引入。 |
| `cgb_palette.h` / `cgb_palette.c` | CGB BG／OBJ 調色盤高階介面。 | 引入標頭檔並編譯原始碼。 |
| `scroll.h` / `scroll.c` | 以編譯器內建操作實作的捲動與分割畫面表。 | 使用 `Scroll_*` 時加入。 |
| `raster.h` / `raster.c` | 分帶光柵捲動與逐掃描線 X 方向變形設定。 | 使用 `Raster_*` 函式時，引入 `raster.h`，並同時編譯 `raster.c` 與 `scroll.c`。 |
| `camera.h` / `camera.c` | 以 `scroll.*` 為基礎的 8.8 定點數鏡頭、簡單全域介面，以及世界與螢幕座標轉換。 | 引入標頭檔並編譯原始碼。 |
| `audio.h` / `audio.c` | Game Boy 音訊驅動程式，提供音樂、音效、聲像、波形與淡入淡出；共有 68 個音符索引，最高為 67（`G6`）。 | 引入 `audio.h` 並編譯 `audio.c`。 |
| `audio_vblank.h` / `audio_vblank.c` | VBlank IRQ BGM 驅動，同樣支援 68 個音符編號，提供可容納 16 筆紀錄的 WRAM 佇列以播放跨記憶體區塊歌曲，並可掛接逐影格處理。 | 直接指標曲目放在固定 bank 0，或由其他 bank 的程式補充佇列；以 `scripts/patch_gb_vblank_irq.ps1` 設定向量 `0x0040`。 |
| `link.h` / `link.c` | 通訊線的序列位元組傳輸，以及協作式邏輯 4 人通訊 `Link4_*`。 | 加入需要通訊的專案。 |
| `link_packet.c` | `link.c` 上的選用封包層，包含 `Link4_*` 各對端獨立的收件匣。 | 需要封包收發時，才與 `link.c` 一起編譯。 |
| `link_dmg07.h` / `link_dmg07.c` | 實體 Nintendo DMG-07 Four Player Adapter 的外部時脈輪詢驅動。 | 與 `link_hwregs_gb.c` 一起編譯；此驅動與邏輯 `Link4_*` API 各自獨立。 |
| `rpg.h` | RPG／ADV／SLG 共用宣告與底層內建操作宣告。 | 使用此功能群組時，在遊戲原始碼中引入。 |
| `rng.c` | `rng8`、`rng16`、`rand_range`、`weighted_choice`、`rng_seed`、`rng_next8`、`rng_next16`、`rng_range`、`rng_chance`。 | 使用 `rpg.h` 的亂數函式時編譯。 |
| `flags.c` | 2048 個旗標位元的位元集合與任務狀態儲存。 | 使用 `rpg.h` 的旗標、任務功能時編譯。 |
| `rle.c` | 解碼 RAM 與遠端 ROM 中簡單的 `[數量][值]` RLE 資料。 | 使用 `rpg.h` 的 `rle_decode*` 時編譯。 |
| `text.c` | 圖塊字串視窗、換頁等待、選項、直接 XY 文字、數值顯示，以及清除／視窗別名。 | 使用 `rpg.h` 的文字功能時編譯。 |
| `menu.c` | 直向選單、基本物品選單與非阻塞選單狀態 API。 | 使用 `rpg.h` 的選單功能時編譯。 |
| `script.c` | RPG／ADV 流程的小型位元組碼執行器。 | 使用 `rpg.h` 的腳本功能時編譯。 |
| `map.c` | 載入封裝地圖，包含碰撞、觸發器、鏡頭與選用的 16×16 組合圖塊。 | 使用 `rpg.h` 的地圖功能時編譯。 |
| `save.c` | MBC5 方式的 SRAM 儲存、載入、檢查與清除，資料包含標頭、版本、長度及檢查碼。 | 使用 `rpg.h` 的存檔功能時編譯。 |
| `slg_unit.c` | 策略遊戲的移動與攻擊範圍。 | 使用 `rpg.h` 的戰術單位功能時編譯。 |
| `slg_path.c` | 廣度優先尋路與移動成本洪水填滿。 | 使用 `rpg.h` 的戰術尋路功能時編譯。 |
| `slg.h` / `slg_board.c` | 棋盤、走法清單與復原堆疊等通用棋類或戰術系統功能。 | 引入 `slg.h` 並編譯 `slg_board.c`；遊戲專屬的評估邏輯另行實作。 |

### 輔助單元

| 檔案 | 用途 | 說明 |
| --- | --- | --- |
| `audio_hwregs_gb.c` | 最小 APU 與波形 RAM 暫存器宣告。 | 專案尚未透過其他檔案宣告這些暫存器時才加入。 |
| `link_hwregs_gb.c` | 通訊專案所需的最小 `SB`、`SC`、`IF`、`IE` 宣告。 | 若其他檔案已宣告序列通訊暫存器，請勿重複加入。 |
| `cgb_tile.c` | 刻意保留為空的 CGB 圖塊編譯單元。 | 公開介面在 `cgb_tile.h`；可編譯此檔案，但執行邏輯不需要它。 |
| `math.c` | ROM 正弦表 `MATH_SIN`。 | 尚未列為有穩定文件的公開 API，目前作為專案輔助資料單元使用。 |

### 參考文件

| 檔案 | 內容 |
| --- | --- |
| `README.md` | 本概覽與建置說明的英文版。 |
| `wire3d_guide_ja.md` | 線框 3D 繪圖器日文入門指南。 |
| `dmg3d_guide_ja.md` | DMG 暫存式線框繪圖器日文入門指南。 |
| `wire3d_cgb_guide.md` | CGB 專用彩色線框繪圖器入門指南。 |
| `physics_guide.html` | 物理程式庫英文使用指南。 |
| `physics_guide_ja.html` | 物理程式庫日文使用指南。 |

## 建置方式

以下指令示範如何將應用程式原始碼與函式庫模組一起建置。請自行準備指令中指定的應用程式檔案。隨附的入門程式可依 `../examples/build.ps1` 與HTML手冊建置。簡短指令 `kitaqgb` 需要執行檔已加入PATH。

將遊戲原始碼與所需程式庫一起編譯：

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

線框 3D 專案須一併編譯繪圖器原始碼：

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

隨附的 `examples/wire3d_minimal.c` 會初始化所有模型欄位，並在 128×96 視埠中旋轉立方體，不需要外部圖像或字型素材。

採用 128×120 視埠的 DMG 專案，可使用 120 列入口 `dmg3d.*`：

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` 使用 D000 的 WRAM 暫存區，以及從 0x8900 開始的圖塊上傳，設定 128×120 可見線框繪圖區。`DMG3D_BeginFrame()` 只重設遮蔽狀態；上傳時會讀取並清除像素。`DMG3D_EndFrame()` 先等待 VBlank，再查詢 STAT 進行傳輸，傳輸可能持續到 VBlank 結束之後。隨附的 `examples/dmg3d_minimal.c` 啟用差分傳輸，每影格重新繪製十字。輔助傳輸為獨立操作，與主暫存區共用來源儲存空間。

CGB 專用彩色專案使用獨立的 `wire3d_cgb.*`：

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` 將 CGB 切換為雙倍速，設定 128×96、2bpp 的 BG 線框繪圖區，並安裝預設的四項 BG 調色盤。一般與 `Fast` 兩組影格 API 都採無畫面撕裂顯示：以 HBlank DMA 將 `0xD300-0xDEFF` 的 3072 位元組傳至未顯示的 VRAM 圖塊區塊，再於 VBlank 切換顯示，不必逐影格改動 LCDC。`Fast` 組省略一般影格結束時的 BG 佇列檢查。可透過 `Wire3DCGB_SetPaletteRGB15()`、`Wire3DCGB_SetLineColor()` 及 `Wire3DCGB_Draw*Color()` 指定顏色。

對於 CAD 產生、依方向劃分的 LOD，`Wire3DCGB_DrawMaskedModel2D()` 可接收預先投影的有號頂點偏移量與封裝的可見邊遮罩。邊的走訪及組合語言光柵化都在繪圖器的記憶體區塊 4 內完成，因此繪製一個模型時，不必為每條線進行跨區塊呼叫。

若畫面變動不大，且遊戲使用 OAM 影子緩衝區，可在進入 VBlank 後依序呼叫 `sprite_flush_oam()` 與 `Wire3DCGB_EndFrameSparseNow()`。這會省略一開始等待下一次 VBlank 的步驟；但 DMA 與顯示切換仍可能因修改範圍、當前掃描線而等待，無法保證全部工作都在同一次 VBlank 內完成。

CGB 線條請使用顏色 1、2、3。一般 128×96 模式會疊加色彩位元，因此 1 與 2 重疊會得到 3。顏色 0 無法擦除線條，應清空影格或使用專用擦除函式。一般 `Wire3DCGB_DrawLine2D` 與模型繪製不會記錄差分上傳範圍。需要同時記錄範圍時，請用 `Wire3DCGB_DrawLineClipped2D` 畫線，或呼叫 `Wire3DCGB_InvalidateFrameHistory`，將整個視埠納入下一次差分上傳。

160×144 模式每影格最多配置 127 個圖塊。高速畫線路徑遇到配置失敗或越界座標時，會設定 `Wire3DCGB_GetFullScreenOverflow()`，並停止後續像素寫入，直到下一影格重設。請讓頂點保持在所選視埠內。三角形遮罩的右側擴張範圍在 128×96 模式下止於 X=127，全螢幕模式下則止於 X=159。請遵循 API 的 WRAM 記憶體區塊映射要求，尤其是全螢幕與 FastMap 功能。

[CGB 三角形遮罩邊界回歸測試](../tests/library/wire3d_cgb_mask_bounds.c)提供檢查兩種視埠的完整程式。

RPG／ADV／SLG 功能建置範例：

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

標準執行階段基本功能檢查的建置範例：

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

序列通訊專案應先編譯序列暫存器宣告單元，再編譯通訊核心：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

由主機選擇通訊對端的協作式邏輯 4 人專案也使用這些檔案。主機呼叫 `Link4_InitHost(slot_count)`，並以 `Link4_SelectPeer()` 或 `Link4_SendPacketTo()` 選擇對端。其他參與者呼叫 `Link4_InitPeer(local_slot, slot_count)`，與槽位 0 的主機通訊。

也可直接建置已指定槽位的對端包裝範例：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

實體 DMG-07 專案使用專用輪詢驅動：

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

請持續呼叫 `LinkDmg07_Poll()`，因為轉接器位元組之間的間隔遠短於一個視訊影格。每次 VBlank 呼叫一次 `LinkDmg07_TickFrame()`，更新採飽和計數的靜默與交握逾時計數器。驅動始終使用外部時脈 `SC=$80`，對連線探測回覆 `88 88 RATE 01`；只有實體玩家 1 能傳送 `AA AA AA AA` 來請求傳輸。

所有主機觀察到 `CC CC CC CC` 後，每個四位元組廣播封包便包含各實體槽位的一個位元組資料。轉接器會在資料提交後的下一個封包才廣播，因此驅動會捨棄第一個內容未定義的封包，並提供收發序號。`LinkDmg07_RequestRestart()` 等待下一個封包邊界，傳送對齊的 `FF FF FF FF`，在收到轉接器完整的全 FF 指示後停止。傳輸階段發生靜默逾時，也會自動排定此重新啟動流程，並保留目前在四位元組封包內的位置。轉接器時脈恢復後，可先完成當前封包，再進入復原流程。

接著在遊戲程式引入所需標頭檔：

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## 使用注意事項

<!-- audio-pitch-range -->
最後八個音符索引會重用前一個八度的頻率；68 個索引不代表有 68 種不同音高。

- `inv_mass_q8 == 0` 表示靜態物體。
- `Wire3D_Init()` 使用 128×96 的 BG 線框繪圖區、從 `0xD000` 開始的 WRAM 暫存區，以及從 `0x8900` 開始的圖塊資料。Wire3D 專案請指定 `--stack-bank=fixed`。
- `Wire3D_BeginFrame()` 清空 WRAM 暫存區；`Wire3D_EndFrame()` 等待 VBlank，再依 STAT 條件分塊複製到 VRAM。
- Wire3D 角度分為 16 檔，基本模型路徑每個模型最多支援 `WIRE3D_MODEL_VERTEX_LIMIT` 個頂點。
- 多個帶面的線框物件重疊時，可使用 `Wire3D_DrawScene()`。它先畫近處物件，累積可見面遮罩，再以保守方式略過遠處被遮住的線條。
- `Wire3DCGB_Init()` 僅支援 CGB，並透過 KEY1／STOP 切換雙倍速。建置時指定 `--cgb=cgb_only`，不要在同一 ROM 混用 `wire3d_cgb.*` 與 DMG 相容的 `wire3d.*`。
- 一般與 `Fast` 影格 API 都會保留舊影格，直到非顯示中的 VRAM 圖塊區塊完成傳輸。不使用 BG 圖塊寫入佇列的場景，優先採用 `Fast`。
- 宣告 `NR10..NR52` 與 `WAVE0..WAVE15` 的檔案，必須先於 `lib/audio.c` 編譯。
- `lib/audio_hwregs_gb.c` 已提供這些宣告；請勿與其他重複宣告相同音訊暫存器的檔案一起編譯。
- `cgb_tile.h` 直接公開內建操作；`lib/cgb_tile.c` 只是佔位單元，一般建置可以省略。
- `cgb_palette.h` 的公開 API 採用 `cgb_*` 命名。
- 選單或設定變更音樂、音效啟用狀態時，請呼叫 `Audio_SetMusicEnabled()`／`Audio_SetSfxEnabled()`。
- `Audio_PlaySFX()` 會記錄目前可見的 ROM 記憶體區塊。已知音效資料所在區塊時，請使用 `Audio_PlaySFXBanked(bank, sfx, priority)`。
- 音樂串流的 `AUDIO_CMD_NOTE`／`AUDIO_CMD_SET_INST` 使用硬體順序的聲道編號：`0=CH1`、`1=CH2`、`2=CH3`、`3=CH4`。
- 自訂 CH3 波形時，將 32 個 4 位元取樣值封裝為 16 位元組，再傳給 `Audio_LoadCustomWave()`。
- `Audio_FadeToMasterVolume()` 的淡變由 `Audio_Update()` 推進，淡變期間仍須每影格呼叫。
- `audio_vblank.c` 定義 VBlank IRQ 向量符號 `__kq_vblank_vector`。BGM 事件由五個位元組組成：`delay, ch1_note, ch2_note, ch3_note, ch4_noise_param`；休止、循環與結束分別使用 `AUDIO_VBLANK_REST`、`AUDIO_VBLANK_LOOP`、`AUDIO_VBLANK_END`。
- 以直接指標播放的 VBlank 曲目須位於固定 bank；佇列模式可從其他 bank 的曲目補充資料。連結 `lib/audio_vblank.c` 後，請執行 `scripts/patch_gb_vblank_irq.ps1 <rom> <map>`，讓向量 `0x0040` 跳至 ISR 並更新 ROM 校驗和。
- 未加入共用 IRQ 分派器時，不要將 `audio_vblank.c` 與其他同樣使用 VBlank 向量 `0x0040` 的程式庫或遊戲中斷入口程式搭配。
- `Scroll_SplitCommit()` 自動啟用 IE 位元 `0x01 | 0x02`，並使用編譯器提供的 VBlank／STAT 處理常式套用分割畫面設定。
- 目前建置中的分割畫面功能保留向量 `0x0040` 與 `0x0048`，暫勿與獨立自訂的 VBlank／STAT 中斷入口程式混用。
- 宣告 `SB`、`SC`、`IF`、`IE` 的檔案，應先於 `lib/link.c`／`lib/link_packet.c` 或 `lib/link_dmg07.c` 編譯。
- `lib/link_hwregs_gb.c` 已提供這些宣告；請勿與其他重複宣告相同序列暫存器的檔案一起編譯。
- 通訊程式庫不佔用序列向量 `0x0058`。啟用中斷模式後，應從自己的 IRQ 入口程式或分派器呼叫 `Link_OnSerialIRQ()`。
- 封包層只保留一層待收資料，適合由逐影格主迴圈及時推進處理。
- `Link4_*` 模擬主機選擇對端的協作式 4 人連線；每次只有一個對端在線路上活動，主機須主動輪替。
- `Link4_TryReadByteFrom()`／`Link4_HasPacketFrom()` 提供各對端獨立的收件匣，讓主機輪詢多位參與者時仍能辨別資料來源。
- `Link_ReadPacket()` 讀取最近收到的一個封包；4 人流程請使用 `Link4_ReadPacketFrom()`。
- `Link4_*` 不實作 Nintendo DMG-07 的電氣行為或協定。實體配件請使用 `link_dmg07.c`，不要在同一 ROM 與 `link.c` 一起編譯。
- DMG-07 的 `GetConnectedMask()` 以位元 0～3 表示實體玩家 1～4。傳輸期間保留最後一次連線探測結果；成員資訊只能在探測階段更新。
- 轉接器未提供時脈時，待處理的 DMG-07 重新啟動流程無法推進。復原流量會被捨棄，不作為正常序列資料提供。若配件重新上電而進入不同階段，不只是暫停時脈，應明確重新初始化驅動與工作階段。
- 物理程式庫只計算線性位置與速度，不包含角動力學。
- Game Boy 等級硬體應控制物體數量，例如維持 8～24 個活動物體。
- 依每個遊戲世界的需求，調整重力、最大速度與求解反覆次數。
- 撞球類遊戲優先使用 `physics2d_circle.*`，而非 AABB 程式庫。
- 目前結構不必另外加入 `random`、`collision`、`ui`、`tilemap`、`dialog`、`board_game`、`simple_physics` 程式庫；對應功能請使用 `rng`、`physics2d`、`text`／`menu`、`map`、`script`、`slg`、`physics2d`。
- `scene.c` 與 `entity.c` 避免在函式指標呼叫中使用指標大小的參數。目前 KITAQGB 的函式指標呼叫路徑，以無參數或單一位元組 ID 最為可靠。

<!-- readme-language-links:start -->
[English](README.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

<!-- manual-language-links:start -->
| Language / 言語 | HTML |
| --- | --- |
| English | [gb-library](https://bartaro.github.io/kitaq-docs/en/gb-library.html) |
| 日本語 | [gb-library](https://bartaro.github.io/kitaq-docs/gb-library.html) |
| 한국어 | [gb-library](https://bartaro.github.io/kitaq-docs/ko/gb-library.html) |
| 简体中文 | [gb-library](https://bartaro.github.io/kitaq-docs/zh-CN/gb-library.html) |
| 繁體中文 | [gb-library](https://bartaro.github.io/kitaq-docs/zh-TW/gb-library.html) |
| Français | [gb-library](https://bartaro.github.io/kitaq-docs/fr/gb-library.html) |
| Español | [gb-library](https://bartaro.github.io/kitaq-docs/es/gb-library.html) |
| Deutsch | [gb-library](https://bartaro.github.io/kitaq-docs/de/gb-library.html) |
<!-- manual-language-links:end -->

<!-- rust-native-20261004:start -->
## 使用原生編譯器呼叫程式庫

lib/ 包含編入遊戲機 ROM 的 C 標頭與原始碼。PC 編譯器與輔助工具使用 Rust；遊戲原始碼、目標平台程式庫與 API 仍使用 C。提供的 SOURCE_MANIFEST.json 與 LIBRARY_MERGE.json 記錄程式庫原始碼的選擇。

Windows、Linux、macOS ARM 和 macOS Intel 的原生建置及執行驗證均成功。KITAQGB 在每個環境通過 48 項測試與 395 項輔助工具檢查；KITAQFC 通過 55 項測試與 401 項檢查，也驗證了 Rust 1.85。PUBLIC_DISTRIBUTION.json 記錄已配置程式的雜湊與驗證來源。公開 GitHub Actions 會獨立建置及驗證這些原始碼。

保存的參考輸出用於檢查 ROM 位元組、診斷與輔助工具格式。先前的 C# 模擬器驗證保留為對應原始碼指紋的歷史紀錄，並不自動證明所有 Rust API、實體硬體或完整的 FDS BIOS 遊戲啟動。原始 PNG 轉換腳本無法取得，因此依規格重新實作，不能宣稱與原腳本位元組一致。

[Rust 原生編譯器與輔助工具](../tools/README.zh-TW.md)

<!-- rust-native-20261004:end -->
