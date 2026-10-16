//! Johnny walking between the six island spots (seg4:0094 and friends).

use super::gfx::Canvas;
use super::gfx::{Clip, FLIP_H};
use super::island::Island;
use super::rng::Rng;
use crate::assets::ExeData;
use crate::assets::exe::WalkFrame;
use crate::assets::resource::Bmp;

pub struct Walk {
    pub spot: i32,
    pub target: i32,
    pub hdg: i32,
    pub target_hdg: i32,
    /// Ticks until the next walk frame (decremented by the 16 ms tick).
    pub timer: u32,
    route: Option<(u8, u8)>,
    path: Option<(usize, usize)>,
    path_start_hdg: i32,
    path_end_hdg: i32,
    path_end_spot: i32,
    frame: Option<WalkFrame>,
}

impl Default for Walk {
    fn default() -> Self {
        Self {
            spot: 1,
            target: 1,
            hdg: 0,
            target_hdg: -1,
            timer: 0,
            route: None,
            path: None,
            path_start_hdg: 0,
            path_end_hdg: 0,
            path_end_spot: 0,
            frame: None,
        }
    }
}

impl Walk {
    /// seg4:0074
    pub fn reset(&mut self) {
        self.route = None;
        self.path = None;
        self.path_start_hdg = self.hdg;
    }

    /// seg4:0b64 – set the destination; true once Johnny stands there with that heading.
    pub fn go_to(&mut self, spot: i32, hdg: i32) -> bool {
        if spot != 0 {
            self.target = spot;
        }
        if hdg != -1 {
            self.target_hdg = hdg;
        }
        (spot == 0 || self.spot == spot) && (hdg == -1 || self.hdg == hdg)
    }

    fn turn_frame(exe: &ExeData, table: &[u8; 7], spot: i32, hdg: i32) -> Option<WalkFrame> {
        let path = *table.get(spot as usize)? as usize;
        exe.paths.get(path)?.get(hdg as usize).copied()
    }

    /// One walk step; draws Johnny (and the trunk in front of him when behind the palm tree).
    pub fn step(&mut self, exe: &ExeData, rng: &mut Rng, sprites: &Bmp, island: &Island, work: &mut Canvas) {
        if self.timer == 0 {
            self.advance(exe, rng);
        }
        let Some(f) = self.frame else { return };
        let Some(s) = sprites.sprites.get(f.sprite as usize) else {
            return;
        };
        let (x, y) = (island.x + f.x, island.y + f.y);
        work.sprite(s, x, y, if f.mirror { FLIP_H } else { 0 }, Clip::FULL);
        island.trunk_overlay(work, (x, y, s.w, s.h));
        self.timer = 4;
    }

    fn advance(&mut self, exe: &ExeData, rng: &mut Rng) {
        // Pick the next path segment towards the target.
        if self.route.is_none() && self.path.is_none() && self.spot != self.target {
            self.route = Some((self.spot as u8, self.target as u8));
        }
        if let Some(key) = self.route
            && self.path.is_none()
            && let Some(list) = exe.routes.get(&key)
        {
            let Some(mut i) = list.iter().position(|&w| w == -7 || w as i32 == -self.spot) else {
                return;
            };
            if list[i] == -7 {
                return;
            }
            i += 1;
            let mut r = rng.below(100) as i32;
            loop {
                r -= list.get(i + 1).copied().unwrap_or(0) as i32;
                if r <= 0 || list.get(i + 2).is_none_or(|&w| w < 0) {
                    break;
                }
                i += 2;
            }
            let id = list[i].max(0) as usize;
            let info = exe.path_info.get(id).copied().unwrap_or([0; 3]);
            self.path = (id > 0 && exe.paths.get(id).is_some_and(|p| !p.is_empty())).then_some((id, 0));
            self.path_start_hdg = info[0] as i32;
            self.path_end_hdg = info[1] as i32;
            self.path_end_spot = info[2] as i32;
            if self.path.is_none() {
                self.hdg = self.path_end_hdg;
                self.spot = self.path_end_spot;
                if self.spot == self.target {
                    self.route = None;
                }
            }
        }

        // Final heading at the destination.
        if self.target_hdg != -1 && self.spot == self.target && self.hdg == self.path_start_hdg {
            if self.path_start_hdg != self.target_hdg {
                self.path_start_hdg = self.target_hdg;
            } else {
                self.frame = Self::turn_frame(exe, &exe.stand_turns, self.spot, self.hdg);
                self.target_hdg = -1;
            }
        }

        // Turn on the spot, the shortest way (random when facing backwards).
        if self.hdg != self.path_start_hdg {
            let d = self.hdg - self.path_start_hdg;
            let dir = if d == 4 || d == -4 {
                if rng.next_u16() & 1 != 0 { -1 } else { 1 }
            } else if d < -4 || (d > 0 && d < 4) {
                -1
            } else {
                1
            };
            self.hdg = (self.hdg + dir) & 7;
            let table = if self.spot == self.target {
                &exe.stand_turns
            } else {
                &exe.walk_turns
            };
            self.frame = Self::turn_frame(exe, table, self.spot, self.hdg);
        }

        // Walk along the path once facing its direction.
        if let Some((id, idx)) = self.path
            && self.hdg == self.path_start_hdg
        {
            let frames = &exe.paths[id];
            self.frame = frames.get(idx).copied();
            if idx + 1 >= frames.len() {
                self.hdg = self.path_end_hdg;
                self.spot = self.path_end_spot;
                self.path = None;
                if self.spot == self.target {
                    self.route = None;
                }
            } else {
                self.path = Some((id, idx + 1));
            }
        }
    }
}
