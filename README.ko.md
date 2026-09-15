# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **한국어**

**컴파일러 한국어 설명서** · **라이브러리 한국어 설명서**

## 이름의 유래

KITAQGB는 Zachtronics와 관련된 NES용 C 컴파일러 NORCAL의 포크로 시작했습니다. NORCAL의 원저작자 Keith Holman의 저작권 고지를 유지하고 있습니다.

NORCAL이라는 이름은 북부 캘리포니아(Northern California)에서 따왔습니다. 지역의 이름을 사용하는 이 발상에서 영감을 받아, 제작자는 자신이 태어나 자란 기타큐슈시를 바탕으로 KITAQGB라는 이름을 지었습니다. KITAQ + GB는 일본 후쿠오카현 기타큐슈시의 애칭인 **北九(キタキュー, Kitakyū)**와 Game Boy를 합친 이름입니다. KITAQ는 일본어 ‘キタキュー’로 읽습니다. 영어 발음 안내는 **kee-tah-KYOO**, IPA는 **/ˌkiːtɑːˈkjuː/**입니다. 끝의 Q는 영어 알파벳 Q의 이름처럼 발음합니다. KITAQGB는 G와 B를 각각 읽어 **kee-tah-KYOO jee bee**라고 부릅니다.

KITAQGB라는 이름에는 두 가지 뜻이 있습니다. **Kernel-Informed Toolchain for AI-Quality Game Boy Development**는 대상 하드웨어를 이해하고, 사람이 직접 프로그래밍하는 과정과 생성형 AI를 활용하는 과정을 모두 지원하는 도구 모음을 지향한다는 뜻입니다.

또 다른 뜻은 **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**입니다. ‘아이들의 상상을 게임보이의 숲에서 진짜 모험으로 바꾸는 도구’라는 의미를 담았습니다. 작은 아이디어나 낙서, AI의 도움으로 만든 시제품을 실제로 즐길 수 있는 모험으로 바꾸는 도구를 만들고 싶다는 창작의 바람을 나타냅니다.

## 프로젝트 상태: 공개 미리 보기

KITAQGB와 KOKURA는 현재 공개 미리 보기 단계의 개발 도구입니다.

실험, 예제 프로젝트, AI를 활용한 게임 제작, 컴파일러 연구, 에뮬레이터 디버깅, 개발 절차 검증에 사용할 수 있습니다. 다만 개발이 계속 진행 중이므로 API, 명령줄 옵션, 출력 형식, 진단, 동작이 버전마다 달라질 수 있습니다.

미리 보기 빌드에는 버그, 미완성 기능, 호환되지 않는 변경이 있을 수 있습니다. 실제 배포에 쓰기 전에 생성 코드, 에뮬레이터 동작, 타이밍 진단, 보고서를 충분히 확인하세요.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB는 게임보이와 게임보이 컬러용 홈브루 소프트웨어를 만드는 오픈 소스 C 도구 모음입니다. 작은 C 프로그램을 `.gb` 또는 `.gbc` ROM으로 컴파일하고, 에뮬레이터에서 얻은 진단과 관찰 결과를 다음 수정에 반영하도록 설계했습니다. 이러한 과정은 AI를 활용한 개발에도 적합합니다.

KITAQGB는 Nintendo와 제휴 관계가 없으며, Nintendo의 보증·후원·승인을 받은 프로젝트가 아닙니다. Game Boy와 Game Boy Color는 Nintendo의 상표입니다.

## KITAQGB가 제공하는 기능

KITAQGB는 게임보이 계열 홈브루 개발을 위한 C 컴파일러와 지원 라이브러리로 구성됩니다. NORCAL을 바탕으로, 현대적인 도구와 AI를 활용한 게임 제작에 필요한 개발 절차를 확장했습니다.

주요 대상은 다음과 같습니다.

- C 소스를 게임보이 ROM으로 컴파일
- 게임보이·게임보이 컬러용 홈브루 제작
- AI도 처리하기 쉬운 진단과 재현 가능한 빌드 보고서
- LR35902 계열 대상의 ROM·헤더 생성과 저수준 코드 생성
- 컬러 팔레트, 타일, 스크롤, 카메라, 오디오, 통신, RPG·ADV·SLG 지원 라이브러리
- KOKURA CLI를 이용한 실행 테스트, 트레이스, 디버깅

KITAQGB에는 **상용 ROM, Nintendo BIOS, Nintendo SDK, 독점 소유권이 있는 게임 자료, Nintendo의 공식 개발 자료가 포함되지 않습니다**.

## 대상 플랫폼

게임보이 호환 소프트웨어, 게임보이 컬러 호환 소프트웨어, CGB 기능을 의도적으로 사용하는 게임보이 컬러 전용 소프트웨어의 홈브루 ROM을 생성합니다. 일반적인 출력 확장자는 다음과 같습니다.

```text
*.gb
*.gbc
```

생성한 ROM은 에뮬레이터에서 테스트하고, 가능하다면 실물 기기나 적절한 플래시 카트리지에서도 확인하세요. 타이밍, 인터럽트, VRAM/OAM 접근, 오디오, 통신 케이블, 뱅크 전환은 하드웨어의 세부 동작에 영향을 받습니다.

## 저장소 구성

컴파일러 소스는 이름이 같은 하위 폴더 `kitaqgb/`에 있습니다. 배포용 Release 실행 파일과 런타임 설정은 최상위 폴더에 두고, 라이브러리와 예제는 별도 폴더로 나눕니다.

```text
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

제공된 컴파일러를 실행하려면 Windows와 .NET Framework 4.8이 필요합니다. 실행 파일, 설정, 라이브러리, 라이선스 고지를 함께 받을 수 있도록 저장소 ZIP을 내려받으세요. 직접 빌드하려면 .NET Framework 4.8 Developer Pack과 Visual Studio Build Tools도 필요합니다. 저장소 최상위 폴더에서 실행하세요.

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Release 빌드는 실행 파일과 설정을 최상위 폴더로 복사합니다. Debug 빌드는 `kitaqgb/bin/Debug`에 남으며, 배포용 Release 컴파일러를 덮어쓰지 않습니다. 빌드 캐시와 PDB 파일은 배포하지 않습니다. 입력 자료와 SHA-256은 [바이너리 빌드 기록](BINARY_BUILD.json)에 있습니다.

## 필요한 개발 환경

주 개발 환경은 Windows, .NET Framework 4.8 대상 참조 파일, MSBuild가 포함된 Visual Studio 또는 Visual Studio Build Tools입니다. 프로젝트 파일은 `.NET Framework v4.8`을 대상으로 하는 MSBuild 형식의 C# 프로젝트입니다.

다른 운영체제에서도 설치된 참조 어셈블리에 따라 Mono/MSBuild로 빌드할 수 있지만, 주로 지원하는 환경은 Windows와 MSBuild입니다.

## KITAQGB 빌드

저장소 최상위 폴더에서 다음 명령을 실행하세요.

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

빌드가 성공하면 실행 파일을 최상위 폴더로 복사합니다.

```text
kitaqgb.exe
```

명령줄 도움말을 확인할 수 있습니다.

```powershell
.\kitaqgb.exe --help
```

## 빠르게 시작하기

작은 템플릿 프로젝트를 만듭니다.

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

컴파일합니다.

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

배포용 프로필로 빌드하려면 다음과 같이 실행합니다.

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

생성한 ROM을 게임보이·게임보이 컬러 에뮬레이터에서 실행하세요. 진단과 관찰 기능을 함께 사용하려면 KOKURA CLI를 사용할 수 있습니다.

## 동봉된 라이브러리 사용

`lib/`에는 재사용 가능한 C 코드가 있습니다. 필요한 라이브러리 소스를 게임 소스와 함께 컴파일하고, 헤더를 찾을 수 있도록 `-I lib`를 지정하세요.

오디오, 팔레트, 스크롤, 카메라, 물리를 함께 쓰는 예입니다. 아래 여러 줄 명령은 **cmd.exe**의 줄 연결 문자를 사용합니다.

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

시리얼 통신을 쓰는 예입니다.

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

RPG·ADV·SLG 기능을 쓰는 예입니다.

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

라이브러리 분류와 주의 사항은 lib/README.ko.md를 참고하세요.

## 자주 쓰는 명령줄 옵션

```text
-o <file>                  출력 ROM 경로
-I <dir>                   헤더 검색 폴더
--profile=dev              개발용 프로필
--profile=release          배포용 프로필
--fast-build / --fast      개발 중 빠른 빌드 사용
--cache                    빌드 캐시 사용
--no-cache                 빌드 캐시 사용 안 함
--disasm                   역어셈블리 출력 생성
--no-disasm                역어셈블리 출력 생략
--diag-json <file>         진단을 JSON으로 저장
--machine-readable         도구가 읽기 쉬운 출력 우선
--deps-out <file>          의존성 정보 출력
--debug-output <dir>       디버그·보조 출력 폴더
--strict                   일부 경고를 오류로 처리
--permissive               일부 진단 조건 완화
--stack-bank=fixed|wramx1  스택 뱅크 모델 선택
--stack-top=<addr>         스택 최상단 주소 선택
--stack-reserve=<bytes>    스택 영역 예약
```

현재 빌드가 지원하는 정확한 옵션은 도움말에서 확인하세요.

```powershell
.\kitaqgb.exe --help
```

## KOKURA CLI와 함께 사용하기

KOKURA CLI는 KITAQGB와 함께 쓰는 에뮬레이터·디버거입니다. 일반적인 흐름은 다음과 같습니다.

1. C 게임 코드를 직접 작성하거나 생성합니다.
2. KITAQGB로 컴파일합니다.
3. 생성한 ROM을 KOKURA CLI에서 실행합니다.
4. 진단, 트레이스, 심볼, 타이밍 관찰, 에뮬레이터 보고서를 기록합니다.
5. 결과를 바탕으로 코드를 수정하거나 다음 디버깅을 진행합니다.

AI를 활용하는 경우에도 컴파일 오류, 에뮬레이터 보고서, 실행 트레이스를 구체적인 수정 작업으로 연결할 수 있습니다.

## 개발 방향

KITAQGB는 범용 현대 C 컴파일러를 목표로 하지 않습니다. 적은 메모리, 뱅크 전환, 엄격한 타이밍을 가진 8비트 게임 플랫폼에 맞춘 홈브루 도구입니다.

예측 가능한 생성 코드, 명확한 진단, 작고 재현 가능한 예제, 사람과 AI가 읽기 쉬운 빌드·디버그 보고서를 중요하게 여깁니다. 필요한 곳에서는 저수준 제어를 제공하고, 가능한 곳에서는 사용하기 쉬운 지원 라이브러리를 제공합니다. 하드웨어를 완전히 감추지 않으면서 게임보이 개발을 더 쉽게 만드는 것이 목표입니다.

<!-- development-prompt:ko:start -->
## 게임 개발 프롬프트

요구 사항을 작성한 뒤 프롬프트 전체를 AI에 전달하세요. 구현, 에뮬레이터 테스트, SARAKURA 분석, 수정 후 재검증까지 다룹니다.

HTML 설명서에서 활용 예 읽기

<details>
<summary>프롬프트 전체 보기</summary>

### KITAQGB·KOKURA·SARAKURA를 활용한 게임 개발

요구 사항을 작성한 뒤 이 문서 전체를 AI에 전달하세요. 명령은 `kitaqgb`, `kitaqfc`, `kokura`, `kurosaki`, `sarakura`, `kitaq-docs` 저장소와 `game-gb` 또는 `game-fc` 프로젝트가 같은 상위 폴더에 있는 구성을 가정합니다. 해당 상위 폴더에서 실행하고, 실제 환경에 맞게 경로를 조정하세요.

#### 요구 사항

- 게임 이름: &lt;작성&gt;
- 장르와 핵심 플레이 방식: &lt;작성&gt;
- 조작 방법과 성공·실패 조건: &lt;작성&gt;
- 필수 화면·스테이지·적·아이템: &lt;작성&gt;
- 그래픽 스타일·배경 음악·효과음: &lt;작성, 제공 자료의 경로도 명시&gt;
- 저장·통신·주변기기 등 추가 요구 사항: &lt;작성 또는 없음&gt;
- 프로젝트 폴더: &lt;작성&gt;
- 재배포 조건: &lt;예: 자체 제작 코드와 소재를 MIT로 공개할 수 있는 상태&gt;

- 대상 기종: &lt;초대 Game Boy / GB·CGB 양쪽 지원 / CGB 전용&gt;
- 성능 목표: &lt;예: 일반 플레이에서 초당 60회 게임 갱신. 부하가 큰 장면에서 허용할 동작도 명시&gt;

#### 수행할 작업

KITAQGB와 해당 라이브러리로 게임을 구현해 주세요. 실행과 디버깅에는 KOKURA를, 진단 정리와 수정 전후 비교에는 SARAKURA를 사용하세요.

합격 기준을 충족할 때까지 명세 구체화 → 작은 단위 구현 → 빌드 → 입력과 관찰 → 원인 조사 → 수정 → 동일 조건 재검증을 반복하세요. 계획 작성, 코드 제시 또는 컴파일 성공만으로 완료하지 마세요.

##### 환경과 합격 기준 확인

1. 작업 폴더의 지침, 도구별 README, HTML 설명서, 사용할 라이브러리의 헤더와 구현을 읽으세요. 실행 파일 경로와 버전 또는 SHA-256을 기록하고, 명령은 실제 `--help` 출력으로, API는 소스로 확인하세요.
2. 입력·화면·소리·진행·갱신 빈도를 판정할 수 있는 합격 기준을 정하세요. 예를 들어 START를 눌렀다 놓으면 시작하고, 충돌하면 잔기가 하나 줄며, 일시 정지 시 지정한 소리가 멈추고 해제 후 다시 재생되는지 확인합니다.
3. 중요한 모호함만 질문하고, 일반적인 되돌릴 수 있는 구현 판단은 자율적으로 진행하세요. 요구 사항이나 합격 기준을 임의로 완화하지 마세요.
4. 먼저 작은 제공 예제를 컴파일러·에뮬레이터·SARAKURA로 실행해 도구 간 연결을 확인하세요. 이것을 요청받은 게임의 완성으로 간주하지 마세요.

##### 작게 시작해 플레이 가능한 형태로 구현

- KITAQGB의 C 문법과 `void main()`을 사용하세요. 데스크톱 C나 GBDK API를 그대로 쓸 수 있다고 가정하지 마세요. 선언뿐 아니라 필요한 `.c` 구현도 빌드에 포함하고, 초기화 순서·단위·부호·범위·버퍼 수명·ROM 뱅크를 확인하세요.
- VRAM/OAM 갱신, VBlank, 인터럽트, 스택, ROM/WRAM 뱅크와 타일·스프라이트 제한을 설계에 반영하세요. 전송 큐의 총용량·여유 용량은 물리 VRAM의 용량·여유 공간과 다릅니다.
- DMG 대상 게임에 CGB 전용 기능을 사용하지 마세요. 양쪽을 지원한다면 각 하드웨어 모드에서 따로 검증하세요.
- 영문자·숫자·기호에는 제공된 자체 제작 `ascii.c` 글꼴을 사용하고, 문자와 타일의 대응을 확인하세요.

- 먼저 부팅·타이틀·조작 가능한 플레이어·성공 또는 실패·재시작을 연결한 뒤 내용을 늘리세요.
- 그래픽·음악·효과음의 편집 가능한 원본과 생성 절차를 보관하고, 빌드가 실제로 내보낸 데이터를 읽는지 확인하세요.
- 소스 주석은 영어, 진행 보고는 한국어로 작성하세요. SARAKURA의 표준 보고서는 영어로 유지하세요.

##### 빌드와 실행 결과 연결

`out/iter-001`처럼 반복별 출력 폴더를 나누세요. 명령, 종료 코드, 소스·소재·도구·ROM·메타데이터의 해시를 기록하세요. 빌드 실패 후 남아 있는 이전 ROM을 실행하지 마세요. 맵·소스 맵·디버그 정보는 ROM과 동일한 빌드에서 나온 것을 사용하세요.

다음은 기본적인 DMG 확인 예입니다. `main.c`와 필요한 라이브러리 구현 파일을 준비하고 옵션과 입력 순서를 게임에 맞게 조정하세요.

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg`는 초대 GB용입니다. CGB 또는 양쪽 지원을 시험할 때는 ROM 헤더와 에뮬레이터 기종 설정을 맞추세요. 예제 입력은 버튼을 놓은 구간 사이에서 START를 한 번 누릅니다. 300프레임 실행이 게임 전체의 검증을 뜻하지는 않습니다.

##### 화면·소리·상태·성능 확인

- 누르기·유지·놓기를 구분한 입력 시나리오를 저장하세요. 부팅, 시작, 이동, 행동, 충돌, 스크롤, 스테이지 전환, 게임 오버, 재시작, 일시 정지와 필요한 저장·통신 등 명세의 모든 경로를 실행하세요.
- 필요한 프레임의 PNG, 입력 데이터, 실행 보고서, 진단 JSONL, WAV와 필요한 상태·메모리 관측을 보관하세요. 도달 프레임과 정지 이유를 확인하세요. 이미지를 실제로 열어 보고, 한 장의 스크린샷만으로 움직임이나 입력 반응을 검증했다고 하지 마세요. 카운터·좌표·상태 전환을 기대값과 비교하고 화면 끝·타일 및 속성 경계·스프라이트 밀집 장면도 확인하세요.
- 음악·효과음·동시 재생·끊김·일시 정지·재개를 확인하세요. WAV 생성만으로 올바른 소리를 증명할 수 없습니다. 들을 수 없는 환경에서는 실시한 파형·수치 검사와 아직 확인하지 못한 청감 품질을 구분하세요.
- 부하가 큰 장면의 대상 CPU 작업량·게임 갱신·전송량을 측정하고 FC에서는 NMI 작업도 포함하세요. 호스트에서의 에뮬레이터 처리 속도를 게임 갱신 빈도나 실기 속도와 동일시하지 마세요. `--allow-unimplemented`로 계속 실행된다고 해서 미구현 기능이 지원되는 것은 아닙니다.

##### 분석·수정·재검증

- 시험한 ROM의 빌드 메타데이터와 해당 실행의 진단 JSONL을 SARAKURA에 전달하세요. CPU 트레이스나 일반 실행 보고서로 대체하지 마세요. `--frames`는 분석 조건이며, SARAKURA는 ROM을 실행하거나 소스를 자동 수정하지 않습니다.
- `report.html`, `ai_diagnostics.json`, `repair_prompt.md`, `retest_plan.json`을 읽고 재현 절차·화면·소리·소스와 대조하세요. 추정한 소스 위치와 원인을 확인된 사실과 구분하고, 정상 대기 루프와 멈춤 버그를 구분하세요. 경고를 개별 판단하고 미지원 이벤트와 분석 한계를 기록하세요. 필터로 경고를 숨기거나 테스트를 줄여 합격시키지 마세요.
- 문제를 최소 재현 예제로 줄이고 원인을 수정한 뒤 다시 빌드하세요. 컴파일러나 에뮬레이터가 원인이면 게임 코드와 분리해 결함을 확인하고 도구 수정에 회귀 검증을 추가하세요.
- 입력·난수 시드·기종 및 영상 방식·매퍼·관측 프레임·진단 설정을 맞춰 재검증하세요. ROM마다 해당 메타데이터를 사용하고 코드나 RAM 배치가 바뀐 뒤 저장 상태를 무조건 재사용하지 마세요.

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


진단 차이는 조작·그래픽·소리의 합격 판정과 함께 사용하세요. 같은 실패가 반복되면 근거와 가설을 다시 검토하고 무작정 수정을 이어 가지 마세요.

##### 완료 조건과 결과물

납품할 소스와 설정으로 만든 최종 ROM에서 모든 필수 시나리오를 다시 실행하세요. 무적 상태·자동 시험 입력·다른 매퍼만으로 최종 빌드의 일반 플레이를 검증했다고 하지 마세요. 요구 사항과 시험의 대응표, 남은 경고의 이유, 미확인·미지원 항목을 명시하세요. 실기 시험을 하지 않았다면 ‘실기 미확인’으로 표시하세요.

소스, 도구·라이브러리 식별 정보, 편집 가능한 소재, 재현 가능한 빌드·검증 스크립트, ROM, 최종 검증 증거, 설치·조작·알려진 제한을 설명한 README를 제공하세요. 필요한 리플레이와 검증 하네스도 포함하세요. 공개·외부 전송은 명시적으로 허용된 범위에서만 수행하세요. 검증 후 불필요한 중간 빌드와 임시 트레이스는 지우되 소스·소재·최종 결과물·필요한 회귀 증거는 보관하세요.

환경이나 권한 때문에 필수 검사를 할 수 없다면 정확한 재현 절차와 필요한 조치를 보고하고, 완료로 처리하지 마세요.

</details>
<!-- development-prompt:ko:end -->

## 상표와 독립성

KITAQGB는 독립적인 오픈 소스 홈브루 프로젝트입니다. Nintendo와 제휴 관계가 없으며, Nintendo의 보증·후원·승인을 받지 않았습니다. Game Boy와 Game Boy Color는 Nintendo의 상표입니다.

필요한 권리가 없다면 Nintendo 로고, 공식 포장 그림, 공식 글꼴, BIOS, 상용 ROM 데이터, 독점 게임 자료를 이 저장소에 추가하지 마세요.

## 라이선스

KITAQGB는 MIT 라이선스로 배포합니다. 기반이 된 NORCAL의 원래 고지는 다음과 같습니다.

```text
Copyright 2019 Keith Holman
```

KITAQGB의 수정·추가 부분에는 다음 고지가 적용됩니다.

```text
Copyright (c) 2026 DAISUKE OBA
```

소프트웨어의 복사본이나 상당 부분을 배포할 때는 NORCAL의 원래 저작권 고지와 MIT 라이선스 고지를 유지해야 합니다. 자세한 내용은 [LICENSE](LICENSE)와 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)를 참고하세요.

## 기여하기

변경 사항을 제출하기 전에 다음을 확인하세요.

- 저작권이 있는 ROM, BIOS, 상용 게임에서 추출한 자료, 공식 SDK 자료를 추가하지 않습니다.
- 배포상 구체적인 이유가 없다면 `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll`, `*.pdb` 같은 빌드 생성물을 소스 커밋에 넣지 않습니다.
- 컴파일러와 코드 생성 오류에는 작고 재현 가능한 테스트 사례를 준비합니다.
- 라이브러리를 추가할 때는 빌드 명령과 필요한 하드웨어 레지스터 선언을 문서화합니다.
- 진단은 사람과 AI 코딩 도구가 다음 행동을 판단할 수 있도록 명확하게 작성합니다.

## 이번 공개본의 상태

이 저장소는 KITAQGB의 첫 공개판을 준비한 것입니다. 프로젝트가 발전하면서 인터페이스, 라이브러리, 진단, 다른 도구와의 연동이 바뀔 수 있습니다.

## 빌드 후 첫 실행

Windows에서 .NET Framework 4.8 Developer Pack과 Visual Studio Build Tools의 MSBuild를 사용합니다. Developer PowerShell에서 실행하세요.

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## 설명서와 라이선스

- 한국어 컴파일러 설명서 / 한국어 라이브러리 설명서
- [오프라인 열람용 설명서 소스](https://github.com/bartaro/kitaq-docs)
- [라이선스](LICENSE) / [일본어 참고 번역](LICENSE.ja)
