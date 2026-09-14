# KITAQGB 라이브러리

[English](README.md) | [日本語](README.ja.md) | **한국어**

[KITAQGB 라이브러리 한국어 설명서](https://bartaro.github.io/kitaq-docs/ko/gb-library.html)에서 각 함수의 사용법과 예제 코드를 볼 수 있습니다.

`wire3d_dmg`는 게임보이용 흑백 와이어프레임 렌더러입니다. 128×96에서는 `wire3d_dmg_96.c`, 128×120에서는 `wire3d_dmg.c`를 선택하고 `Wire3DDMG_*` 함수를 사용하세요. `wire3d`와 `dmg3d`는 각각의 해상도에 대응하는 호환용 진입점으로 남아 있습니다. 한 ROM에는 진입점을 하나만 컴파일하세요. 컬러 전용 렌더러 `wire3d_cgb`는 별도로 사용합니다.

[렌더러 가이드](wire3d_dmg_guide.md) / [일본어 가이드](wire3d_dmg_guide_ja.md)

이 폴더의 파일은 세 종류로 나뉩니다.

- 공개 API: 게임에서 헤더를 포함하고 필요한 소스와 함께 빌드하는 재사용 라이브러리입니다.
- 보조 파일: 선택 기능, 레지스터 선언, 빈 소스 등입니다. 각각이 독립적인 공개 API는 아닙니다.
- 참고 문서: 사용법을 설명하는 자료이며 ROM 빌드에 넣지 않습니다.

## 파일 분류

### 공개 API

| 파일 | 기능 | 기본 사용법 |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 축에 평행한 사각형(AABB)의 2D 물리, 중력 적분, 반복 계산을 통한 접촉 해결. | `physics2d.h`를 포함하고 `physics2d.c`를 함께 컴파일합니다. |
| `physics2d_circle.h` / `physics2d_circle.c` | 공을 사용하는 게임에 맞춘 원형 물체의 2D 물리. | 헤더를 포함하고 소스를 함께 컴파일합니다. |
| `physics3d.h` / `physics3d.c` | 가속도, 질량에 따른 반발, 파괴 플래그를 갖춘 3D AABB 물리와 `kq3d_dot_q8_8()`. | 헤더를 포함하고 소스를 함께 컴파일합니다. |
| `wire3d.h` / `wire3d.c` | 고정소수점 3D 와이어프레임 렌더러. WRAM 작업 버퍼, 모델의 숨은 선 처리, 장면 가림 마스크를 지원합니다. | `wire3d.h`를 포함하고 `wire3d.c`를 컴파일합니다. |
| `dmg3d.h` / `dmg3d.c` | D000의 128×120 버퍼를 쓰는 DMG 렌더러. 정해진 순서의 인라인 어셈블리 선 그리기와 STAT을 확인하는 D000→8900 전송. | 1bpp 작업 버퍼가 필요한 경우 헤더와 소스를 포함합니다. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB 전용 8MHz 컬러 렌더러. 2bpp WRAM 버퍼, 숨은 선·장면 가림 처리, 클리핑 어셈블리, HBlank DMA를 통한 화면 갱신. | CGB 전용 ROM으로 빌드하고 헤더와 소스를 포함합니다. |
| `system.h` / `system.c` | 초기화, 프레임 수, VBlank 대기, 협조적인 VBlank 콜백, DI/EI 래퍼. | 프레임 단위의 게임 루프에 사용합니다. |
| `input.h` / `input.c` | 버튼의 누른 상태, 누른 순간, 뗀 순간, 반복 입력을 프레임별로 관리합니다. | 메뉴, 액션, 퍼즐, 전략 게임의 조작에 사용합니다. |
| `vram.h` / `vram.c` | BG 타일 쓰기, 사각형 채우기, 맵 블록, memcpy, memset을 예약하는 VRAM 명령 큐. | 게임 처리 중 갱신을 예약하고, 안전한 시간에 `vram_flush()` 또는 `vram_flush_now()`로 전송합니다. |
| `sprite.h` / `sprite.c` | OAM 작업용 복사본, 스프라이트 할당, 메타스프라이트, 애니메이션, OAM DMA, 주사선별 개수 초과 확인. | OBJ를 이용한 렌더링에 사용합니다. |
| `fixed.h` / `fixed.c` | Q8.8 고정소수점, `Vec2`, `KQRect`, clamp/min/max/lerp, 기본 사각형 판정. | 이동, 물리, 카메라, AI 평가값 등에 사용합니다. |
| `scene.h` / `scene.c` | 타이틀·게임·일시정지 같은 장면 표와 전환·갱신·그리기 호출 분배. | 게임 상태의 흐름을 구성할 때 사용합니다. |
| `entity.h` / `entity.c` | 최대 `ENTITY_MAX`개의 작은 게임 개체를 고정 배열로 관리하는 풀. | 콜백에는 개체 ID가 전달됩니다. `entity_get(id)`로 실제 데이터를 얻습니다. |
| `danmaku.h` / `danmaku.c` | 고정소수점 탄환 96개 풀, 32방향 부채꼴 탄막, 명중·스침 이벤트, OAM 개수 제한에 묶이지 않는 CGB BG 타일 합성. | 헤더와 소스를 포함하고 `danmaku_guide.md`와 완성 게임 `ressen_gbc`를 참고하세요. |
| `bank.h` / `bank.c` | 컴파일러 내장 연산 위에서 먼 뱅크의 데이터·포인터·함수 호출과 간단한 MBC 뱅크 전환을 지원합니다. | 뱅크를 넘는 데이터 접근에 사용합니다. |
| `asset.h` / `asset.c` | 자료 ID별 설명자 표와 원시 데이터·타일 로드. | 헤더와 소스를 포함합니다. 향후 `assets.h/c/json` 생성 도구의 출력 형식으로도 사용할 수 있습니다. |
| `debug.h` / `debug.c` | KOKURA 등에서 확인하는 ROM 내부의 작은 트레이스·단언·표식 버퍼. | 무거운 프로파일링은 ROM 밖에서 처리합니다. |
| `chain.h` / `chain.c` | 뱀, 밧줄, 열차, 관절 스프라이트 등에 쓰는 좌표 이력 링 버퍼. | 뒤쪽 조각이 과거 위치를 따라 움직이게 할 때 사용합니다. |
| `cgb_tile.h` | CGB 타일·속성 관련 컴파일러 내장 연산의 공개 선언. | `__settile...` 같은 함수를 쓰는 게임 소스에서 포함합니다. |
| `cgb_palette.h` / `cgb_palette.c` | CGB BG/OBJ 팔레트를 다루는 상위 API. | 헤더를 포함하고 소스를 함께 컴파일합니다. |
| `scroll.h` / `scroll.c` | 내장 연산을 이용한 스크롤과 화면 분할 테이블. | `Scroll_*` 함수를 사용할 때 포함합니다. |
| `raster.h` / `raster.c` | 띠 단위의 래스터 스크롤과 주사선별 X 방향 변형 패턴. | `raster.h`를 포함하고 `raster.c`와 `scroll.c`를 컴파일합니다. |
| `camera.h` / `camera.c` | `scroll.*` 기반의 8.8 고정소수점 카메라, 간단한 전역 함수, 월드·화면 좌표 변환. | 헤더를 포함하고 소스를 함께 컴파일합니다. |
| `audio.h` / `audio.c` | 음악, 효과음, 패닝, 파형, 페이드를 다루는 공통 오디오 드라이버. 음표 번호 67(`G6`)까지 68개 번호를 지원합니다. | 오디오가 필요한 프로젝트에 포함합니다. |
| `audio_vblank.h` / `audio_vblank.c` | 같은 68개 음표 번호를 쓰는 VBlank IRQ BGM 드라이버. 뱅크에 배치한 곡을 위한 WRAM 큐 16레코드와 선택적 프레임 훅. | 직접 포인터로 읽는 곡은 고정 뱅크 0에 두고, 다른 뱅크의 곡은 큐를 보충하며 재생합니다. `scripts/patch_gb_vblank_irq.ps1`로 벡터 `0x0040`을 설정합니다. |
| `link.h` / `link.c` | 통신 케이블용 바이트 전송과 협조적인 논리 4인 통신 `Link4_*`. | 통신 기능이 필요한 프로젝트에 포함합니다. |
| `link_packet.c` | `link.c` 위에 추가하는 패킷 계층과 `Link4_*`의 상대별 수신함. | 패킷 송수신이 필요할 때만 `link.c`와 함께 컴파일합니다. |
| `link_dmg07.h` / `link_dmg07.c` | 실물 Nintendo DMG-07 Four Player Adapter용 외부 클록 폴링 드라이버. | `link_hwregs_gb.c`와 함께 컴파일합니다. 논리 API `Link4_*`와는 별개입니다. |
| `rpg.h` | RPG·ADV·SLG의 공통 선언과 저수준 내장 연산 선언. | 이 기능군을 사용하는 게임 소스에서 포함합니다. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, `rng_seed`, `rng_next8`, `rng_next16`, `rng_range`, `rng_chance`. | `rpg.h`의 난수 함수를 사용할 때 컴파일합니다. |
| `flags.c` | 플래그 2048개를 관리하는 비트 집합과 퀘스트 상태 저장. | `rpg.h`의 플래그·퀘스트 기능에 사용합니다. |
| `rle.c` | RAM이나 먼 ROM의 `[개수][값]` 형식 RLE를 해제합니다. | `rpg.h`의 `rle_decode*` 함수에 사용합니다. |
| `text.c` | 타일 문자열 창, 페이지 대기, 선택지, XY 지정 출력, 숫자 출력, 지우기·창 조작의 별칭. | `rpg.h`의 텍스트 기능에 사용합니다. |
| `menu.c` | 세로 메뉴, 최소한의 인벤토리 메뉴, 메인 루프를 멈추지 않는 메뉴 상태 API. | `rpg.h`의 메뉴 기능에 사용합니다. |
| `script.c` | RPG·ADV 진행을 위한 작은 바이트코드 실행기. | `rpg.h`의 스크립트 기능에 사용합니다. |
| `map.c` | 묶음 맵 로드, 충돌·트리거·카메라, 선택적인 16×16 메타타일 지원. | `rpg.h`의 맵 기능에 사용합니다. |
| `save.c` | 헤더, 버전, 길이, 체크섬을 갖춘 MBC5 방식 SRAM 저장·로드·확인·지우기. | `rpg.h`의 저장 기능에 사용합니다. |
| `slg_unit.c` | 전략 게임의 이동·공격 범위. | `rpg.h`의 전술 개체 기능에 사용합니다. |
| `slg_path.c` | 너비 우선 탐색과 이동 비용에 따른 도달 영역 계산. | `rpg.h`의 경로 기능에 사용합니다. |
| `slg.h` / `slg_board.c` | 보드게임·전술 시스템용 보드, 수 목록, 되돌리기 스택. | `slg.h`와 `slg_board.c`를 포함합니다. 게임 고유의 평가 함수는 별도로 작성합니다. |

### 보조 파일

| 파일 | 기능 | 참고 사항 |
| --- | --- | --- |
| `audio_hwregs_gb.c` | APU와 파형 RAM 레지스터의 최소 선언. | 다른 파일에서 같은 레지스터를 선언하지 않는 경우에만 사용합니다. |
| `link_hwregs_gb.c` | 통신용 `SB`, `SC`, `IF`, `IE`의 최소 선언. | 다른 파일에서 통신 레지스터를 이미 선언했다면 중복해서 포함하지 않습니다. |
| `cgb_tile.c` | 의도적으로 비워 둔 CGB 타일 기능의 컴파일 단위. | 공개 선언은 `cgb_tile.h`에 있습니다. 컴파일해도 문제는 없지만 동작에 필요한 소스는 아닙니다. |
| `math.c` | ROM의 사인 테이블 `MATH_SIN`. | 아직 안정적인 공개 API로 문서화하지 않았습니다. 당분간 프로젝트 보조 데이터로 취급하세요. |

### 참고 문서

| 파일 | 내용 |
| --- | --- |
| `README.md` | 이 개요와 빌드 방법의 영어판. |
| `wire3d_guide_ja.md` | 와이어프레임 3D 렌더러의 일본어 입문서. |
| `dmg3d_guide_ja.md` | DMG 작업 버퍼 방식 렌더러의 일본어 입문서. |
| `wire3d_cgb_guide.md` | CGB 전용 컬러 렌더러 입문서. |
| `physics_guide.html` | 물리 라이브러리 영어 가이드. |
| `physics_guide_ja.html` | 물리 라이브러리 일본어 가이드. |

## 빌드 방법

아래 명령 일부에는 `wire3d_cube_demo.c`처럼 공개 저장소에 없는 과거 개발용 데모가 나옵니다. 해당 소스를 준비한 뒤 사용하는 빌드 틀입니다. 동봉된 입문 예제는 `../examples/build.ps1`과 HTML 설명서를 사용하세요. 짧은 명령 이름 `kitaqgb`는 실행 파일이 PATH에 등록되어 있다고 가정합니다.

게임 소스와 사용하는 라이브러리 소스를 함께 지정합니다.

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

와이어프레임 3D 프로젝트에서는 렌더러 소스를 함께 컴파일합니다.

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

동봉된 `examples/wire3d_minimal.c`는 모델의 모든 필드를 초기화하고 128×96 화면에서 정육면체를 회전시킵니다. 외부 그림이나 글꼴 자료는 필요하지 않습니다.

128×120 DMG 프로젝트는 120행 호환 진입점 `dmg3d.*`를 사용할 수 있습니다.

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()`은 D000의 WRAM 작업 버퍼와 0x8900부터 시작하는 타일 전송을 사용해 128×120 영역을 설정합니다. `DMG3D_BeginFrame()`은 가림 상태만 초기화합니다. 픽셀은 전송할 때 지워집니다. `DMG3D_EndFrame()`은 VBlank를 기다린 뒤 STAT을 확인하면서 전송하므로 VBlank가 끝난 뒤까지 처리할 수 있습니다. 동봉된 `examples/dmg3d_minimal.c`는 변경 영역 전송을 켜고 매 프레임 십자 모양을 다시 그립니다. 보조 전송은 별도 조작이며, 전송 원본 저장 공간을 주 버퍼와 공유합니다.

CGB 전용 컬러 프로젝트에서는 `wire3d_cgb.*`를 사용합니다.

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()`은 CGB를 배속 모드로 바꾸고 128×96·2bpp BG 영역과 기본 4색 팔레트를 설정합니다. 일반 프레임 API와 `Fast` API 모두 화면 중간 갱신으로 인한 찢어짐을 피하는 표시 방식을 씁니다. `0xD300–0xDEFF`의 3072바이트를 HBlank DMA로 비표시 VRAM 타일 뱅크에 보내고, VBlank 중에 표시를 바꿉니다. 매 프레임 LCDC를 바꿀 필요는 없습니다. `Fast` API는 일반 프레임 종료 시 수행하는 BG 큐 검사를 생략합니다. 색은 `Wire3DCGB_SetPaletteRGB15()`, `Wire3DCGB_SetLineColor()`, `Wire3DCGB_Draw*Color()`로 지정합니다.

CAD에서 방향별로 생성한 LOD 자료에는 `Wire3DCGB_DrawMaskedModel2D()`를 사용할 수 있습니다. 이미 투영된 부호 있는 꼭짓점 오프셋과 표시할 모서리의 압축 비트마스크를 전달합니다. 모서리 순회와 어셈블리 래스터라이저는 렌더러 뱅크 4 안에서 실행되므로 선마다 뱅크를 넘나드는 호출을 하지 않습니다.

변경 부분이 적고 OAM 작업용 복사본도 쓰는 게임은 VBlank에 들어간 뒤 `sprite_flush_oam()`, `Wire3DCGB_EndFrameSparseNow()` 순서로 호출할 수 있습니다. 처음의 ‘다음 VBlank 대기’를 생략하지만, 변경 범위와 현재 주사선에 따라 DMA나 표시 전환에서 기다릴 수 있습니다. 모든 작업이 같은 VBlank 안에 끝난다는 보장은 없습니다.

CGB 선의 색은 1·2·3을 사용하세요. 일반 128×96 모드는 색 비트를 합치므로 1과 2가 겹치면 3이 됩니다. 색 0으로 선을 지울 수는 없습니다. 프레임 전체를 지우거나 전용 지우기 함수를 사용하세요. 일반 `Wire3DCGB_DrawLine2D`와 모델 그리기는 부분 전송용 변경 범위를 기록하지 않습니다. 범위 기록이 필요한 선은 `Wire3DCGB_DrawLineClipped2D`를 사용하거나, `Wire3DCGB_InvalidateFrameHistory`를 호출해 다음 부분 전송에 전체 화면을 포함하세요.

160×144 모드는 프레임당 최대 127개 타일을 할당합니다. 빠른 선 그리기 경로에서 할당 실패나 범위 밖 좌표가 발생하면 `Wire3DCGB_GetFullScreenOverflow()`가 설정되며, 다음 프레임 초기화까지 픽셀 쓰기를 중단합니다. 꼭짓점은 선택한 화면 안에 두세요. 삼각형 마스크의 여백은 128×96에서 X=127, 전체 화면에서 X=159까지입니다. 특히 전체 화면과 FastMap을 사용할 때는 API에 설명된 WRAM 뱅크 매핑 조건을 지켜야 합니다.

두 모드의 경계를 확인하는 완성 프로그램은 [CGB 삼각형 마스크 경계 회귀 테스트](../tests/library/wire3d_cgb_mask_bounds.c)에 있습니다.

과거 개발용 `examples/wire3d_cgb_hiddenline_demo.c`는 조작 가능한 숨은 선 테스트입니다. `START`로 표시 수를 1~3개로 바꾸고 `B`로 대상을 선택합니다. 방향키는 X/Y 이동, `A`+위/아래는 Z 이동, `A`+왼쪽/오른쪽은 Z 회전, `SELECT`+방향키는 X/Y축의 22.5도 회전입니다. 숨은 선·물체 간 가림 처리는 항상 켜져 있고, 충돌한 물체는 서로 밀려납니다. 이 개발용 데모는 공개 저장소에 포함되지 않습니다.

RPG·ADV·SLG 기능의 빌드 예입니다.

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

표준 런타임 기본 동작 확인용 빌드 예입니다.

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

시리얼 통신에서는 통신 코어보다 먼저 하드웨어 레지스터 정의 파일을 지정합니다.

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

호스트가 통신 상대를 선택하는 협조적 논리 4인 통신도 같은 파일을 사용합니다. 호스트는 `Link4_InitHost(slot_count)`로 초기화하고 `Link4_SelectPeer()` 또는 `Link4_SendPacketTo()`로 상대를 선택합니다. 다른 참가자는 `Link4_InitPeer(local_slot, slot_count)`로 초기화하고 슬롯 0의 호스트와 통신합니다.

슬롯 번호를 미리 지정한 참가자용 래퍼도 직접 빌드할 수 있습니다.

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

실물 DMG-07에는 전용 폴링 드라이버를 사용합니다.

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

`LinkDmg07_Poll()`은 계속 반복 호출하세요. 어댑터의 바이트 전송 간격은 비디오 한 프레임보다 훨씬 짧습니다. `LinkDmg07_TickFrame()`은 VBlank마다 한 번 호출해 무응답·연결 절차 대기용 포화 카운터를 갱신합니다. 드라이버는 항상 외부 클록 `SC=$80`을 설정하고, 연결 확인에는 `88 88 RATE 01`로 응답합니다. 물리 플레이어 1만 `AA AA AA AA`로 전송을 요청할 수 있습니다.

모든 기기가 `CC CC CC CC`를 받으면 각 물리 슬롯의 한 바이트씩을 묶은 4바이트 방송 패킷을 사용합니다. 제출한 데이터는 다음 패킷에서 방송되므로, 드라이버는 처음의 미정의 패킷을 버리고 송수신 순서 번호를 제공합니다. `LinkDmg07_RequestRestart()`는 다음 패킷 경계를 기다려 정렬된 `FF FF FF FF`를 보내고, 어댑터에서 네 바이트가 모두 FF인 표시를 받으면 멈춥니다. 전송 단계의 무응답 제한 시간이 지나도 현재 4바이트 안의 위치를 유지한 채 이 재시작을 예약합니다. 클록이 돌아오면 진행 중인 패킷을 마치고 복구 단계로 넘어갈 수 있습니다.

게임 코드에서는 필요한 헤더를 포함합니다.

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

## 사용 시 참고 사항

마지막 8개 음표 번호는 현재 이전 옥타브의 주파수를 재사용합니다. 68개 번호가 있다는 것이 서로 다른 음높이 68개를 보장하는 것은 아닙니다.

- `inv_mass_q8 == 0`은 움직이지 않는 물체를 나타냅니다.
- `Wire3D_Init()`은 128×96 BG 영역, `0xD000`부터의 WRAM 버퍼, `0x8900`부터의 타일 데이터를 사용합니다. `--stack-bank=fixed`로 빌드하세요.
- `Wire3D_BeginFrame()`은 WRAM 버퍼를 지우고, `Wire3D_EndFrame()`은 VBlank를 기다린 뒤 STAT을 확인하면서 VRAM에 묶음 전송합니다.
- Wire3D 각도는 16단계입니다. 기본 모델 경로는 모델당 최대 `WIRE3D_MODEL_VERTEX_LIMIT`개의 꼭짓점을 지원합니다.
- 면이 있는 물체를 선으로 표현하고 여러 물체가 겹친다면 `Wire3D_DrawScene()`을 쓰세요. 가까운 물체부터 그리고 면 마스크를 누적해 먼 선을 보수적으로 가립니다.
- `Wire3DCGB_Init()`은 CGB 전용이며 KEY1/STOP으로 배속 전환을 합니다. `--cgb=cgb_only`로 빌드하고, DMG 호환 `wire3d.*`와 같은 ROM에 넣지 마세요.
- 일반·`Fast` 프레임 API는 비표시 VRAM 뱅크 전송이 끝날 때까지 이전 프레임을 유지합니다. BG 타일 쓰기 큐가 없는 장면에는 `Fast`가 적합합니다.
- `NR10..NR52`와 `WAVE0..WAVE15`를 선언하는 파일은 `lib/audio.c`보다 먼저 컴파일하세요.
- `lib/audio_hwregs_gb.c`가 해당 선언을 제공합니다. 같은 오디오 레지스터를 선언하는 다른 파일과 중복해서 포함하지 마세요.
- `cgb_tile.h`는 내장 연산을 직접 공개합니다. `lib/cgb_tile.c`는 빈 파일이므로 일반 빌드에서 생략할 수 있습니다.
- `cgb_palette.h`의 공개 API 이름은 `cgb_*`입니다.
- 메뉴나 설정에서 음악·효과음 사용 여부를 바꾸면 `Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()`를 호출하세요.
- `Audio_PlaySFX()`는 호출 시 보이는 ROM 뱅크를 기록합니다. 효과음 데이터 뱅크를 명확히 알고 있다면 `Audio_PlaySFXBanked(bank, sfx, priority)`를 사용하세요.
- 음악 스트림의 `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST`는 기존 채널 번호를 유지합니다. `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3`입니다.
- CH3의 사용자 파형은 4비트 샘플 32개를 16바이트에 담아 `Audio_LoadCustomWave()`로 전달하세요.
- `Audio_FadeToMasterVolume()`의 페이드는 `Audio_Update()`에서 진행됩니다. 페이드 중에도 매 프레임 호출하세요.
- `audio_vblank.c`는 VBlank IRQ 벡터 심볼 `__kq_vblank_vector`를 정의합니다. BGM 이벤트는 `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`의 5바이트이며, 쉼·반복·끝에는 `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP`, `AUDIO_VBLANK_END`를 사용합니다.
- 직접 포인터로 읽는 VBlank 곡은 고정 뱅크 데이터여야 합니다. 큐 모드에서는 다른 뱅크의 곡으로 보충할 수 있습니다. `lib/audio_vblank.c`를 링크한 뒤 `scripts/patch_gb_vblank_irq.ps1 <rom> <map>`을 실행해 벡터 `0x0040`을 ISR 점프로 바꾸고 ROM 체크섬을 갱신하세요.
- VBlank 벡터 `0x0040`을 소유하는 다른 라이브러리나 게임 스텁과 `audio_vblank.c`를 함께 쓰려면 공용 IRQ 분배 처리가 필요합니다.
- `Scroll_SplitCommit()`은 IE 비트 `0x01 | 0x02`를 자동으로 켜고, 컴파일러의 VBlank/STAT 핸들러로 화면 분할을 재생합니다.
- 화면 분할 기능은 해당 빌드에서 벡터 `0x0040`과 `0x0048`을 예약합니다. 현재는 별도 사용자 VBlank/STAT 스텁과 함께 쓰지 마세요.
- `SB`, `SC`, `IF`, `IE` 선언 파일은 `lib/link.c` / `lib/link_packet.c` 또는 `lib/link_dmg07.c`보다 먼저 컴파일하세요.
- `lib/link_hwregs_gb.c`가 해당 선언을 제공합니다. 같은 시리얼 레지스터를 선언하는 다른 파일과 중복해서 넣지 마세요.
- 통신 라이브러리는 시리얼 벡터 `0x0058`을 정의하지 않습니다. 인터럽트 모드에서는 자신의 IRQ 스텁이나 분배 처리에서 `Link_OnSerialIRQ()`를 호출하세요.
- 패킷 계층은 한 건 깊이의 수신 저장 공간을 사용합니다. 프레임 단위 메인 루프에서 자주 처리하는 방식이 적합합니다.
- `Link4_*`는 호스트가 대상을 선택하는 협조적 4인 통신입니다. 한 번에 한 상대만 선로를 사용하므로 호스트가 순서대로 바꿔야 합니다.
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()`은 상대별 수신함을 제공합니다. 여러 상대를 확인해도 데이터가 누구에게서 왔는지 유지됩니다.
- `Link_ReadPacket()`은 기존의 ‘가장 최근 패킷’을 읽는 API입니다. 4인 통신에는 `Link4_ReadPacketFrom()`을 쓰세요.
- `Link4_*`는 Nintendo DMG-07의 전기적 동작이나 프로토콜을 구현하지 않습니다. 실물에는 `link_dmg07.c`를 쓰고 같은 ROM에 `link.c`와 함께 넣지 마세요.
- DMG-07의 `GetConnectedMask()`는 물리 플레이어 1~4를 비트 0~3으로 나타냅니다. 전송 중에는 마지막 연결 확인 값을 유지하며, 참가자 목록은 연결 확인 단계에서만 갱신할 수 있습니다.
- DMG-07 재시작 요청은 어댑터 클록이 없는 동안 진행되지 않습니다. 복구용 통신은 일반 데이터로 노출하지 않고 버립니다. 단순히 클록이 멈춘 것이 아니라 전원을 껐다 켜서 다른 단계가 되었다면 드라이버와 세션을 명시적으로 다시 초기화하세요.
- 물리 라이브러리는 선형 위치·속도만 다루며 회전 운동 역학은 다루지 않습니다.
- 게임보이급 하드웨어에서는 활성 물체 수를 적게 유지하세요. 예를 들면 8~24개 정도입니다.
- 게임에 맞춰 월드별 중력, 최대 속도, 접촉 해결 반복 횟수를 조절하세요.
- 당구 같은 게임에는 AABB보다 `physics2d_circle.*`가 적합합니다.
- 현재 구성에서는 별도의 `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game`, `simple_physics` 라이브러리를 추가하지 마세요. 각각 `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg`, `physics2d`를 사용합니다.
- `scene.c`와 `entity.c`는 함수 포인터 호출에 포인터 크기의 인수를 넘기지 않습니다. 현재 KITAQGB의 함수 포인터 호출 경로는 인수가 없거나 1바이트 ID를 전달할 때 가장 안정적입니다.
