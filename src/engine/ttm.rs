//! TTM animation scripts: per-TTM context and scene records, and the frame interpreter
//! (seg12:040c and its dispatchers).

use std::rc::Rc;

use super::Env;
use super::gfx::{Canvas, Clip, HEIGHT, WIDTH};
use crate::assets::resource::{Bmp, Op, Ttm};

/// A loaded TTM script with its bitmap slots (shared by all scenes of the script).
pub struct TtmCtx {
    pub ttm: Ttm,
    bmps: [Option<Rc<Bmp>>; 6],
    saved_zone: Option<(Clip, Canvas)>,
}

impl TtmCtx {
    pub fn new(ttm: Ttm) -> Self {
        Self {
            ttm,
            bmps: Default::default(),
            saved_zone: None,
        }
    }
}

/// Scene states (+0x2F in the original record).
pub mod state {
    pub const IDLE: u16 = 0;
    pub const ONCE: u16 = 1;
    pub const REPEAT: u16 = 2;
    pub const TIMED: u16 = 3;
    pub const DONE: u16 = 4;
    pub const FROZEN: u16 = 5;
}

/// A scene: one tagged entry point (0x1111) of a TTM script, scheduled by ADS.
#[derive(Clone, Debug)]
pub struct Scene {
    pub ttm: usize,
    pub slot: u16,
    pub tag: u16,
    pub start: i32,
    pub goto: i32,
    pub cur: i32,
    pub looping: bool,
    /// Set once the current frame ran: later executions only redraw (no loads, sounds, timers).
    pub redraw: bool,
    pub bmp_slot: usize,
    pub fg: u8,
    pub bg: u8,
    pub clip_on: bool,
    pub clip: Clip,
    pub delay: i32,
    pub tag_state: u16,
    pub plays: u16,
    pub state: u16,
    pub repeats: i32,
    pub end_time: u32,
    pub next_time: u32,
}

impl Scene {
    pub fn new(ttm: usize, slot: u16, tag: u16, frame: usize) -> Self {
        let mut s = Self {
            ttm,
            slot,
            tag,
            start: frame as i32,
            goto: -1,
            cur: frame as i32,
            looping: false,
            redraw: false,
            bmp_slot: 0,
            fg: 15,
            bg: 15,
            clip_on: true,
            clip: Clip::FULL,
            delay: 0,
            tag_state: 0,
            plays: 0,
            state: state::IDLE,
            repeats: 0,
            end_time: 0,
            next_time: 0,
        };
        s.reset();
        s
    }

    /// seg10:0b3e
    pub fn reset(&mut self) {
        self.bmp_slot = 0;
        self.cur = self.start;
        self.goto = -1;
        self.fg = 15;
        self.bg = 15;
        self.delay = 0;
        self.next_time = 0;
        self.repeats = 0;
        self.plays = 0;
        self.redraw = false;
        self.state = state::IDLE;
        self.tag_state = 0;
        self.looping = false;
        self.clip_on = true;
        self.clip = Clip::FULL;
    }

    /// seg10:196e: advance to the next (or pending GOTO) frame once the delay elapsed.
    pub fn advance(&mut self, now: u32) -> bool {
        if self.delay != 0 {
            if self.next_time > now {
                return false;
            }
            self.next_time = now.wrapping_add(self.delay as u32);
        }
        self.redraw = false;
        if self.goto != -1 {
            self.cur = self.goto;
            self.goto = -1;
        } else {
            self.cur += 1;
        }
        true
    }
}

/// Side results of running one frame.
pub struct FrameResult {
    pub delay: Option<i32>,
    pub purge: bool,
    pub goto: Option<i32>,
}

fn clip_of(scene: &Scene) -> Clip {
    if scene.clip_on { scene.clip } else { Clip::FULL }
}

fn buffer<'b>(env: &'b mut Env<'_>, index: i32) -> &'b mut Canvas {
    match index {
        0 => env.front,
        2 => env.bg,
        _ => env.work,
    }
}

