# open-scrantic

### A 1:1 Rust reimplementation of *Screen Antics: Johnny Castaway* (Sierra/Dynamix, 1993).

**The original Windows 3.1 screensaver, rebuilt from its disassembly and running natively.**

The binary contains **only the logic**: all art, scripts, sounds and game tables are read from
the **original installed files**. No game data is shipped in this repository.

[Quick start](#quick-start) · [How it works](#how-it-works) · [FAQ](#faq) · [Development](#development) ·
[Credits](#credits)

> [!IMPORTANT]
> You must supply your own copy of the original game files. See [Legal notice](#legal-notice).

## Quick start

```sh
cargo run --release -- /path/to/install
```

Replace `/path/to/install` with the directory holding `SCRANTIC.SCR`, `RESOURCE.MAP` and
`RESOURCE.001`. Details on requirements and options follow.

## How it works

```text
SCRANTIC.SCR ──► assets::exe       ──► sounds, scene catalogue, walk routes, RNG seed
RESOURCE.MAP ─┐
RESOURCE.001 ─┴► assets::resource  ──► images, palette, TTM/ADS scripts
                                              │
                    engine: story planner ──► ADS scheduler ──► TTM frames ──► canvas
                    (calendar, chapters)      (z-order, tags)   (draw, sound)    │
                                                                      winit + softbuffer + rodio
```

The original executable was disassembled and the disassembly is the ground truth for behavior:
code comments cite original addresses (`seg10:1acb`, `DS:1756`). Formats, VM semantics, story
rules, timing and deliberate deviations are recorded in [`docs/ENGINE.md`](docs/ENGINE.md).

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
cargo run --release -- /path/to/install
```

There are no options. It always runs fullscreen, like the original screensaver: any key, mouse click
or mouse movement exits. `GAME_DIR` defaults to the current directory. Sound and visual options live
in `SCRANTIC.INI` (see below).

## Configuration

Options and story progress are stored in `SCRANTIC.INI`, section `[ScreenSaver.ScreenAntics]`, using
the same keys as the original. Location: the user configuration directory
(`~/Library/Application Support/open-scrantic/` on macOS, `~/.config/open-scrantic/` on Linux,
`%APPDATA%\open-scrantic\` on Windows). The file is rewritten on clean exit only, so edit it while
the program is closed. Keys are case-insensitive; unknown keys and non-numeric values are ignored.

**Options** (`0` = off, any other value = on; default `1`):

| Key          | Effect when off                                                         |
|--------------|-------------------------------------------------------------------------|
| `Background` | no island or ocean background composition                               |
| `Clouds`     | no clouds in the sky                                                    |
| `Waves`      | no shore waves or ocean whitecaps                                       |
| `Sounds`     | sound effects muted                                                     |

**Story state** (maintained by the engine; edit only to force a chapter or time of day):

| Key            | Default | Meaning                                                              |
|----------------|---------|----------------------------------------------------------------------|
| `NumDays`      | `0`     | story chapter of the day, 1–11; advances once per calendar day       |
| `Introduction` | `0`     | next chapter to show; the chapter plays when it is `<= NumDays`      |
| `CurrentYear`, `CurrentMonth`, `CurrentDay` | `0` | date of the last run, used to detect a new day |
| `StartTime`    | `900`   | start of the 16-half-hour day/night cycle, as `HHMM`                 |

An out-of-range `NumDays` (below 2 or above 11) restarts the story at chapter 1 on the next day
change. Delete the file to reset everything. Details: [`docs/ENGINE.md`](docs/ENGINE.md).

## Release build

`cargo build --release` produces an optimized, portable binary at `target/release/open-scrantic`
(fat LTO, a single codegen unit, `panic = "abort"`, stripped symbols).

For a binary tuned to the build machine only (it may not run on older CPUs):

```sh
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

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

## FAQ

**Why do I need the original files?**
The art, animation scripts and sounds are copyrighted by their owners. This project reimplements
only the engine and reads those assets at runtime.

**Does it work with any build of the game?**
No. The loader validates known strings in `SCRANTIC.SCR` and rejects other builds of the executable.

**How faithful is it?**
Behavior follows the disassembly, including the original 16-bit arithmetic and random number
generator. Intentional differences (for example, full-frame redraw instead of dirty rectangles)
are listed in [`docs/ENGINE.md`](docs/ENGINE.md).

**Where is my progress saved?**
In `SCRANTIC.INI` in the user configuration directory, on clean exit only.

## Documentation

- [`docs/ENGINE.md`](docs/ENGINE.md) — recovered formats, VM semantics, story rules, timing.
- [`CLAUDE.md`](CLAUDE.md) — architecture and conventions for contributors and coding agents.

## Contributing

Run these before sending changes; warnings fail the build (`Cargo.toml` denies them):

```sh
cargo clippy --all-targets
cargo fmt --check
OPEN_SCRANTIC_DATA=/path/to/install cargo test
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org/). Never commit game
data. When porting more logic, cite the original address in the comment, as existing code does.

## Legal notice

*Screen Antics: Johnny Castaway* is a trademark and copyright of its respective owners (Sierra /
Dynamix). This is an independent, non-commercial interoperability project, not affiliated with or
endorsed by them. It does not include or distribute any original game data.

## Credits

The original `SCRANTIC.SCR` was disassembled and decompiled with
[REA: Reverse Engineer Anything](https://github.com/morluto/rea). The recovered behavior
documented in `docs/ENGINE.md` comes from that analysis.

## License

The source code is released under the [MIT License](LICENSE), Copyright (c) 2026 Glauber Lima.
The license covers this repository only; the original game and its data remain the property of
their owners (see the [legal notice](#legal-notice)).
