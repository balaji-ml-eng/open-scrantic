//! Presentation of the 640x480 frame: optional dedithering and scaling to the window.
//! Nothing here touches the engine; it only changes what reaches the screen (documented deviation).

use crate::engine::gfx::{HEIGHT, WIDTH};

const W: usize = WIDTH as usize;
const H: usize = HEIGHT as usize;

fn pack(c: [u8; 3]) -> u32 {
    (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

/// RGB frame from the palette indices, unchanged.
pub fn plain(px: &[u8], pal: &[[u8; 3]; 256]) -> Vec<u32> {
    px.iter().map(|&i| pack(pal[i as usize])).collect()
}

/// Blends two colors in linear light (gamma 2.2) so the average of a checkerboard keeps its brightness.
fn average(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    let mix = |x: u8, y: u8| {
        let (x, y) = ((x as f32 / 255.0).powf(2.2), (y as f32 / 255.0).powf(2.2));
        (((x + y) / 2.0).powf(1.0 / 2.2) * 255.0).round() as u8
    };
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

/// RGB frame where every 2x2 checkerboard block (`a b / b a`, a != b) is replaced by the average of
/// its two colors. The dither of the original is this pattern in two palette indices, so detection
/// is exact index equality. A pixel in several blocks keeps the partner of the first one found.
pub fn dedither(px: &[u8], pal: &[[u8; 3]; 256]) -> Vec<u32> {
    let mut partner: Vec<Option<u8>> = vec![None; W * H];
    for y in 0..H - 1 {
        for x in 0..W - 1 {
            let i = y * W + x;
            let (a, b, c, d) = (px[i], px[i + 1], px[i + W], px[i + W + 1]);
            if a == d && b == c && a != b {
                for (j, q) in [(i, b), (i + 1, a), (i + W, a), (i + W + 1, b)] {
                    partner[j].get_or_insert(q);
                }
            }
        }
    }
    let mut pairs = vec![u32::MAX; 256 * 256];
    px.iter()
        .zip(&partner)
        .map(|(&p, &q)| match q {
            None => pack(pal[p as usize]),
            Some(q) => {
                let slot = &mut pairs[p as usize * 256 + q as usize];
                if *slot == u32::MAX {
                    *slot = pack(average(pal[p as usize], pal[q as usize]));
                }
                *slot
            }
        })
        .collect()
}

/// Draws `src` (640x480 RGB) into `dst` (`w` x `h`) by pixel replication, as large as fits and
/// centered, black elsewhere.
pub fn scale(src: &[u32], dst: &mut [u32], w: usize, h: usize) {
    dst.fill(0);
    let k = (w as f64 / W as f64).min(h as f64 / H as f64);
    let (dw, dh) = ((W as f64 * k) as usize, (H as f64 * k) as usize);
    if dw == 0 || dh == 0 {
        return;
    }
    let (ox, oy) = ((w - dw) / 2, (h - dh) / 2);
    let cols: Vec<usize> = (0..dw).map(|x| (x * W / dw).min(W - 1)).collect();
    for y in 0..dh {
        let row = &src[(y * H / dh).min(H - 1) * W..][..W];
        let out = &mut dst[(y + oy) * w + ox..][..dw];
        for (d, &sx) in out.iter_mut().zip(&cols) {
            *d = row[sx];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> [[u8; 3]; 256] {
        let mut p = [[0u8; 3]; 256];
        p[1] = [255, 255, 255];
        p[2] = [0, 0, 255];
        p
    }

    fn checker(a: u8, b: u8) -> Vec<u8> {
        (0..W * H)
            .map(|i| if (i % W + i / W).is_multiple_of(2) { a } else { b })
            .collect()
    }

    #[test]
    fn checkerboard_becomes_one_color() {
        let out = dedither(&checker(0, 2), &pal());
        assert!(out.iter().all(|&c| c == out[0]));
        assert_ne!(out[0], pack(pal()[0]));
        assert_ne!(out[0], pack(pal()[2]));
    }

    #[test]
    fn solid_lines_and_lone_pixels_are_kept() {
        let mut px = vec![1u8; W * H];
        px[10 * W + 10] = 2; // lone pixel
        for x in 0..W {
            px[20 * W + x] = 2; // one-pixel line
        }
        assert_eq!(dedither(&px, &pal()), plain(&px, &pal()));
    }

    #[test]
    fn letterbox_is_centered_and_extreme_sizes_work() {
        let rgb = vec![pack([9, 9, 9]); W * H];
        let (w, h) = (1000, 480);
        let mut dst = vec![1; w * h];
        scale(&rgb, &mut dst, w, h);
        assert_eq!(dst[0], 0);
        assert_eq!(dst[w / 2], pack([9, 9, 9]));
        for (w, h) in [(1, 1), (7680, 4320)] {
            scale(&rgb, &mut vec![0; w * h], w, h);
        }
    }
}
