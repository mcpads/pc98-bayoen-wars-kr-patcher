# 바요엔워즈 대마도전략물어 (PC-98) 한글 패처

`Disc Station Vol. 05` Disk 1에 수록된 PC-98용 《바요엔워즈 대마도전략물어》를 독립 실행 HDM으로 꺼내고 한글 패치를 적용하는 Rust 코드입니다. 원본 판별, MZ+LHa 설치기 해제, PC-98 FAT12 이미지 생성과 readback, GAIJI 한글 글리프 생성, 대사·인터페이스·전투 호출문 재배치, 오프닝·엔딩·타이틀 그래픽 교체, V30 코드 패치와 RetroGame Patcher용 패치 계획 작성을 제공합니다.

배포용 패치와 적용 방법은 [마도물어 시리즈 한글 번역 프로젝트](https://github.com/mcpads/madou-monogatari-kr-patch/tree/main/pc98-bayoen-wars)에서 제공합니다.

## 제공하지 않는 것

이 저장소에는 원본 디스크, 패치를 적용한 디스크, 번역 JSON, 폰트 파일, 한글 타이틀 원화와 아르르 교체 스프라이트가 없습니다. 따라서 이 저장소만으로는 배포 패치를 다시 만들 수 없습니다. 아래 입력을 직접 갖춘 경우에만 한글화 빌드 명령이 디스크를 생성합니다.

이 커밋의 코드는 배포 저장소의 1.0.0 패키지(`Bayoen Wars (PC-98) KR v1.0.0.zip`, SHA-256 `a9347cf631001bf0ce46bc947785c42b9d5ea47ff4c99be04e56a3a1fe4e1171`)를 만든 코드입니다. 아래 입력을 갖추고 RetroGame Patcher 커밋 `31d2a8f`로 패키지 스크립트를 실행하면 같은 바이트의 ZIP이 나옵니다.

## 빌드와 테스트

```bash
cargo build --release
cargo test
```

기본 테스트는 합성 입력만 사용합니다. 원본 디스크, 폰트, 번역, 타이틀 원화나 아르르 스프라이트가 필요한 테스트는 `#[ignore = "requires ..."]`로 필요한 입력을 밝혀 두었습니다. 입력을 갖춘 뒤 `cargo test -- --ignored`로 실행하며, 입력이 없으면 성공으로 넘어가지 않고 실패합니다. 원본 디스크 경로는 `DS5_DISK1`, 타이틀 원본 프레임 경로는 `BAYOEN_TITLE_SOURCE_PREVIEW` 환경 변수로 지정합니다.

## 지원 원본

`Disc Station Vol. 05` Disk 1의 헤더 없는 HDM을 지원합니다. 빌더는 디스크 전체의 크기와 SHA-256을 확인하고, 다르면 진행하지 않습니다.

| 원본 | 크기 | SHA-256 |
| --- | --- | --- |
| Disc Station Vol. 05 Disk 1 | 1,261,568바이트 | `94f73ae53493719d983dadae3adbade79ee8174789de6b817edf09670b94b558` |

```bash
cargo run --release -- verify-source --source "Disc Station Vol. 05 (Disk 1).hdm"
```

한글화 없이 게임만 담은 독립 실행 HDM은 원본만으로 만들 수 있습니다.

```bash
cargo run --release -- build --source "Disc Station Vol. 05 (Disk 1).hdm"
```

## 빌드 입력

| 입력 | 기본 경로 | 비고 |
| --- | --- | --- |
| 번역 | `assets/translations/` | `index.json`과 `segments/*.json` (`--translations`로 변경) |
| 타이틀 원화 manifest | `assets/title_art/development-title-art.json` | SHA-256 `00dbb5f50e4a4dfc87a5150772bccbf3a65bc1469915bd355cc3bf317cb46da9` |
| 타이틀 원화 | `assets/title_art/title-authored-frame.png` | 1280×800 RGB, SHA-256 `232b67df24e2f4ed358dbb7dd78deb3038afcfb4789f2dba3b80d683a632ede7` |
| 원본 타이틀 프레임 | 직접 생성 | 아래 `graphics-audit`가 만드는 `title-source-frame-0c00.png` |
| 본문·오프닝·엔딩 폰트 | `assets/fonts/Galmuri14.ttf` | [Galmuri](https://github.com/quiple/galmuri) 2.404 |
| 난이도 그래픽, 타이틀 로고 폰트 | `assets/fonts/Galmuri11-Bold.ttf` | [Galmuri](https://github.com/quiple/galmuri) 2.403 |
| 선택·난이도 그래픽, 타이틀 부제 폰트 | `assets/fonts/MulmaruMono.ttf` | [물마루](https://github.com/mushsooni/mulmaru) 1.0 |
| 폰트 라이선스 | `assets/fonts/Galmuri-OFL.txt`, `assets/fonts/Mulmaru-OFL.txt` | 각 폰트 배포처의 OFL 전문 |

폰트 디렉터리는 `BAYOEN_WARS_FONT_DIR` 환경 변수로 바꿀 수 있습니다. `assets/fonts/*.json`의 폰트 프로필이 크기·기준선과 폰트 SHA-256을 고정하며, 폰트 파일의 SHA-256이 다르면 빌드가 진행하지 않습니다.

```text
6fe6c3fe4369e3837ac348431e8670733d67aa4bd550982baa72cc93c81a1c68  Galmuri14.ttf
5265b2f437fe81f0c8095b44c0173dd9a276b58a42552bf983f21c0e69e6e8af  Galmuri11-Bold.ttf
34a1641eb4e94449b26192321e8e0c2bd4f07ef3674fac8abed33d8953a7f70d  MulmaruMono.ttf
```

원본 타이틀 프레임은 원본 디스크에서 다음처럼 만듭니다. 타이틀 원화 manifest는 이 PNG의 SHA-256 `f931aaae708a5bdf0581bfe9f6acd31bea085e203af46ddc6852b7ac2efe9c15`를 요구합니다.

```bash
cargo run --release -- graphics-audit \
  --source "Disc Station Vol. 05 (Disk 1).hdm" \
  --output-directory work/graphics-audit
```

## 패치 생성

배포 패치는 [RetroGame Patcher](https://github.com/mcpads/retro-patcher)의 패키지 형식(`recipe.json`과 파일별 BPS를 담은 ZIP)입니다. 위 입력을 갖추고 RetroGame Patcher 저장소를 받은 뒤 다음 스크립트를 실행합니다.

```bash
scripts/build-in-game-development-patch-package.sh \
  "Disc Station Vol. 05 (Disk 1).hdm" \
  work/graphics-audit/title-source-frame-0c00.png \
  path/to/retro-patcher \
  out/bayoen-wars-ko.zip
```

스크립트는 게임 본편만 한글화한 content HDM을 만들고(`build-mad-scene-narrative-development`), 패치 계획을 쓰고(`write-in-game-retro-patcher-plan`), 패키지를 두 번 작성해 바이트가 같은지 확인합니다. 이어 원본에 패키지를 다시 적용해 73개 논리 파일을 content HDM과 대조합니다. 부팅 셸, 실행 전 메뉴와 외부 드라이버는 원본 그대로 둡니다.

## 그 밖의 명령

개별 한글화 단계만 넣은 개발 디스크(`build-*-development`), 구조 조사(`survey-source`), 번역 작업공간 추출(`extract-translation-workspace`), 그래픽·음성 진단(`graphics-audit`, `audio-audit`)과 번역 글리프·레이아웃 감사의 사용법은 `cargo run -- help <명령>`으로 확인할 수 있습니다.

## 라이선스

이 저장소의 소스 코드는 [MIT License](LICENSE)로 제공합니다.
