# GBSEQ — 使用手冊

**2026-11-14 — source update / ソース更新:** 按 Ctrl+K 或鋼琴圖示開啟演奏鍵盤，鍵盤音訊連續輸出。新增或開啟曲目前，視窗內會提示儲存、捨棄或取消未儲存的編輯。播放ROM在SOLO/MASTER模式下按B切換循環，TIME顯示目前位置／總時長。

播放時按需產生短音訊區塊。切換監聽、靜音或獨奏會保留播放位置；音符和速度編輯在下次播放時生效。

[Audio implementation and source checks](../../SOURCE_UPDATE_20261114.md). The packages in `bin/` retain their recorded build revisions; build the current source for this update.

[MIDI 檔案與裝置、演奏鍵盤、錄音及 512 種音色](docs/MIDI_PERFORMANCE.md) · `Ctrl+K` · `import-midi` / `export-midi`

匯出的ROM包含用於啟動相容性驗證的48位元組任天堂標誌資料。GBSEQ的MIT授權不授予任天堂標誌或商標的獨立使用權，GBSEQ與任天堂沒有合作關係，也未獲其認可。[英文／日文聲明](docs/NINTENDO_NOTICE.md)。

在匯出選單中選擇可編輯的 UGE（含自訂波形）或 MOD（GBT 固定音色）。每臺主機各匯出一個檔案，存入 `assets/music`。 [CLI: `export-uge` / `export-mod`](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ由DAISUKE OBA製作，提供plita桌面GUI與獨立CLI，共用編輯模型、驗證、KOKURA音源及匯出功能。LOCAL 4CH、CABLE 8CH、DMG07 16CH分別支援1、2、4台Game Boy。每台CH1/CH2為脈衝波、CH3為波形、CH4為雜訊。每曲1–256小節，每小節16步（每聲道最多4096步），BPM 30–300，可儲存循環設定。 在「顯示 → 語言」選擇介面語言。此設定與樂曲分開儲存。 `--language en|ja|ko|zh-CN|zh-TW|es|pt-BR|fr|de`

## 演奏輸入與 WAV 匯出

選擇目標軌道，按 Ctrl+K 開啟鍵盤。白鍵是 A/S/D/F/G/H/J，黑鍵是 W/E/T/Y/U，Z/X 切換八度。按 IN 和 OUT 切換 MIDI 裝置。步進輸入每次前進 1、2 或 4 個十六分音符；錄音從曲首開始錄一遍。保護模式會略過重疊的輸入，覆寫模式只替換重疊的區段。從檔案選單匯入 MIDI，從匯出選單寫出 MIDI。請用 .gbseq 保留 GB 專用音色。

每種聲道有 128 個預設音色，可從下拉選單挑選；使用者音色可另存為 .gbpreset 並再次載入。和弦標記可分別為整體混音和各軌道開啟。全曲移調不會改動雜訊軌。

WAV 會套用開始匯出時的監聽、靜音與獨奏設定。監聽全部會混合目前模式的所有主機，P1–P4 只匯出指定主機。一旦啟用獨奏，就只播放未靜音的獨奏軌。僅選取軌道進行編輯不會改變聲音。所有軌道靜音時，WAV 無聲但長度不變。

`gbseq-cli render-wav song.gbseq mix.wav` · `gbseq-cli render-wav song.gbseq p2.wav --player 2`

[匯出 VBlank 驅動程式與樂曲的 C 原始碼](docs/KITAQGB_VBLANK.md): `gbseq-cli export-vblank song.gbseq output` · `--strict` · `--driver-dir`.
記錄採用 audio.h 的聲道順序。VBlank 驅動程式的 CH3 只有三角波與鋸齒波；若不允許近似，請使用 --strict。各主機的原始碼分開編譯。

按 **E** 或按兩下音軌標題即可開啟該軌的鋼琴捲軸編輯器；再按 E 返回總覽。拖曳音符右端可延長至下一個音符之前或曲末。在 CH3 自訂波形圖上可直接繪製 32 個取樣點，Ctrl+Z 可復原一筆。

## 分聲部播放 ROM 與監聽

在匯出選單選 **Playback ROMs (no SRAM)**，即可產生 P1.gb 至 P4.gb；每個 ROM 為 512 KiB，內含曲名及本機的四個聲道，不需要存檔。按 START 可單機試聽。連線時在各機停止狀態按 SELECT，一台按 A 選 MASTER，其餘按 B 選 PEER，再由主機按 START 播放或停止。四人轉接器的主機接第 1 埠，與聲部編號無關。在 GBSEQ 中可獨立於編輯畫面監聽全部或單一聲部，WAV 依目前的監聽選擇匯出。

`gbseq-cli export-player-roms song.gbseq output_folder`

## 安裝與啟動

從[bin/](bin/)選擇Windows x86_64、Linux x86_64、macOS Apple Silicon或Intel套件並完整解壓縮，保留程式旁的assets與授權檔。Windows GUI為gbseq.exe，Unix為./gbseq。下列CLI範例的gbseq-cli，在PowerShell使用./gbseq-cli.exe，在Unix使用./gbseq-cli；加入PATH後可省略前綴。一般使用不需編譯器、另外安裝plita或Python。

GUI需要桌面環境、相容GPU驅動及試聽用音訊裝置。Linux面向Ubuntu 24.04以後的相容系統，使用ALSA、X11/Wayland、libxkbcommon與libxkbcommon-x11及Vulkan/OpenGL執行期程式庫。macOS執行檔未簽署、未公證；核對來源與雜湊後，必要時透過系統「打開」或「隱私權與安全性」僅允許此應用程式，不要停用全域防護。CLI產生WAV不需要螢幕或音效裝置。Unix若遺失執行權限，可用chmod +x gbseq gbseq-cli。

## GUI編輯

選擇連線模式與P1–P4。DMG07的ALL16同時顯示16聲道。雙擊空白軌道建立音符，雙擊音符開啟詳細面板。拖曳主體改變時間與音高，拖右緣改長度。每軌為單音，拒絕重疊。小節數為1–256。滾輪及底部捲軸移動；F顯示全曲，Home回開頭，+/-縮放。播放時畫面跟隨位置。Space播放/停止，LOOP ON/OFF控制循環。播放時按需產生短音訊區塊。切換監聽、靜音或獨奏會保留播放位置；音符和速度編輯在下次播放時生效。

Ctrl/Shift點擊切換選取，Ctrl+A選取目前軌道。左右按格線移動，上下半音，Shift+上下八度。Ctrl+D複製，Delete刪除；G切換格線，Q對齊起點；Ctrl+Z/Y復原/重做，Ctrl+C/V複製/貼上。Ctrl+O開啟，Ctrl+S儲存，Ctrl+Shift+S另存新檔。取代未儲存編輯前會確認，覆寫前請備份原件。

## 音色與CH3波形

每音符可設音量、左/右/中央聲像、脈衝工作週期與包絡、CH1掃頻、CH3輸出位準/預設、CH4雜訊。AUDITION試聽選取音符。CH3沒有硬體包絡。NEW USER WAVE編輯32個4位元取樣，拖曳繪製0–15。可存64個使用者波形，另有7個內建波形，配置至16個有效預設並選100/50/25%位準。每音符的WAVE PRESET讓音色隨時間改變。硬體重寫波形會短暫關閉CH3，不是連續交叉淡化。

## CLI

track/player從1起算，start從0；length=1為十六分音符，16為4/4一小節。pitch 0–47對應C2–B5，velocity為0–15。players只接受1、2、4。編輯必須指定--out，明確指定的既有檔可能被覆寫。不合法編輯以非零碼結束且不儲存。transform另有--shift、--quantize、--velocity。WAV預設混合所有啟用的主機；--player N 只輸出指定主機。匯出可取代既有輸出。

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## LINK16與KITAQGB

LINK16 SAV產生V4格式128 KiB MBC5電池映像：link16_local.sav；link16_cable.p1.sav/.p2.sav；或link16_dmg07.p1.sav到.p4.sav。使用配對的LINK16 ROM版本及模擬器存檔名稱規則，取代前先備份舊SAV。P1也包含其他玩家的鏡像。SAV本身不能啟動。每台有4個共用包絡預設，超額值可能近似；載入後請試聽。V4不儲存CH1掃頻。frames_per_step_override逐影格音效可用於預覽/WAV/KITAQGB，但BPM式LINK16匯出會拒絕。

KITAQGB以玩家為單位匯出C/音訊串流。使用實體聲道ID CH1=0、CH2=1、CH3=2、CH4=3的相容audio.h/audio.c。這不能將16聲道塞進一台Game Boy。GBSEQ預覽把步長量化成視訊影格，因此部分BPM與分數時鐘ROM的實際速度不同。模擬器渲染不等於真機測試。

## 資料與Python

.gbseq是JSON，不是MIDI；改副檔名不會轉換格式。AI產生的資料須validate，檢查曲長、有效聲道、唯一ID、音域與重疊。[詳細規格](docs/README.md)為日文。選用Python 3.10+介面時，把python/加入PYTHONPATH，將GBSEQ_NATIVE_LIBRARY設為隨附gbseq.dll/libgbseq.dylib/libgbseq.so的絕對路徑。Python聲道/玩家從0起算，直接編輯後呼叫validate；player=None混合全部玩家。

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## 建置、疑難排解與授權

使用Rust 1.93.1與系統C/C++工具。Linux建置需libasound2-dev、libudev-dev、libxkbcommon-dev, libxkbcommon-x11-dev、libwayland-dev、libx11-dev、libxi-dev、libxcursor-dev及Mesa開發程式庫。CI逐OS驗證測試、音訊/存檔匯出與GUI啟動，verification記錄來源提交。這些檢查不驗證實際音效裝置輸出、實體操作或Game Boy硬體。

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


啟動失敗查看GPU驅動與終端錯誤；缺字檢查assets/fonts；無聲檢查裝置及mute/solo；匯出失敗檢查validate指出的音符。本體為MIT，Copyright (c) 2026 DAISUKE OBA。請讀[LICENSE](LICENSE)、[第三方聲明](THIRD_PARTY_NOTICES.md)及[稽核清單](LICENSE_AUDIT.json)，重新散布時保留完整聲明與字型授權。第三方素材與匯入音樂各自保留原有權利。
