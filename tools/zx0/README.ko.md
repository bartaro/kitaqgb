# 네이티브 보조 도구

[en](../README.en.md) · [ja](../README.ja.md) · [ko](../README.ko.md) · [zh-CN](../README.zh-CN.md) · [zh-TW](../README.zh-TW.md) · [fr](../README.fr.md) · [es](../README.es.md) · [de](../README.de.md)

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

```sh
sh tools/zx0/build.sh
```
