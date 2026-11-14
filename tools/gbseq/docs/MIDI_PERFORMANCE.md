# GBSEQ — MIDI, performance keyboard and instruments

## 日本語

**ファイル → MIDIを読み込み**で新しい編集用の曲に変換し、**出力 → MIDIを書き出し**で全プレイヤーの有効トラックを1つの標準MIDIファイルに書き出します。読み込み後の「保存」は別の `.gbseq` を作ります。MIDIは演奏情報の交換用で、GBのデューティ・スイープ・CH3波形・ループ設定を保存する形式ではありません。原本は `.gbseq` で保存してください。

編集したいトラックを選び、**左上の鍵盤アイコン**または **Ctrl+K** で演奏鍵盤を開きます。「表示 → 演奏鍵盤 / MIDI」からも開けます。パネル上端で入力先のP番号・CHと入力モードを確認してください。MIDI機器を接続しなくても、画面の鍵盤とPCキーの両方で試奏・ステップ入力・録音ができます。

鍵盤の音色は、選択中の同じトラックの音符を最優先にします。音符を選ばない場合はトラックの入力プリセットを、未設定ならカーソル付近の音符の音色を使います。音符のデューティ・エンベロープ・CH3波形／音量・パンなどを編集した結果は、次の押鍵から反映されます。PCキー、マウス、MIDIで共通です。

- 白鍵は **A S D F G H J**、黒鍵は **W E T Y U**。マウスでも弾けます。**Z / X** または **Oct − / +** でオクターブを変更します。押している間だけ発音します（音色自体の減衰は別です）。各GBトラックは単音です。
- **IN** をクリックして入力ポート、**OUT** で出力ポートを順に切り替えます。最後の次はOFFです。外部鍵盤の任意のMIDIチャンネルのノート36～83（C2～B5）を選択トラックのGB音色で試奏できます。OUTは再生中の曲を外部音源へ送ります。入力をそのまま送り返すMIDI Thruではありません。専用ケーブル、OSに認識されるUSB MIDI機器、または仮想ポートを使用してください。
- **試奏**は譜面を変更しません。**ステップ入力**では押鍵で音符を追加し、**入力長**（16分・8分・4分・2分）だけ進みます。休符は音を置かず進みます。マウスを押したまま隣の鍵へ移動すると次の音を入力します。
- **リアルタイム録音**は曲頭から1回、伴奏を鳴らしながら録音します。**録音Q**をクリックして16分・8分・4分・2分から開始位置のグリッドを選びます。押鍵の長さは16分単位で保持します（最低1ステップ・曲末まで）。録音中は「録音停止」に変わり、曲末でも停止します。録音テイクは1回のUndoで戻せます。
- 入力済みの音符は、対象を選んで **G** でグリッドを選び、**Q／クオンタイズ**で開始位置を揃えます。トラック全体なら先に **Ctrl+A**。音の長さ・音色・強さは変えません。重なる場合は選択全体を変更せず、録音用の「上書き」とは独立しています。
- 初期値の**既存音符を保護**では、重なる入力を拒否します。**上書き**にすると入力区間だけを置き換え、古い音符の前後は残ります。どちらの入力方法にも適用され、設定は次回起動時にも復元されます。
- **音色 ▾** は選択トラック専用のドロップダウンです。各CHに128種類、4系統で計512種類。下端の左右・ホイールでページ切替、その行をクリック、または1～8で決定します。選択中の同一トラックの音符と、以後の新規入力に適用します。別プレイヤー・別トラックは変わりません。
- **ユーザー音色を保存**で選択音符（未選択ならトラックの入力音色）を独立した `.gbpreset` に保存できます。ファイル名が音色名になります。CH3は32サンプルの実波形を含みます。標準の保存先はユーザー設定の `gbseq/presets` で、この場所のファイルは次回も一覧に表示されます。別フォルダのファイルは**ユーザー音色を開く**で一覧末尾へ追加し、選んで適用してください。対応するCHでのみ選択できます。
- CH3は各音色専用の32サンプル波形を使います。たとえば **001 Velvet Sub** は低域向き、**026 Hollow Clarinet** は奇数倍音中心のリード、**074 Orbit Chime** はFM風の金属音です。基本波・倍音・非対称ランプ・パルス幅や位相変調を使って作った固定波形で、演奏中にシンセのフィルターやFMが動くわけではありません。既存曲・ユーザープリセットに格納済みの波形は変わりません。
- 128種類は**選べるライブラリの数**です。LINK16の曲内では波形・レベルを16スロットに割り当て、ユーザー波形を64個まで保存できます。使用中のスロットを別の音色で壊すことはありません。
- **編集**から選択音符を半音またはオクターブ単位で移調できます。「曲全体を半音上げる／下げる」は全プレイヤーのCH1～CH3に適用し、CH4は維持します。音域外になる場合は変更しません。

## English

