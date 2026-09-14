# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **한국어**

**[컴파일러 한국어 설명서](https://bartaro.github.io/kitaq-docs/ko/kitaqgb.html)** · **[라이브러리 한국어 설명서](https://bartaro.github.io/kitaq-docs/ko/gb-library.html)**

## 이름의 유래

KITAQGB는 Zachtronics와 관련된 NES용 C 컴파일러 NORCAL의 포크로 시작했습니다. NORCAL의 원저작자 Keith Holman의 저작권 고지를 유지하고 있습니다.

**NORCAL이라는 이름은 북부 캘리포니아(Northern California)에서 따왔습니다.** 지역의 이름을 사용하는 이 발상에서 영감을 받아, KITAQGB의 제작자 DAISUKE OBA는 자신이 태어나 자란 도시인 **기타큐슈**를 바탕으로 KITAQGB라는 이름을 지었습니다.

**KITAQGB**라는 이름에는 두 가지 뜻이 겹쳐 있습니다.

- **Kernel-Informed Toolchain for AI-Quality Game Boy Development**는 대상 하드웨어를 이해하고, 사람이 직접 개발하는 과정과 생성형 AI를 활용하는 과정을 모두 지원하는 도구 모음을 지향한다는 뜻입니다.
- **KITAQ + GB**는 **일본 후쿠오카현 기타큐슈시**의 애칭과 **Game Boy**를 합친 이름입니다. **KITAQ**는 기타큐슈의 애칭인 **北九(キタキュー, Kitakyū)**를 나타냅니다.

영어 발음 안내로는 **KITAQ**를 **“kee-tah-KYOO”**, IPA **/ˌkiːtɑːˈkjuː/**로 읽으면 일본어 **キタキュー**에 가깝습니다. 끝의 **Q**는 영어 알파벳 **Q**의 이름처럼 발음합니다. **KITAQGB**는 **“kee-tah-KYOO jee bee”**로, **G**와 **B**를 각각 영어 글자 이름으로 읽습니다.

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

라이브러리 분류와 주의 사항은 [lib/README.ko.md](lib/README.ko.md)를 참고하세요.

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

- [한국어 컴파일러 설명서](https://bartaro.github.io/kitaq-docs/ko/kitaqgb.html) / [한국어 라이브러리 설명서](https://bartaro.github.io/kitaq-docs/ko/gb-library.html)
- [오프라인 열람용 설명서 소스](https://github.com/bartaro/kitaq-docs)
- [라이선스](LICENSE) / [일본어 참고 번역](LICENSE.ja)
