# open-scrantic

Uma reimplementação moderna, em Rust, do *Screen Antics: Johnny Castaway* (Sierra/Dynamix, 1993),
o protetor de tela de Windows 3.1. O binário novo contém **apenas a lógica**: toda a arte, scripts,
sons e tabelas do jogo são lidos dos **arquivos originais instalados**.

## Requisitos

- Rust 1.99.1 (fixado em `rust-toolchain.toml`)
- Os arquivos da instalação original, num diretório:

| Arquivo        | Tamanho     | Conteúdo                                                        |
|----------------|-------------|-----------------------------------------------------------------|
| `SCRANTIC.SCR` | 295 952     | executável Win16 original (sons + tabelas de lógica)            |
| `RESOURCE.MAP` | 1 461       | índice dos recursos                                             |
| `RESOURCE.001` | 1 175 645   | imagens (BMP/SCR), paleta, scripts de animação (TTM/ADS)        |

## Uso

```sh
cargo run --release -- /caminho/da/instalacao          # janela 640x480
cargo run --release -- /caminho/da/instalacao --scale 2
cargo run --release -- /caminho/da/instalacao --saver  # tela cheia, sai com qualquer entrada
```

Teclas (modo janela): `F`/`F11` alterna tela cheia, `Esc`/`Q` sai.

O progresso da história e as opções ficam em `SCRANTIC.INI` no diretório de configuração do
usuário (ex.: `~/Library/Application Support/open-scrantic/` no macOS), com as mesmas chaves do
original: `Background`, `Clouds`, `Waves`, `Sounds`, `NumDays`, `Introduction`,
`CurrentYear/Month/Day` e `StartTime`.

## Desenvolvimento

```sh
OPEN_SCRANTIC_DATA=/caminho/da/instalacao cargo test           # testes com os dados reais
OPEN_SCRANTIC_DEBUG=1 cargo run --release -- /caminho           # mostra o plano de cada sessão
cargo run --release --example snapshot -- /caminho /tmp/out 2000 100 1000 42   # render headless em PNG
```

Arquitetura:

- `src/assets/` — leitura dos arquivos originais: arquivo de recursos Dynamix (RLE/LZW, BMP, SCR,
  PAL, TTM, ADS) e o executável NE (sons `WAVESFXn` e tabelas do segmento de dados).
- `src/engine/` — o motor: VM de TTM (`ttm.rs`), VM de ADS e escalonador de cenas (`ads.rs`),
  ilha e animações ambientes (`island.rs`), caminhada (`walk.rs`), planejamento da história
  (`mod.rs`), gerador aleatório original (`rng.rs`), primitivas gráficas (`gfx.rs`), INI (`config.rs`).
- `src/main.rs` — janela (winit + softbuffer), relógios e entrada; `src/audio.rs` — som (rodio).

Os detalhes do que foi extraído por engenharia reversa estão em [`docs/ENGINE.md`](docs/ENGINE.md).
