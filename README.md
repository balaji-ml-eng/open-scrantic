# open-scrantic

A modern Rust reimplementation of *Screen Antics: Johnny Castaway* (Sierra/Dynamix, 1993), the
Windows 3.1 screensaver. The new binary contains **only the logic**: all art, scripts, sounds and
game tables are read from the **original installed files**.

## Requirements

- Rust 1.99.1 (pinned in `rust-toolchain.toml`)
- The files of the original installation, in one directory:

| File           | Size        | Contents                                                     |
|----------------|-------------|--------------------------------------------------------------|
| `SCRANTIC.SCR` | 295,952     | original Win16 executable (sounds + game-logic tables)       |
| `RESOURCE.MAP` | 1,461       | resource index                                               |
| `RESOURCE.001` | 1,175,645   | images (BMP/SCR), palette, animation scripts (TTM/ADS)       |

## Usage

```sh
cargo run --release -- /path/to/install            # 640x480 window
cargo run --release -- /path/to/install --scale 2
cargo run --release -- /path/to/install --saver    # fullscreen, exits on any input
```

Keys (windowed mode): `F`/`F11` toggles fullscreen, `Esc`/`Q` quits.

Story progress and options are stored in `SCRANTIC.INI` in the user configuration directory
(e.g. `~/Library/Application Support/open-scrantic/` on macOS), using the same keys as the
original: `Background`, `Clouds`, `Waves`, `Sounds`, `NumDays`, `Introduction`,
`CurrentYear/Month/Day` and `StartTime`.

## Development

```sh
OPEN_SCRANTIC_DATA=/path/to/install cargo test                  # tests against the real data
OPEN_SCRANTIC_DEBUG=1 cargo run --release -- /path/to/install    # prints each session's plan
cargo run --release --example snapshot -- /path/to/install /tmp/out 2000 100 1000 42   # headless PNG render
```

Architecture:

- `src/assets/` — reading the original files: the Dynamix resource archive (RLE/LZW, BMP, SCR,
  PAL, TTM, ADS) and the NE executable (`WAVESFXn` sounds and data-segment tables).
- `src/engine/` — the engine: TTM VM (`ttm.rs`), ADS VM and scene scheduler (`ads.rs`), island
  and ambient animation (`island.rs`), walking (`walk.rs`), story planning (`mod.rs`), the original
  random number generator (`rng.rs`), drawing primitives (`gfx.rs`), INI settings (`config.rs`).
- `src/main.rs` — window (winit + softbuffer), clocks and input; `src/audio.rs` — sound (rodio).

What was recovered through reverse engineering is documented in [`docs/ENGINE.md`](docs/ENGINE.md).
