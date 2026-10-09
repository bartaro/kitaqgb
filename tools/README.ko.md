# 네이티브 보조 도구

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

모든 보조 실행 파일은 Rust로 구현되었습니다. 실행 시 .NET, Python, Pillow가 필요하지 않습니다. `cargo build --locked --release`로 모든 실행 파일을 빌드합니다. Windows에서는 명령 이름에 `.exe`를 붙입니다.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0는 1~65535바이트를 받아 C# 인코더의 출력을 유지합니다. `raw`, 개수/값 `rle`, 9바이트 `KQA1` 자동 컨테이너를 지원합니다. 크기가 같으면 raw, RLE, ZX0 순서로 선택합니다. `--decompress`는 출력 크기를 제한한 순방향 ZX0 v2 스트림을 받습니다. 역방향 스트림과 v1은 지원하지 않습니다. 형식 설계자는 Einar Saukas이며 KITAQ 구현의 라이선스는 MIT입니다.

VBlank 도구는 고정 뱅크 심볼과 PUSH 패턴을 검사한 뒤 지정 ROM을 직접 수정하고 체크섬을 갱신합니다. `--no-header-fix`는 체크섬을 유지합니다. 패턴 검색은 인터럽트 루틴 전체를 검증하지 않습니다. 수정 전에 백업하세요.

<!-- wire3d-feedback:start -->

Wire3D 시간 측정, 88행 프로파일과 독립 시계

DMG의 WIRE3D_DMG_HEIGHT는 88·96·120, CGB의 WIRE3DCGB_HEIGHT는 88·96을 지원합니다. 기본값은 DMG 120, CGB 96입니다. 88행 화면은 128×88이며 중심 Y=44입니다. 라이브러리와 호출 코드의 설정을 맞추고 일반 진입점 대신 wire3d_dmg_88.c / wire3d_cgb_88.c를 컴파일합니다. DMG 88은 96행의 모델 구조와 16개 변 제한을 사용합니다. CGB 160×144 모드는 그대로입니다.

[검증 결과와 예제](https://bartaro.github.io/kitaq-docs/ko/gb-library.html#wire3d-feedback-20261009)

Wire3D 측정 및 회귀 검증 스크립트는 Python 3을 사용합니다. 에뮬레이터 검증에는 KOKURA Python 브리지와 C API DLL이 필요합니다. PNG 출력용 Pillow는 선택 사항입니다.

<!-- wire3d-feedback:end -->
