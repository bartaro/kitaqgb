# GBSEQ — MIDI, performance keyboard and instruments

## 日本語

**ファイル → MIDIを読み込み**で新しい編集用の曲に変換し、**出力 → MIDIを書き出し**で全プレイヤーの有効トラックを1つの標準MIDIファイルに書き出します。読み込み後の「保存」は別の `.gbseq` を作ります。MIDIは演奏情報の交換用で、GBのデューティ・スイープ・CH3波形・ループ設定を保存する形式ではありません。原本は `.gbseq` で保存してください。

**表示 → 演奏鍵盤 / MIDI（K）**でパネルを開きます。編集したいトラックを先に選んでください。

- 白鍵は **A S D F G H J**、黒鍵は **W E T Y U**。マウスでも弾けます。**Z / X** または **Oct − / +** でオクターブを変更します。押している間だけ発音します（音色自体の減衰は別です）。各GBトラックは単音です。
- **IN** をクリックして入力ポート、**OUT** で出力ポートを順に切り替えます。最後の次はOFFです。外部鍵盤の任意のMIDIチャンネルのノート36～83（C2～B5）を選択トラックのGB音色で試奏できます。OUTは再生中の曲を外部音源へ送ります。入力をそのまま送り返すMIDI Thruではありません。専用ケーブル、OSに認識されるUSB MIDI機器、または仮想ポートを使用してください。
- **試奏**は譜面を変更しません。**ステップ入力**では押鍵で音符を追加し、**入力長**（1・2・4個の16分音符）だけ進みます。休符は音を置かず進みます。
- **録音 / 停止**は曲頭から1回、伴奏を鳴らしながら録音します。開始後に弾いてください。押鍵と離鍵を最寄りの16分に丸め、最低1ステップを確保します。途中停止・曲末で確定し、録音テイクは1回のUndoで戻せます。
- 初期値の**既存音符を保護**では、重なる入力を拒否します。**上書き**にすると入力区間だけを置き換え、古い音符の前後は残ります。どちらの入力方法にも適用され、設定は次回起動時にも復元されます。
- **音色 ▾** は選択トラック専用のドロップダウンです。各CHに128種類、4系統で計512種類。下端の左右・ホイールでページ切替、その行をクリック、または1～8で決定します。選択中の同一トラックの音符と、以後の新規入力に適用します。別プレイヤー・別トラックは変わりません。
- **ユーザー音色を保存**で選択音符（未選択ならトラックの入力音色）を独立した `.gbpreset` に保存できます。ファイル名が音色名になります。CH3は32サンプルの実波形を含みます。標準の保存先はユーザー設定の `gbseq/presets` で、この場所のファイルは次回も一覧に表示されます。別フォルダのファイルは**ユーザー音色を開く**で一覧末尾へ追加し、選んで適用してください。対応するCHでのみ選択できます。
- CH3の128種類は**選べるライブラリの数**です。LINK16形式の上限（曲内で同時に割り当て可能な波形・レベルの組み合わせ16スロット、ユーザー波形64個）は変えていません。使用中のスロットを別の音色で壊すことはありません。
- **編集**から選択音符を半音またはオクターブ単位で移調できます。「曲全体を半音上げる／下げる」は全プレイヤーのCH1～CH3に適用し、CH4は維持します。音域外になる場合は変更しません。

## English

File → Import MIDI converts an SMF to an unsaved editable project. Export → Export MIDI writes all active lanes (including muted ones) to a format-1 SMF with a tempo track and a named track per lane. Save the original GB instruments, waves and loop mode in the native project.

Open **View → Keyboard / MIDI (K)**. Select the destination editor track. Play the seven white keys with **A S D F G H J**, the five black keys with **W E T Y U**, change octave with **Z/X**, or play with the mouse. Click **IN / OUT** to cycle available ports back to Off. IN listens on all channels, C2–B5, velocity-sensitive; OUT plays the song, not MIDI Thru. Listen does not edit; Step input advances by the chosen grid (1, 2 or 4 sixteenths). Rest advances silently. REC starts a one-pass take from the beginning with the backing track; releases determine duration. Stop finalizes the take; Undo reverts it. Protect rejects collisions; Overwrite replaces just the covered interval. The preference survives restart.

The instrument dropdown offers 128 original patches for each physical voice type, independently selectable per track/player. Its footer/wheel pages the list; click a row or press 1–8. Applying changes selected notes on that track and its default for future input. Save user preset captures the selected note/current patch as a portable `.gbpreset`, including the actual custom CH3 wave. The per-user presets directory is scanned when opening the dropdown; use Load user preset for other locations. LINK16 projects retain 16 wave/level slots and up to 64 custom waves; occupied slots are protected. Edit offers semitone and octave transpose for the selection, or semitone key changes for the entire pitched arrangement without moving percussion.

## File conversion and routing

**コード進行の表示 / Chord display:** 「表示 → コード進行の表示」で「全体のコード」と各 P1～P4 / CH のラベルを独立して切り替えます。「各トラック一括」は個別ラベルを一括オン／オフにします。全体はルーラー下、個別の推定は各レーン上に表示します。解析は各拍の音高と長さから行い、単音／二音では半小節・小節内の分散和音も調べます。三和音、7th、sus、6thなどが候補です。CH4・無音・根拠の乏しい部分は空欄です。推定結果であり、転調・異名同音・経過音など作曲意図を常に特定できるものではありません。音符自体や出力ROMには変更を加えません。

**View → Chord display** independently toggles the combined chord strip and each player's individual lane labels. The bulk switch controls all lane labels, not the combined strip. Labels estimate harmony from duration-weighted pitch classes on each beat, with half-bar/bar arpeggio context for sparse passages. Noise, silence and insufficient evidence produce no symbol. Zoom in to read faster changes. These are editor annotations, not changes to the score or exported audio.

```text
gbseq-cli import-midi input.mid output.gbseq
gbseq-cli export-midi song.gbseq output.mid
```

- Input: SMF types 0 and 1, metrical/PPQN timing, up to 256 four-beat bars. Nearest sixteenth; rounded notes last at least one step. Octave-fold out-of-range file pitches (live input instead ignores them). The conversion report notes folding, tempo changes, clamping to 30–300 BPM and non-4/4 meters. Only the initial tempo is used; controllers, sustain pedal, program changes, bends and other automation are not imported. Independent type-2 sequences and SMPTE timing are rejected.
- Untagged MIDI: give a source track/channel its previous available voice when possible. Split chords across melodic voices; GM channel 10 goes to noise. Reject excess simultaneous voices rather than dropping them. GBSEQ's own track names preserve exact lane identities and empty players across round trips. Velocity is converted between GB 1–15 and MIDI 1–127; silent GB notes are omitted.
- Output: 480 PPQN, middle C = 60, continuous 4/4 grid. P1 uses MIDI channels **1, 2, 3, 10**; P2 **5, 6, 7, 8**; P3 **9, 4, 11, 12**; P4 **13, 14, 15, 16**. Configure your external multitimbral instrument accordingly, including percussion for the additional noise lanes. MIDI files use the score BPM (or the explicit frame override); the live MIDI transport follows the GB audio clock. Live song output honors track mute/solo. Stop and loop transitions release outstanding notes. The input voice is separate from the backing audio.