The performance voice uses the selected note on the current lane first, then its input preset, then the nearest preceding note at the cursor (or the first note when the cursor is earlier). Detail edits to duty, envelope, CH3 waveform/level and pan take effect on the next key press. PC keys, mouse keys and MIDI use the same instrument resolution.

File → Import MIDI converts an SMF to an unsaved editable project. Export → Export MIDI writes all active lanes (including muted ones) to a format-1 SMF with a tempo track and a named track per lane. Save the original GB instruments, waves and loop mode in the native project.

Open **View → Keyboard / MIDI (Ctrl+K)** or click the piano icon below the GBSEQ logo. Select the destination track. Play white keys with **A S D F G H J**, black keys with **W E T Y U**, change octave with **Z/X**, or play/drag across the on-screen keys. No MIDI port is needed. Listen leaves the score alone. Step input inserts a note and advances by the Step length; Rest advances silently. Record live starts a one-pass take with backing; Stop recording ends it. **REC Q** independently cycles 1/16, 1/8, 1/4 and 1/2: it snaps attacks while keeping key duration to the nearest native step (minimum one). Protect rejects collisions; Overwrite replaces only the covered interval. Undo reverts the take. For existing notes, select them (Ctrl+A for the lane), cycle the edit grid with **G**, then press **Q**. If the result overlaps, nothing changes. Click IN/OUT to cycle optional external ports; IN accepts all channels, C2–B5, and OUT plays the song, not MIDI Thru.

The instrument dropdown offers 128 original patches for each physical voice type, independently selectable per track/player. Its footer/wheel pages the list; click a row or press 1–8. Applying changes selected notes on that track and its default for future input. Save user preset captures the selected note/current patch as a portable `.gbpreset`, including the actual custom CH3 wave. The per-user presets directory is scanned when opening the dropdown; use Load user preset for other locations. LINK16 projects retain 16 wave/level slots and up to 64 custom waves; occupied slots are protected. Edit offers semitone and octave transpose for the selection, or semitone key changes for the entire pitched arrangement without moving percussion.

## File conversion and routing

CH3 uses 128 individually named, independently specified 32-sample wavetables, not numbered variants of the same instrument. Try **001 Velvet Sub**, **026 Hollow Clarinet** and **074 Orbit Chime** for bass, odd-harmonic reed and metallic colours. FM and pulse shaping describe how the static wave was designed, not runtime modulation. Existing projects and user presets retain their stored waves.

**コード進行の表示 / Chord display:** 「表示 → コード進行の表示」で「全体のコード」と各 P1～P4 / CH のラベルを独立して切り替えます。「各トラック一括」は個別ラベルを一括オン／オフにします。全体はルーラー下、個別の推定は各レーン上に表示します。解析は各拍の音高と長さから行い、単音／二音では半小節・小節内の分散和音も調べます。三和音、7th、sus、6thなどが候補です。CH4・無音・根拠の乏しい部分は空欄です。推定結果であり、転調・異名同音・経過音など作曲意図を常に特定できるものではありません。音符自体や出力ROMには変更を加えません。

**View → Chord display** independently toggles the combined chord strip and each player's individual lane labels. The bulk switch controls all lane labels, not the combined strip. Labels estimate harmony from duration-weighted pitch classes on each beat, with half-bar/bar arpeggio context for sparse passages. Noise, silence and insufficient evidence produce no symbol. Zoom in to read faster changes. These are editor annotations, not changes to the score or exported audio.

```text
gbseq-cli import-midi input.mid output.gbseq
gbseq-cli export-midi song.gbseq output.mid
```

- Input: SMF types 0 and 1, metrical/PPQN timing, up to 256 four-beat bars. Nearest sixteenth; rounded notes last at least one step. Octave-fold out-of-range file pitches (live input instead ignores them). The conversion report notes folding, tempo changes, clamping to 30–300 BPM and non-4/4 meters. Only the initial tempo is used; controllers, sustain pedal, program changes, bends and other automation are not imported. Independent type-2 sequences and SMPTE timing are rejected.
- Untagged MIDI: give a source track/channel its previous available voice when possible. Split chords across melodic voices; GM channel 10 goes to noise. Reject excess simultaneous voices rather than dropping them. GBSEQ's own track names preserve exact lane identities and empty players across round trips. Velocity is converted between GB 1–15 and MIDI 1–127; silent GB notes are omitted.
- Output: 480 PPQN, middle C = 60, continuous 4/4 grid. P1 uses MIDI channels **1, 2, 3, 10**; P2 **5, 6, 7, 8**; P3 **9, 4, 11, 12**; P4 **13, 14, 15, 16**. Configure your external multitimbral instrument accordingly, including percussion for the additional noise lanes. MIDI files use the score BPM (or the explicit frame override); the live MIDI transport follows the GB audio clock. Live song output honors track mute/solo. Stop and loop transitions release outstanding notes. The input voice is separate from the backing audio.
