# GBSEQ — 사용 설명서

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

DAISUKE OBA의 GBSEQ는 plita GUI와 독립 CLI를 제공하는 게임보이 시퀀서입니다. 편집 모델, 검증, KOKURA 음원과 내보내기를 공유합니다. LOCAL 4CH / CABLE 8CH / DMG07 16CH는 1·2·4대를 지원합니다. 각 기기의 CH1·CH2는 펄스, CH3은 파형, CH4는 노이즈입니다. 곡은 1–256마디, 마디당 16스텝(채널당 최대 4096), BPM 30–300이며 반복 여부를 저장합니다. GUI 표시는 영어이고 설명서는 9개 언어입니다.

## 설치와 실행

[bin/](bin/)에서 Windows x86_64, Linux x86_64, macOS Apple Silicon 또는 Intel 패키지를 선택하여 전체를 풉니다. assets와 라이선스 파일도 실행 파일 옆에 둡니다. GUI는 Windows에서 gbseq.exe, Unix에서 ./gbseq입니다. CLI 예제의 gbseq-cli는 PowerShell에서 ./gbseq-cli.exe, Unix에서 ./gbseq-cli로 실행합니다(PATH에 등록했다면 접두사 불필요). 일반 사용에는 컴파일러·별도 plita·Python이 필요 없습니다.

GUI에는 데스크톱, 호환 GPU 드라이버, 미리 듣기용 오디오 장치가 필요합니다. Linux는 Ubuntu 24.04 이후 호환 환경과 ALSA, X11/Wayland, libxkbcommon + libxkbcommon-x11, Vulkan/OpenGL 라이브러리를 사용합니다. macOS 바이너리는 서명·공증되지 않았습니다. 출처와 해시를 확인하고 필요하면 시스템의 열기/개인정보 보호 및 보안에서 해당 앱만 허용하세요. 전역 보호 기능을 끄지 마세요. CLI의 WAV 생성에는 화면이나 음성 장치가 필요 없습니다. Unix 실행 권한이 빠졌다면 chmod +x gbseq gbseq-cli를 사용합니다.

## GUI 편집

연결 모드와 P1–P4를 선택합니다. DMG07의 ALL16은 모든 채널을 동시에 표시·재생합니다. 빈 레인을 두 번 클릭하면 음표를 만들고, 음표를 두 번 클릭하면 상세 편집기를 엽니다. 몸체를 끌어 위치·음높이, 오른쪽 끝을 끌어 길이를 변경합니다. 레인마다 단음이므로 겹침은 거부됩니다. 마디 수는 1–256입니다. 휠·아래 스크롤바로 이동, F로 전체 보기, Home으로 처음, +/-로 확대합니다. 재생 화면은 현재 위치를 따라갑니다. Space는 재생/정지, LOOP ON/OFF는 반복입니다. 첫 재생은 곡 전체를 렌더링하므로 긴 곡은 준비 시간이 필요합니다.

Ctrl/Shift 클릭으로 선택을 전환하고 Ctrl+A로 현재 레인을 선택합니다. 좌우는 그리드 이동, 상하는 반음, Shift+상하는 옥타브 변경입니다. Ctrl+D 복제, Delete 삭제, G 그리드 변경, Q 시작 정렬, Ctrl+Z/Y 실행 취소/다시 실행, Ctrl+C/V 복사/붙여넣기. Ctrl+O 열기, Ctrl+S 저장, Ctrl+Shift+S 다른 이름 저장. 저장하지 않은 편집을 교체하기 전에 확인하며, 덮어쓰기 전 원본을 보관하세요.

## 음색과 CH3

음표마다 음량·좌/우/중앙 팬, 펄스 듀티·엔벌로프, CH1 스위프, CH3 레벨·프리셋, CH4 노이즈를 설정합니다. AUDITION은 선택 음표를 미리 듣습니다. CH3에는 하드웨어 엔벌로프가 없습니다. NEW USER WAVE에서 32개 4비트 샘플(0–15)을 드래그하여 그립니다. 사용자 파형 64개와 기본 파형 7개를 16개 활성 프리셋에 배정하며 레벨은 100/50/25%입니다. 음표별 WAVE PRESET으로 음색을 바꿉니다. 실제 파형 교체는 CH3을 잠시 끄므로 연속 크로스페이드가 아닙니다.

