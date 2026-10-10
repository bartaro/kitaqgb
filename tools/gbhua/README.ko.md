# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA는 PNG를 Game Boy 타일, 팔레트, 맵으로 변환합니다. 제작자: DAISUKE OBA. 자체 코드는 MIT 라이선스입니다.

## 빌드와 실행

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## macOS 실행 파일

Apple Silicon 및 Intel Mac용 CLI: [다운로드 및 실행 안내](MACOS.md).
해당 압축 파일을 풀고 터미널에서 그 폴더로 이동하여 `./gbhua --help`를 실행합니다.
아래 예제의 `./target/release/gbhua` 대신 `./gbhua`를 사용합니다. Rust와 Python 설치는 필요 없습니다.
최소 OS 빌드 설정은 macOS 11.0이며 macOS 15에서 테스트합니다. Apple Developer ID 서명 및 공증은 없습니다.

## 작업 순서

PNG 가져오기, JSON 보고서 확인, 검증, 미리 보기, 내보내기 순서로 사용합니다. 성공 시 JSON 객체 하나를 출력합니다. AI에는 프로젝트 전체 대신 파일 경로와 짧은 보고서를 전달하십시오.

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

이미지 크기는 8의 배수(8..2040)여야 합니다. --size는 최근접 보간으로 크기를 바꿉니다. DMG는 4단계 명암, CGB는 RGB555 근사와 타일당 4색, 최대 8팔레트를 사용합니다. 투명 영역은 흰색과 합성됩니다. 동일 타일은 공유하며 고유 타일이 256개를 넘으면 중단합니다. 경고와 미리 보기를 확인하십시오.

전체 프로젝트는 .gbh로 저장합니다. 기존 JSON 프로젝트는 확장자만 .gbh로 변경하면 됩니다. 이전 확장자 로더는 없습니다. GBTD/GBR/GBTB 및 GBMB/GBM은 교환 형식으로 모든 정보를 보존하지 않습니다. GBMB 출력은 같은 이름의 .gbr도 생성하므로 함께 보관하십시오.

기존 파일 덮어쓰기에는 --force가 필요합니다. 입력과 출력 경로는 달라야 합니다. 종료 코드: 0 성공, 1 검증 실패, 2 인수/I/O/변환 오류. 이미지 생성 API나 네트워크 업로드는 없습니다. 의존성의 원래 라이선스는 LICENSE, THIRD_PARTY_NOTICES.md, DEPENDENCIES.json을 참조하십시오.

## GUI / Python

로컬 GUI와 Python 바인딩은 같은 변환 코어를 사용합니다. 독립 CLI 패키지에는 GUI가 없습니다. 전체 로컬 작업 공간에서는 gbhua_gui를 실행하고 Python에서는 import gbhua를 사용합니다. GUI에는 그룹 메뉴, 저장 대화상자, DMG/CGB 비교, 7가지 크기, 16바이트 입력, 실행 취소/다시 실행, 맵 확대와 스크롤이 있습니다.

