# GBSEQ — manual do usuário

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ, de DAISUKE OBA, oferece GUI com plita e CLI independente, compartilhando modelo, validação, áudio KOKURA e exportadores. LOCAL 4CH, CABLE 8CH e DMG07 16CH representam 1, 2 e 4 Game Boy. Cada jogador tem CH1/CH2 de pulso, CH3 de onda e CH4 de ruído. São 1–256 compassos de 16 passos (4096 por canal), BPM 30–300 e opção de repetição. A interface usa inglês; os manuais estão em nove idiomas.

## Instalação

Em [bin/](bin/), escolha Windows x86_64, Linux x86_64 ou macOS Apple Silicon/Intel. Extraia o pacote inteiro, mantendo assets e licenças junto aos programas. A GUI é gbseq.exe no Windows ou ./gbseq no macOS/Linux. Nos exemplos, gbseq-cli deve ser ./gbseq-cli.exe no PowerShell ou ./gbseq-cli no Unix, salvo se estiver no PATH. O uso normal não exige compilador, outra instalação de plita nem Python.

A GUI precisa de desktop, driver gráfico compatível e dispositivo de áudio para ouvir. Linux destina-se ao Ubuntu 24.04 ou sistemas posteriores compatíveis, com ALSA, X11/Wayland, libxkbcommon e bibliotecas Vulkan/OpenGL. Os binários macOS não são assinados nem notarizados: confira origem e hash e, se necessário, autorize apenas este aplicativo em Abrir/Privacidade e Segurança, sem desativar a proteção global. A CLI gera WAV sem tela nem dispositivo de som. Se necessário no Unix: chmod +x gbseq gbseq-cli.

## Edição gráfica

Escolha modo e P1–P4. ALL16 no DMG07 exibe e toca os dezesseis canais. Clique duas vezes numa faixa vazia para inserir nota; numa nota para abrir o inspetor. Arraste o corpo para mudar tempo/altura ou a borda direita para duração. Cada faixa é monofônica e rejeita sobreposições. Ajuste 1–256 compassos. Navegue com roda/barra inferior, F para enquadrar a música, Home para início, +/- para zoom. A tela acompanha a reprodução. Space inicia/para; LOOP ON/OFF controla repetição. A primeira reprodução renderiza a música inteira e pode demorar.

Ctrl/Shift-clique alterna seleção; Ctrl+A seleciona a faixa atual. Esquerda/direita move pela grade; cima/baixo muda semitons, Shift+cima/baixo oitavas. Ctrl+D duplica, Delete exclui, G troca grade, Q alinha inícios. Ctrl+Z/Y desfaz/refaz, Ctrl+C/V copia/cola. Ctrl+O abre, Ctrl+S salva, Ctrl+Shift+S salva como. A troca de alterações não salvas pede confirmação; guarde uma cópia antes de sobrescrever.

## Timbres e ondas

Por nota, ajuste nível, panorama esquerda/direita/centro, ciclo de trabalho e envelope de pulso, sweep CH1, nível/preset CH3 e ruído CH4. AUDITION pré-escuta a nota. CH3 não possui envelope de hardware. NEW USER WAVE desenha 32 amostras de 4 bits (0–15). São 64 ondas de usuário e sete internas, atribuídas a 16 presets ativos com nível 100/50/25%. WAVE PRESET automatiza o timbre por nota. A troca de onda desliga brevemente CH3; não é crossfade contínuo.

## CLI

track/player começam em 1; start em 0. length 1 é semicolcheia, 16 um compasso 4/4. pitch aceita 0–47, velocity 0–15. players aceita 1, 2 ou 4. Edições exigem --out e podem sobrescrever esse destino explícito. Uma edição inválida termina com código não zero e não é salva. transform também aceita --shift, --quantize e --velocity. --all-players mistura todos; o padrão é P1. Exportações podem substituir arquivos existentes.

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## Exportação

LINK16 gera imagens de bateria MBC5 V4 de 128 KiB: link16_local.sav; link16_cable.p1.sav/.p2.sav; ou link16_dmg07.p1.sav–.p4.sav. Use ROM/versão LINK16 compatível e a regra de nomes do emulador. Faça backup dos SAV existentes. P1 também inclui espelhos das partes remotas. Nenhuma ROM LINK16 acompanha este pacote; SAV não inicia sozinho. Há quatro presets de envelope compartilhados por jogador; valores extras podem ser aproximados. Confira ouvindo após importar. V4 não guarda sweep CH1. frames_per_step_override permite efeitos por quadro em prévia/WAV/KITAQGB, mas é rejeitado pelo LINK16 baseado em BPM.

KITAQGB exporta C/fluxo de um jogador; repita para cada um. Use audio.h/audio.c compatíveis com CH1=0, CH2=1, CH3=2 e CH4=3. Não junta dezesseis canais numa Game Boy. A prévia quantiza passos a quadros de vídeo; alguns BPM diferem de ROM com relógio fracionário. Emulação não equivale a teste em hardware físico.

## Dados e Python

.gbseq é JSON, não MIDI; mudar a extensão não converte. Valide dados gerados por IA e verifique duração, canais ativos, IDs únicos, alturas e sobreposições. As [especificações detalhadas](docs/README.md) estão em japonês. Para Python 3.10+ opcional, adicione python/ ao PYTHONPATH e defina GBSEQ_NATIVE_LIBRARY com o caminho absoluto de gbseq.dll/libgbseq.dylib/libgbseq.so. Os índices Python começam em zero; chame validate após editar data diretamente. player=None mistura todos.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## Compilação, problemas e direitos

Use Rust 1.93.1 e ferramentas C/C++ do sistema. Linux requer libasound2-dev, libudev-dev, libxkbcommon-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev e bibliotecas de desenvolvimento Mesa. CI verifica modelo, edição, áudio/SAV e início da GUI por OS; verification registra o commit. Não testa saída física de áudio, entrada física nem hardware Game Boy.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


Se não iniciar, confira driver GPU e erro no terminal. Texto ausente: restaure assets/fonts; silêncio: dispositivo e mute/solo; exportação: validate e nota indicada. GBSEQ usa MIT, Copyright (c) 2026 DAISUKE OBA. Preserve [LICENSE](LICENSE), [avisos de terceiros](THIRD_PARTY_NOTICES.md), [auditoria](LICENSE_AUDIT.json) e licenças das fontes ao redistribuir. Materiais externos e músicas importadas mantêm os próprios direitos.
