# GBSEQ — 操作説明書

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

DAISUKE OBA制作のGBSEQは、plita製GUIと独立したCLIを備えるゲームボーイ用シーケンサーです。編集モデル、検証、KOKURA音源、書き出し処理を共用します。LOCAL 4CH・CABLE 8CH・DMG07 16CHで1台・2台・4台分を制作できます。各台はCH1矩形波、CH2矩形波、CH3波形、CH4ノイズ。1曲1～256小節、1小節16ステップ（各CH最大4096）、BPM30～300、ループ有無を設定できます。GUIの表示は英語、説明書は9言語です。

## 導入と起動

[bin/](bin/)からWindows x86_64、Linux x86_64、macOS Apple SiliconまたはIntel用を選び、アーカイブ全体を展開してください。assetsとライセンス類も実行ファイルと一緒に置きます。GUIはWindowsで`gbseq.exe`、macOS/Linuxで`./gbseq`。CLIは`gbseq-cli`です。PowerShellでは`./gbseq-cli.exe`、UnixではPATHに登録していなければ`./gbseq-cli`とします。以下の例はその接頭辞を省略しています。通常の利用にコンパイラー、別途のplita、Pythonは不要です。

GUIにはデスクトップ環境、対応GPUドライバー、試聴用の音声デバイスが必要です。Linux版はUbuntu 24.04以降の互換環境向けで、ALSA、X11/Wayland、libxkbcommon、Vulkan/OpenGLの実行時ライブラリを使用します。macOS版は未署名・未公証です。入手元とハッシュを確認し、必要な場合はOSの「開く」または「プライバシーとセキュリティ」でアプリ単位の許可を行ってください。保護機能全体を無効にする必要はありません。CLIのWAV生成には画面や音声デバイスは不要です。Unixでは実行属性を保持し、必要に応じて`chmod +x gbseq gbseq-cli`を実行します。

## GUIで制作

通信構成とP1～P4を選びます。DMG07のALL16では16CHを同時表示・再生します。空きレーンのダブルクリックで音符を作成、音符のダブルクリックで詳細を開きます。本体ドラッグで時刻・音高、右端で長さを変更。各レーンは単音なので重複配置はできません。小節数欄は1～256。ホイールと下部スクロールバーで移動、Fで全曲表示、Homeで先頭、+/-で拡縮します。再生中は画面が演奏位置に追従します。Spaceで再生／停止、LOOP ON/OFFで繰り返しを指定。初回再生は全曲を音声化するため、長い曲では準備時間がかかります。

Ctrl/Shiftクリックで複数選択、Ctrl+Aで現在レーン全選択。左右でグリッド単位移動、上下で半音、Shift+上下でオクターブ変更。Ctrl+Dで複製、Deleteで削除、Gでグリッド変更、Qで開始位置を整列。Ctrl+Z/Yで取消／やり直し、Ctrl+C/Vでコピー／貼付。Ctrl+Oで開く、Ctrl+Sで保存、Ctrl+Shift+Sで別名保存。未保存編集を置き換える際は確認されます。上書き前に元ファイルを控えてください。

## 音色と波形

各音符の音量、左右／中央パン、矩形波のデューティ比とエンベロープ、CH1スイープ、CH3出力レベル／プリセット、CH4ノイズを設定できます。AUDITIONは選択音の試聴です。CH3にはハードウェアエンベロープがありません。NEW USER WAVEで32サンプルの4bit波形を開き、ドラッグで0～15を描きます。ユーザー波形は64個、組込波形は7種類。16個の有効プリセットに割り当て、100/50/25%のレベルを選びます。音符ごとのWAVE PRESETで途中の音色を変更します。実機での波形書換えはCH3を一時停止するため、連続クロスフェードではありません。

## CLI

track/playerは1始まり、startは0始まりのステップ数です。length=1は16分音符、16は4/4の1小節。pitch=0～47はC3～B6、velocity=0～15。`--players`は1・2・4のみ。編集には`--out`が必須で、指定先が既存なら上書きされます。不正編集は非0終了し保存されません。transformには`--shift`、`--quantize`、`--velocity`もあります。`--all-players`で全台ミックス、未指定ではP1です。書き出しも既存出力を置き換える場合があります。

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


## LINK16・KITAQGBへの出力

LINK16 SAVはV4形式の128KiB MBC5バッテリーイメージです。LOCALは`link16_local.sav`、2台は`link16_cable.p1.sav`と`.p2.sav`、4台は`link16_dmg07.p1.sav`～`.p4.sav`。対応するLINK16 ROMとエミュレーターの保存名規則を使用し、既存SAVを必ずバックアップしてください。P1には他台のミラー領域もあります。このGBSEQ配布物にLINK16 ROMは含みません。SAV単体では起動できません。LINK16のエンベロープは1台で共用4種類までで、それを超える値は近似される場合があります。出力後に読み込みと音を確認してください。CH1スイープはV4には保存されません。`frames_per_step_override`によるフレーム単位SFXは試聴・WAV・KITAQGB出力に対応し、BPM方式のLINK16出力では拒否されます。

KITAQGB出力は1台分のC／ストリームデータです。台ごとに出力してください。物理CH1=0、CH2=1、CH3=2、CH4=3の現行互換audio.h/audio.cを使います。16CHを1台に詰め込む機能ではありません。GBSEQ試聴はステップ長を映像フレーム単位に丸めるため、BPMによっては分数クロックのROMと実テンポが異なります。エミュレーターでの音声生成は実機検証とは別です。

## データとPython

`.gbseq`はJSON形式で、MIDIではありません。拡張子を変えるだけでは変換できません。生成AIのデータはvalidateし、曲長、使用CH、ID重複、音域、音符重複を確認してください。詳細仕様は[docs/](docs/README.md)。任意のPython 3.10以降連携ではpython/をPYTHONPATHに追加し、GBSEQ_NATIVE_LIBRARYに同梱のgbseq.dll／libgbseq.dylib／libgbseq.soの絶対パスを指定します。Pythonのtrack/playerは0始まり、直接編集後はvalidateします。player=Noneで全台出力します。

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## ビルド・困ったとき・権利表記

Rust 1.93.1と各OSのC/C++開発環境を使用します。Linuxではlibasound2-dev、libudev-dev、libxkbcommon-dev, libxkbcommon-x11-dev、libwayland-dev、libx11-dev、libxi-dev、libxcursor-dev、Mesa開発ライブラリ等が必要です。CIは各OSでビルド・編集テスト・音声／SAV出力・GUI起動を確認し、verificationにソースコミットを記録します。音声デバイスの出音、物理入力、実機はこの自動検査には含みません。

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


起動失敗時はGPUドライバーと端末のエラー表示を確認。文字欠けはassets/fonts、無音は出力デバイスとmute/solo、出力失敗はvalidateの該当音符を確認します。本体はMIT、Copyright (c) 2026 DAISUKE OBA。[LICENSE](LICENSE)・[第三者表記](THIRD_PARTY_NOTICES.md)・[監査一覧](LICENSE_AUDIT.json)を参照し、再配布時はライセンスとフォント表記を保持してください。第三者素材と読み込む音楽の権利はそれぞれに帰属します。
