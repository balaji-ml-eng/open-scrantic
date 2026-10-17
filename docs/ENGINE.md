# How Johnny Castaway works

For how the Rust code is organized, see [`ARCHITECTURE.md`](ARCHITECTURE.md).

Reverse-engineering notes on `SCRANTIC.SCR` (Win16 NE executable, Borland C++ 1991, 13 code
segments + DGROUP). Addresses are written as `segment:offset`; `DS:xxxx` is the data segment.

## Resource files

- `RESOURCE.MAP`: 6-byte header, volume name (13 bytes), `u16` count, `(u32 hash, u32 offset)`
  entries. In `RESOURCE.001`, each resource is: name (13 bytes), `u32` size, data.
- Each resource is a sequence of `TAG:` chunks + `u32` size (bit 31 = container).
- Compression (`BIN:`, `SCR:`, `TT3:`): method byte + `u32` size; 1 = RLE, 2 = LZW
  (9–12-bit LSB-first codes; code 0x100 resets the dictionary and realigns the bit stream).
- Images are 4 bpp (high nibble first); index 0 (magenta) is transparent. The `VGA:` chunks are
  irrelevant (noise); the `JOHNCAST.PAL` palette only uses 16 colors (6 bits per channel).

## TTM: animation scripts (`seg12:040c`)

- `u16` instructions: the low nibble is the number of `i16` arguments (0xF = NUL-terminated string,
  padded to an even size). The script is split into *frames* at each `0x0FF0` (UPDATE); `PAG:`
  gives the frame count.
- Each `0x1111 tag` defines a **scene** starting at that frame. `0x1201` (GOTO) resolves to the
  frame whose **first** opcode is `0x1101/0x1111` with that tag.
- A frame executed for the first time has side effects (loading images/screens, sounds, random
  delay); afterwards it only redraws (flag `+0x0E` of the scene record).
- Opcodes used: sprites `0xA5x4` (bits 4–7 = mirroring: 2 horizontal, 1 vertical), line
  `0xA0A4` (end point not drawn, like `LineTo`), rectangle `0xA104`/`0xA114`, ellipse
  `0xA404/0xA424` (radius = h/2), pixel `0xA002`, clip `0x4004` (inclusive), "copy to background"
  `0x4204`, delay `0x1021` (16 ms ticks), random delay `0x2022` (`a + rand % (b - a)`),
  sound `0xC051`, end of scene `0x0110`, save/restore zone `0xA054/0xA064`.
- Drawing, clip and copy coordinates get the island offset (`origin.x + 1`, `origin.y`),
  plus 272 px for scenes with flag `0x400`.

## ADS: sequencing (`seg10`)

- Sections (tags) separated by `0xFFFF`. Block language: conditions `0x13x0`
  (`0x1330` never played, `0x1350` just finished, `0x1360` stopped, `0x1370` running),
  `AND 0x1420`/`OR 0x1430` evaluated left to right with short-circuiting,
  `ELSE 0x1500`, `ENDIF 0x1510`, `RANDOM 0x3010 … 0x30FF` (weights; `0x3020` = do nothing).
- `0x2005 slot tag n`: starts a scene (`n=0` once, `n>0` n times, `n<0` for −n ticks);
  `0x2010` stops it; `0x4000/0x4010` brings the scene to the front/back; `0xF010` stops a tag;
  `0xF200` starts another tag in parallel.
- Scheduler, on every step: propagates tag state to their scenes, **re-evaluates the whole script
  of every active tag**, then executes (draws) the current frame of every scene in drawing order,
  advancing it when its delay has elapsed. A tag is "done" when none of its scenes is still running
  (scenes that GOTO their own frame count as infinite loops and do not block).

## Story (`seg4`)

- Catalogue of 80 scenes at `DS:1756` (17 bytes each): weight, start and end spots/headings,
  duration, time-of-day window, story day, flags (role: 1 opening, 2 middle, 4 final,
  8 repeatable; island modifiers: tide, night, no island, no raft, frozen waves; island position
  constraints), ADS id and tag. Scenes with ADS `0xFF` are standing pauses (3–7 s).
- Each **session** picks: with a 10% chance the chapter of the day, a final scene, an opening scene
  and middle scenes drawn by weight until a budget of 300 duration units is spent, respecting
  compatibility with the island position already decided.
- **Day/night cycle**: 16 half-hours starting at `StartTime` (9:00 by default): 12 of day
  (random `OCEAN0x.SCR`) and 4 of night (`NIGHT.SCR`).
- **Chapters** (11): `NumDays` advances one per calendar day, as long as the current chapter has
  already been shown; after the 11th it starts over. The raft grows with the day (5 stages).
- **Holidays**: Halloween (Oct 29–31), Christmas (Dec 23–25), New Year (Dec 29–Jan 1),
  St. Patrick's (Mar 15–17).

## Walking

Six spots on the island; routes per (from, to) pair at `DS:0A96`, with alternative paths drawn by
percentage; 34 frame paths `(sprite|mirror, x, y)`; turning the shortest way (random at 180°).
When Johnny's feet are above `origin.y + 293` and he crosses the trunk, the trunk is redrawn in
front of him.

## Timing and randomness

- 16 ms tick (`4 × 1000/(13×18)` ms) for delays and timers; the story step runs on the 50 ms
  `WM_TIMER` (actual 55 ms granularity on Windows 3.1). Both are emulated.
- Additive lagged generator over a 56-word table (`DS:1CE6`), warmed up by discarding
  `time() & 0xFFFF` numbers.
- Sounds: `WAVESFX1..25` resources (11 and 13 are missing), 8-bit, 11,025 Hz; a new sound cuts
  the previous one. `WAVESFX25` is the chime played when the chapter of the day starts.

## Deliberate differences from the original

- The original draws with dirty rectangles across three buffers; here every step recomposes the
  whole frame (background + that step's drawing). The result is the same, except for small
  artifacts of the original (e.g. shore waves "reverting" to their first frame when something
  passes over them).
- The title screen stays up for 2 s (on 1993 hardware it was visible while the first session loaded).
- Ellipses and lines use their own rasterization, which may differ by 1 px from GDI.
- The configuration dialog and the screensaver password were not reimplemented (options can be
  edited in the INI). Features this screensaver does not use (animated palettes, 256 colors, ADS
  variables) were not implemented.
