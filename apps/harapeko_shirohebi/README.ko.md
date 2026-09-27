# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | **한국어** | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

**DAISUKE OBA**가 제작한 Game Boy / Game Boy Color용 점수 경쟁 게임입니다. 흰 뱀을 조종해 석류를 먹고 몸을 늘리면서 지뢰와 자신의 몸을 피하세요.

- **게임 소개 및 공식 ROM 다운로드:** <https://bartaro.itch.io/harapeko-shirohebi>
- **프로그램 해설:** [한국어 HTML](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-ko.html)
- **라이선스:** [MIT](LICENSE), 저작권 © 2026 DAISUKE OBA. 이 디렉터리의 게임 소스, 직접 제작하여 제공하는 그래픽·글꼴 데이터·음악·효과음, 문서에 적용됩니다. 재배포할 때 라이선스 고지를 유지하세요. KITAQGB와 의존 구성 요소에는 [저장소 라이선스](../../LICENSE)의 고지가 적용되며, [원본 ASCII 글꼴 고지](../../licenses/fonts/ASCII-font-MIT.txt)도 포함됩니다.

HTML 해설에는 프로그램 흐름도, 몸통 추종 알고리즘, 라이브러리 사용 예제, 소스 파일 안내가 있습니다. 위 링크에서 KITAQ Docs 페이지를 브라우저로 바로 읽을 수 있습니다. HTML과 스타일시트는 [kitaq-docs 저장소](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi)에서 관리합니다.

## Windows에서 빌드하기

Windows, .NET Framework 4.8, PowerShell, 이 저장소 전체의 체크아웃 또는 ZIP이 필요합니다. 최상위 컴파일러 파일과 `lib/`를 함께 두세요. 그래픽과 오디오는 C 배열로 제공하므로 리소스 편집기는 필요하지 않습니다.

저장소 최상위 디렉터리에서 실행합니다.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

게임 디렉터리에서 실행할 수도 있습니다.

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

기본 출력은 `out/shirohebi.gb`이며, `out/shirohebi.map`과 `out/build_manifest.json`도 생성합니다. `-OutputDirectory C:\build\shirohebi`로 출력 위치를 바꿀 수 있습니다. 빌드 성공 시에는 `-KeepBuildFiles`를 지정하지 않은 한 임시 컴파일 디렉터리를 지웁니다. 실패하면 진단을 위해 보관합니다. 출력 파일은 Git 추적 대상에서 제외됩니다.

스크립트는 분할된 소스를 합치고 저장소의 라이브러리와 컴파일한 뒤, 인터럽트 루틴이 고정 ROM에 있는지 검사합니다. 이어서 VBlank 벡터를 설치하고 카트리지 체크섬을 갱신합니다. 결과는 DMG와 CGB에서 실행할 수 있는 **64 KiB MBC5 ROM, 8 KiB 배터리 백업 RAM** 구성입니다. 분할 파일을 따로 컴파일하거나 벡터·체크섬 단계를 생략하지 마세요.

이 게임 디렉터리에는 **미리 빌드한 실행 파일이나 ROM이 없습니다**. 컴파일러는 저장소 최상위에 있으며, 배포된 게임 ROM은 itch.io에서 받으세요.

## 조작

| 화면 | 조작 방법 |
|---|---|
| 타이틀 | START: 시작. UP/DOWN/SELECT: MUSIC 또는 SOUND 선택. LEFT/RIGHT/A: 선택한 옵션 켜기/끄기. |
| 플레이 | LEFT/RIGHT: 뱀이 바라보는 방향을 기준으로 회전. UP 길게 누르기: 가속. START: 일시정지. |
| 일시정지 | START: 재개. SELECT: 타이틀 복귀 확인창 열기. |
| 확인창 | LEFT/RIGHT/SELECT: 선택. A: 확인. B/START: 취소. |
| 이름 입력 | UP/DOWN: A–Z 또는 마침표 선택. LEFT/RIGHT/SELECT: 커서 이동. A: 다음 글자로 이동하거나 세 번째 글자에서 완료. START: 완료. |
| 재시도 | LEFT/RIGHT/SELECT: YES/NO 선택. A/START: 확인. |

타이틀에서 SELECT+START를 누르면 점수 삭제 확인창을 엽니다. 화면 전환 후에는 모든 버튼을 놓아야 새 입력을 받습니다. 상태 전환과 구현 세부 사항은 HTML 해설을 참고하세요.
