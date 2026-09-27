# ZX0 호환 리소스 압축

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | **한국어** | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

[API와 예제](https://bartaro.github.io/kitaq-docs/ko/gb-library.html#module-zx0)

PC 압축기와 GB 압축 해제기는 ZX0 v2 정방향 스트림을 지원하는 KITAQ의 독자 구현입니다. KITAQ 구현은 MIT 라이선스로 배포하며, 저작권은 Copyright (c) 2026 DAISUKE OBA입니다.

ZX0 형식과 원래 압축 알고리즘의 설계자는 [Einar Saukas](https://github.com/einar-saukas/ZX0)입니다. 형식에 대한 이 표기는 KITAQ 구현의 저작권·라이선스와 구분됩니다. [LICENSE](../../LICENSE)와 [LICENSE.ja](../../LICENSE.ja)도 참고하세요.

저장소 최상위 디렉터리에서 PC 도구를 빌드합니다.

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

독립 실행 도구는 .NET Framework 4.x를 사용합니다. 압축 입력은 1~65535바이트입니다. 탐색 횟수를 제한한 해시 체인을 사용하므로 최적의 압축 크기를 보장하지 않습니다. KITAQ 래퍼가 없는 일반 ZX0 v2 데이터를 출력합니다. 역방향 스트림, 접두 사전, ZX0 v1은 이 인터페이스의 지원 대상이 아닙니다. 원본 리소스의 권리는 각 제작자에게 있습니다.

인코딩된 출력은 대상 API의 65535바이트 크기 매개변수 범위에 들어가야 합니다. 자동 선택 컨테이너는 9바이트 헤더도 이 한도에 포함합니다. 큰 리소스는 나누고, 실제 하드웨어의 훨씬 작은 RAM 용량과 뱅크 창도 고려하세요. 빈 raw 데이터를 C 헤더로 출력하면 저장 공간용 바이트 하나를 두지만 논리적 `_SIZE`는 0입니다. 빈 순수 ZX0 스트림은 지원하지 않습니다.

비압축 데이터, 개수/값 RLE, ZX0를 비교해 가장 작은 페이로드를 선택하려면 다음과 같이 실행합니다.

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

자동 모드는 9바이트 KQA1 헤더를 추가하고 선택한 코덱을 보고합니다. 페이로드 크기를 비교하며 동률이면 raw, RLE, ZX0 순으로 우선합니다. 이는 순수 ZX0 스트림이 아닌 KITAQ 리소스 컨테이너입니다. 헤더는 `KQA1`, 코덱 1바이트(0 raw, 1 RLE, 2 ZX0), 리틀 엔디언 u16 원본 크기, 리틀 엔디언 u16 페이로드 크기 순입니다. 본문은 바로 뒤에 붙습니다. 이 컨테이너에는 `asset_decompress`를 사용하세요. `--format=raw`와 `--format=rle`는 선택한 페이로드만 출력하며, RLE는 개수 0으로 끝납니다.

대상 프로그램에서 `zx0.h`를 포함하고 `lib/zx0.c`를 함께 컴파일하세요. `zx0_decompress`에는 출력 위치, 출력 용량, 압축 입력 위치, 압축 크기를 전달합니다. 반환된 바이트 수와 `zx0_error`를 모두 확인하세요. 오류가 나면 출력이 일부만 기록될 수 있으므로 실패한 결과를 표시하거나 사용하지 마세요. 입력과 출력 버퍼는 겹치거나 현재 매핑된 CPU 뱅크 창의 경계를 넘어서는 안 됩니다. 공유 작업 영역을 사용하므로 인터럽트에서 재진입하면 안 됩니다.

버퍼가 CPU 주소 범위를 넘어 처음 주소로 되돌아가는 경우도 허용되지 않습니다.

`zx0_decompress_vram`은 LCD가 꺼져 있을 때만 선택한 GB VRAM 뱅크에 씁니다. 화면, 뱅크, 인터럽트 설정은 유지합니다. 장면 초기화 중에 업로드하고, 리소스 전체를 한 번의 VBlank 안에 처리할 수 있다고 가정하지 마세요.