## CLI

track/player는 1부터, start는 0부터 계산합니다. length 1은 16분음표, 16은 4/4 한 마디입니다. pitch 0–47은 C2–B5, velocity는 0–15입니다. players는 1·2·4만 지원합니다. 편집은 --out이 필수이며 명시한 기존 파일을 덮어쓸 수 있습니다. 잘못된 편집은 0이 아닌 종료 코드로 실패하고 저장되지 않습니다. transform은 --shift, --quantize, --velocity도 받습니다. --all-players는 전체 믹스, 기본은 P1입니다. 내보내기는 기존 출력을 교체할 수 있습니다.

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


## 내보내기

LINK16 SAV는 V4, 128 KiB MBC5 배터리 이미지입니다. 이름은 link16_local.sav, link16_cable.p1.sav/.p2.sav, link16_dmg07.p1.sav–.p4.sav입니다. 대응하는 LINK16 ROM과 에뮬레이터 저장 이름을 사용하고 기존 SAV를 백업하세요. P1에는 다른 기기 파트의 미러도 들어갑니다. 이 배포에는 LINK16 ROM이 없으며 SAV 자체는 실행 ROM이 아닙니다. 엔벌로프는 기기마다 공유 프리셋 4개로 제한되어 초과 값은 근사될 수 있습니다. 가져온 뒤 소리를 확인하세요. V4는 CH1 스위프를 저장하지 않습니다. frames_per_step_override의 프레임 단위 효과음은 미리 듣기/WAV/KITAQGB에만 지원되고 BPM 기반 LINK16에서는 거부됩니다.

KITAQGB는 플레이어별 C/스트림 데이터를 내보냅니다. CH1=0, CH2=1, CH3=2, CH4=3의 호환 audio.h/audio.c를 사용하세요. 16채널을 한 게임보이에 합치는 기능은 아닙니다. GBSEQ 미리 듣기는 스텝 길이를 비디오 프레임으로 반올림하므로 일부 BPM은 분수 클록 ROM과 다를 수 있습니다. 에뮬레이터 검증은 실기 검증이 아닙니다.

## 데이터와 Python

.gbseq는 JSON이며 MIDI가 아닙니다. 확장자 변경만으로 변환되지 않습니다. AI가 만든 데이터는 validate로 검사하고 길이·활성 채널·고유 ID·음역·겹침을 확인하세요. [상세 사양](docs/README.md)은 일본어입니다. 선택적 Python 3.10+ 연동은 python/을 PYTHONPATH에 추가하고 GBSEQ_NATIVE_LIBRARY를 동봉 gbseq.dll/libgbseq.dylib/libgbseq.so의 절대 경로로 설정합니다. Python 인덱스는 0부터이며 직접 편집 후 validate를 호출합니다. player=None은 전체 믹스입니다.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## 빌드·문제 해결·라이선스

Rust 1.93.1과 OS C/C++ 도구가 필요합니다. Linux 빌드는 libasound2-dev, libudev-dev, libxkbcommon-dev, libxkbcommon-x11-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev 및 Mesa 개발 라이브러리를 사용합니다. CI는 각 OS에서 테스트·오디오/저장 내보내기·GUI 시작을 검사하고 verification에 소스 커밋을 기록합니다. 실제 오디오 출력·물리 입력·게임보이 하드웨어는 자동 검사의 대상이 아닙니다.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


실행 오류는 그래픽 드라이버와 터미널 메시지, 글꼴 문제는 assets/fonts, 무음은 출력 장치와 mute/solo를 확인하세요. 내보내기 오류는 validate의 해당 음표를 조사합니다. GBSEQ 본체는 MIT, Copyright (c) 2026 DAISUKE OBA입니다. [LICENSE](LICENSE), [제3자 고지](THIRD_PARTY_NOTICES.md), [감사 목록](LICENSE_AUDIT.json)을 확인하고 재배포 시 모든 고지와 글꼴 라이선스를 유지하세요. 외부 소재와 가져온 음악은 각각의 권리를 유지합니다.
