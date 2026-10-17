# GBHUA

Windows 10/11 x64에는 [Microsoft Visual C++ v14 Redistributable x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)도 필요합니다. `VCRUNTIME140.dll`이 없다는 메시지가 나오면 Microsoft 공식 최신 x64 패키지를 설치하세요. 다른 사이트에서 DLL만 내려받지 마세요.

## 다운로드와 시작

[GUI 다운로드](../GUI.md)에서 운영체제에 맞는 압축 파일을 내려받아 전체를 풉니다. Windows에서는 `GBHUA.exe`를 열고 옆의 `assets` 폴더를 유지합니다. Mac에서는 `GBHUA.app`을 응용 프로그램 폴더에 복사합니다. Apple Silicon과 Intel 버전을 구분하세요. 실행에 Rust나 Python은 필요 없습니다. Developer ID 서명과 공증은 제공되지 않습니다. 공식 저장소에서 받은 파일인지 확인하고, macOS가 차단하면 시스템 설정 > 개인정보 보호 및 보안에서 이 앱만 허용하세요. 시스템 보호를 끄지 마세요.

GUI 표시는 영어이며 이 문서가 번역된 설명서입니다. Mac에서도 `Ctrl`은 Control 키입니다. 아래 빌드 명령은 저장소 루트에서 시작합니다. CLI는 기본적으로 화면이나 GPU 없이 작동합니다.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA는 PNG를 Game Boy 타일, 팔레트, 맵으로 변환합니다. 제작자: DAISUKE OBA. 자체 코드는 MIT 라이선스입니다.

## 빌드와 실행

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

시작 직후 수정하지 않은 문서에는 저장 경고가 나타나지 않으며 실제 편집 후에는 확인합니다. Windows GUI는 기본적으로 콘솔을 열지 않습니다. 진단 시 `cargo run -p gbhua --bin gbhua_gui --features gui-console`을 사용합니다.

패널 제목과 캔버스 상단을 정렬했습니다. 열기, 저장, 실행 취소/다시 실행 아이콘 위에 마우스를 올리면 이름이 표시됩니다. Graphic 위 크기 버튼으로 크기를 선택하고 Map 위 전체 보기, 빼기, 더하기로 배율을 바꿉니다. DMG/CGB는 오른쪽 위에서 전환합니다. 너비가 1180px 미만이면 왼쪽의 Tiles / Palettes 탭을 전환합니다. 휠이나 스크롤바로 팔레트와 정보를 스크롤합니다. Escape는 메뉴를 닫습니다.

1. 왼쪽 타일을 선택하고 Graphic > 8x8에서 7가지 크기 중 하나를 고릅니다. 큰 그림은 선택한 ID부터 연속 타일을 사용합니다. 색을 선택해 드래그하고 오른쪽 클릭으로 색을 가져옵니다.

2. TOOLS > FILL은 연결 영역, TILES > FILL TILE은 전체 그림을 채웁니다. FLIP X/Y, ROTATE로 변형하고 EDIT의 COPY/PASTE GRAPHIC으로 복제합니다. UNDO/REDO는 최대 64회입니다.

3. TOOLS > MAP으로 배치하고 오른쪽 클릭으로 타일과 팔레트를 가져옵니다. RECT는 3x3, FILL은 영역, ATTR는 팔레트와 X 반전, COLL/EVT는 표시 전환입니다. VIEW의 Fit/1x/2x/4x, 격자, 표시를 선택하고 휠과 스크롤바로 이동합니다.

4. BYTECODE를 클릭해 Ctrl+A, Ctrl+V로 16바이트를 붙여 넣습니다. IMPORT는 첫 8x8 타일에 적용하고 EXPORT는 텍스트를 갱신합니다. 방향키/Home/End로 이동, Shift로 선택, Ctrl+C/X로 복사/잘라내기, Backspace/Delete로 삭제합니다.

5. FILE > SAVE와 EXPORT는 저장 위치를 묻습니다. 미저장 * 상태의 열기/가져오기/종료 확인에서 Yes는 저장, No는 버리기, Cancel은 계속 편집입니다. PNG 가져오기는 프로젝트를 교체하며 Undo가 가능합니다.

6. SPRITES > NEW META로 만들고 ADD/DUP/CLEAR FRAME, PREV/NEXT FRAME으로 관리합니다. METASPRITE에서 클릭으로 부품을 추가하고 오른쪽 클릭으로 제거합니다. DMG / CGB는 저장된 팔레트를 바꾸지 않고 DMG/CGB를 비교합니다.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

전체 프로젝트는 .gbh로 저장합니다. 기존 JSON 프로젝트는 확장자만 .gbh로 변경하면 됩니다. 이전 확장자 로더는 없습니다. GBTD/GBR/GBTB 및 GBMB/GBM은 교환 형식으로 모든 정보를 보존하지 않습니다. GBMB 출력은 같은 이름의 .gbr도 생성하므로 함께 보관하십시오.

## PNG / Python / CLI

이미지 크기는 8의 배수(8..2040)여야 합니다. --size는 최근접 보간으로 크기를 바꿉니다. DMG는 4단계 명암, CGB는 RGB555 근사와 타일당 4색, 최대 8팔레트를 사용합니다. 투명 영역은 흰색과 합성됩니다. 동일 타일은 공유하며 고유 타일이 256개를 넘으면 중단합니다. 경고와 미리 보기를 확인하십시오.

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

[CLI: 한국어](../README.ko.md)

## PNG 가져오기와 문제 해결

`FILE > IMPORT PNG` 또는 PNG 끌어놓기를 사용합니다. 먼저 DMG/CGB를 선택하세요. 현재 프로젝트가 교체되므로 먼저 저장합니다. 자동 CGB 팔레트는 색상 오차를 고려해 드문 하이라이트와 어두운 윤곽을 보존합니다. Game Boy의 색상 제한은 그대로입니다. 이전 변환에서 사라진 광택은 원본 PNG를 다시 가져와야 합니다. 이미 감소된 `.gbh`에서는 복원할 수 없습니다. `--size`는 CLI 옵션입니다.

타일 선택 후 BYTECODE 안을 클릭하고 정확히 16바이트를 입력한 뒤 IMPORT를 누릅니다. EXPORT는 텍스트만 갱신합니다. 맵의 색이나 방향이 다르면 팔레트와 반전 속성을 확인하세요. 모든 편집 정보는 `.gbh`로 저장합니다. 오류는 상태 표시줄에 나타나며 대화상자 취소는 저장하지 않습니다.

macOS/Linux: `python3 -m venv .venv`, `source .venv/bin/activate`, `python -m pip install maturin`, `python -m maturin develop --features python` 순서로 실행합니다. 바인딩 빌드에는 Python 3.10 이상과 Rust가 필요합니다.