/// Runs frame `frame` of `ctx` for `scene`. Returns None if the frame does not exist.
pub fn run_frame(ctx: &mut TtmCtx, scene: &mut Scene, env: &mut Env<'_>) -> Option<FrameResult> {
    let frame = usize::try_from(scene.cur).ok()?;
    let ops: &[Op] = ctx.ttm.frames.get(frame)?;
    let ops = ops.to_vec();
    let skip = scene.redraw;
    let mut res = FrameResult {
        delay: None,
        purge: false,
        goto: None,
    };
    let (ox, oy) = env.offset;

    for op in &ops {
        let mut a = [op.arg(0), op.arg(1), op.arg(2), op.arg(3), op.arg(4), op.arg(5)];
        // Island offset (globals 0x46E6/0x46E4) for drawing, clip and copy-zone opcodes.
        if (0xA000..=0xAFFF).contains(&op.code) || op.code == 0x4004 || op.code == 0x4204 {
            match op.code {
                0x4004 | 0xA0A4 => {
                    a[0] += ox;
                    a[1] += oy;
                    a[2] += ox;
                    a[3] += oy;
                }
                0xA601 | 0xAF1F | 0xAF2F => {}
                _ => {
                    a[0] += ox;
                    a[1] += oy;
                }
            }
        }
        let clip = clip_of(scene);
        match op.code {
            0x0FF0 => break,
            // Sound
            0xC051 => {
                if !skip {
                    env.sounds.push(a[0] as u32);
                }
            }
            // Drawing: always executed, also when only redrawing the frame.
            0xA002 => env.work.pixel(a[0], a[1], scene.fg, clip),
            0xA0A4 => env.work.line(a[0], a[1], a[2], a[3], scene.fg, clip),
            0xA104 | 0xA114 => env
                .work
                .rect(a[0], a[1], a[2], a[3], scene.fg, scene.bg, op.code == 0xA104, clip),
            0xA404 | 0xA424 => {
                let r = a[3] >> 1;
                env.work.ellipse(a[0] + r, a[1] + r, r, scene.fg, scene.bg, clip);
            }
            0xA504 | 0xA514 | 0xA524 | 0xA534 => {
                let flags = ((op.code & 0xF0) >> 4) as u8;
                if let Some(bmp) = usize::try_from(a[3])
                    .ok()
                    .and_then(|s| ctx.bmps.get(s))
                    .and_then(Option::as_ref)
                    && let Some(sprite) = usize::try_from(a[2]).ok().and_then(|f| bmp.sprites.get(f))
                {
                    env.work.sprite(sprite, a[0], a[1], flags, clip);
                }
            }
            0xA054 | 0xA064 => {
                let (w, h) = (a[2], a[3]);
                if skip || w <= 0 || h <= 0 || a[0] + w > WIDTH || a[1] + h > HEIGHT {
                    continue;
                }
                if op.code == 0xA054 {
                    ctx.saved_zone = Some((Clip::rect(a[0], a[1], w, h), env.bg.clone()));
                } else if let Some((r, saved)) = &ctx.saved_zone {
                    env.work
                        .copy_rect_from(saved, r.x1, r.y1, r.x2 - r.x1 + 1, r.y2 - r.y1 + 1);
                }
            }
            // State
            0x0110 => res.purge = true,
            0x1021 => res.delay = Some(a[0]),
            0x2022 => {
                if !skip {
                    let span = (a[1] - a[0]) as u16;
                    let rem = env.rng.below(span) as i16;
                    res.delay = Some(a[0] + (rem as i32).abs());
                }
            }
            0x1051 => scene.bmp_slot = a[0].clamp(0, 5) as usize,
            0x1201 => res.goto = Some(ctx.ttm.frame_of_tag(a[0]).map_or(-1, |f| f as i32)),
            0x2002 => {
                scene.fg = a[0] as u8;
                scene.bg = a[1] as u8;
            }
            0x4004 => {
                scene.clip = Clip {
                    x1: a[0],
                    y1: a[1],
                    x2: a[2],
                    y2: a[3],
                };
                scene.clip_on = true;
            }
            0x4204 => {
                if !skip {
                    env.bg.copy_rect_from(env.work, a[0], a[1], a[2], a[3]);
                }
            }
            0xB606 => {
                if !skip {
                    let src = buffer(env, a[4]).clone();
                    buffer(env, a[5]).copy_rect_from(&src, a[0], a[1], a[2], a[3]);
                }
            }
            0x0080 => {
                if !skip {
                    ctx.bmps[scene.bmp_slot] = None;
                }
            }
            0xF01F => {
                if !skip && let Some(name) = &op.text {
                    match env.archive.scr(name) {
                        Ok(pic) => {
                            env.work.load_picture(&pic);
                            env.bg.px.copy_from_slice(&env.work.px);
                        }
                        Err(e) => eprintln!("warning: {e}"),
                    }
                }
            }
            0xF02F => {
                if !skip && let Some(name) = &op.text {
                    let bmp = if name.eq_ignore_ascii_case("JOHNWALK.BMP") {
                        Some(Rc::clone(env.johnwalk))
                    } else {
                        match env.archive.bmp(name) {
                            Ok(b) => Some(Rc::new(b)),
                            Err(e) => {
                                eprintln!("warning: {e}");
                                None
                            }
                        }
                    };
                    ctx.bmps[scene.bmp_slot] = bmp;
                }
            }
            // Tags, palettes, image regions, frame setup: no effect on this platform.
            _ => {}
        }
    }
    Some(res)
}
