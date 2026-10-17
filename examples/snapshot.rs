//! Headless run: steps the engine with a fixed wall clock and saves PNG snapshots.
//! Usage: cargo run --example snapshot -- GAME_DIR OUT_DIR [STEPS] [EVERY] [HHMM] [SEED] [STORY_DAY] [SCENE|list]

use std::fs;
use std::path::Path;

use open_scrantic::assets::GameData;
use open_scrantic::engine::config::Config;
use open_scrantic::engine::gfx::{HEIGHT, WIDTH};
use open_scrantic::engine::{Clock, Engine, STEP_MS, TICK_MS};

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn png(path: &Path, w: usize, h: usize, rgb: &[u8]) {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(u8::from(last));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |tag: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = tag.to_vec();
        c.extend_from_slice(data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    fs::write(path, out).unwrap();
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2);
}

/// Optional positional argument `i`: `default` when absent, an error when present but invalid.
fn arg<T: std::str::FromStr>(a: &[String], i: usize, name: &str, default: T) -> T {
    match a.get(i) {
        None => default,
        Some(v) => v.parse().unwrap_or_else(|_| fail(&format!("invalid {name}: {v}"))),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = Path::new(a.get(1).unwrap_or_else(|| fail("missing GAME_DIR")));
    let out = Path::new(a.get(2).unwrap_or_else(|| fail("missing OUT_DIR")));
    let steps: u64 = arg(&a, 3, "STEPS", 400);
    let every: u64 = arg(&a, 4, "EVERY", 50);
    let hhmm: i32 = arg(&a, 5, "HHMM", 1000);
    let seed: u64 = arg(&a, 6, "SEED", 12345);
    let story_day: i32 = arg(&a, 7, "STORY_DAY", 0);
    if every == 0 {
        fail("EVERY must be at least 1");
    }
    if !(0..2400).contains(&hhmm) || hhmm % 100 >= 60 {
        fail("HHMM must be a time between 0000 and 2359");
    }
    if !(0..=11).contains(&story_day) {
        fail("STORY_DAY must be between 0 and 11");
    }
    fs::create_dir_all(out).unwrap();
    let clock = Clock {
        year: 1993,
        month: 6,
        day: 1,
        hour: hhmm / 100,
        minute: hhmm % 100,
    };
    let mut config = Config::default();
    if story_day > 0 {
        (config.num_days, config.intro) = (story_day, story_day);
        (config.year, config.month, config.day) = (clock.year, clock.month, clock.day);
    }
    let mut engine = Engine::new(GameData::load(dir).unwrap(), config, seed).unwrap();
    match a.get(8).map(String::as_str) {
        Some("list") => {
            for i in 0..engine.scene_count() {
                println!("{i}: {}", engine.scene_label(i));
            }
            return;
        }
        Some(v) => {
            let i: usize = v.parse().unwrap_or_else(|_| fail(&format!("invalid SCENE: {v}")));
            if i >= engine.scene_count() {
                fail(&format!("SCENE must be below {}", engine.scene_count()));
            }
            engine.force_scene(i);
        }
        None => {}
    }
    let mut ticks = 0u64;
    for step in 1..=steps {
        while (ticks + 1) * TICK_MS <= step * STEP_MS {
            ticks += 1;
            engine.tick();
        }
        engine.step(&clock);
        let sounds = engine.take_sounds();
        if !sounds.is_empty() {
            println!("step {step}: sounds {sounds:?}");
        }
        if step % every == 0 {
            let rgb: Vec<u8> = engine
                .frame()
                .px
                .iter()
                .flat_map(|&i| engine.palette[i as usize])
                .collect();
            png(
                &out.join(format!("step{step:05}.png")),
                WIDTH as usize,
                HEIGHT as usize,
                &rgb,
            );
        }
    }
}
