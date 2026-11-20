//! Picks two background colors from an album cover.
//!
//! Pixels are grouped into 12 hue buckets weighted by how colorful they are.
//! The strongest bucket becomes the main color; the strongest bucket with a
//! clearly different hue becomes the second. Mostly-gray covers fall back to
//! their average color plus a darker shade of it.

const BINS: usize = 12;

type Rgb = (f32, f32, f32);

pub fn extract(bytes: &[u8]) -> Option<[String; 2]> {
    let img = image::load_from_memory(bytes).ok()?.thumbnail(32, 32).to_rgb8();

    let mut bins = [(0f32, 0f32, 0f32, 0f32); BINS]; // weight, r, g, b
    let (mut sum, mut n) = ((0f32, 0f32, 0f32), 0f32);

    for p in img.pixels() {
        let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
        sum = (sum.0 + r, sum.1 + g, sum.2 + b);
        n += 1.0;

        let (h, s, v) = hsv(r, g, b);
        if s < 0.2 || v < 0.15 || v > 0.97 {
            continue;
        }
        let w = s * v;
        let bin = &mut bins[((h / 30.0) as usize).min(BINS - 1)];
        *bin = (bin.0 + w, bin.1 + r * w, bin.2 + g * w, bin.3 + b * w);
    }
    if n == 0.0 {
        return None;
    }

    let total: f32 = bins.iter().map(|b| b.0).sum();
    if total < n * 0.04 {
        let avg = (sum.0 / n, sum.1 / n, sum.2 / n);
        return Some([hex(avg), hex(shade(avg, 0.5))]);
    }

    let mut order: Vec<usize> = (0..BINS).collect();
    order.sort_by(|a, b| bins[*b].0.total_cmp(&bins[*a].0));
    let avg = |i: usize| {
        let b = bins[i];
        (b.1 / b.0, b.2 / b.0, b.3 / b.0)
    };

    let first = order[0];
    let c1 = avg(first);
    let c2 = order[1..]
        .iter()
        .copied()
        .find(|&i| bins[i].0 > bins[first].0 * 0.15 && hue_distance(i, first) >= 2)
        .map(avg)
        .unwrap_or_else(|| shade(c1, 0.5));

    Some([hex(c1), hex(c2)])
}

fn hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let max = r.max(g).max(b);
    let d = max - r.min(g).min(b);
    let s = if max == 0.0 { 0.0 } else { d / max };
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, max)
}

fn hue_distance(a: usize, b: usize) -> usize {
    let d = a.abs_diff(b);
    d.min(BINS - d)
}

fn shade(c: Rgb, f: f32) -> Rgb {
    (c.0 * f, c.1 * f, c.2 * f)
}

fn hex(c: Rgb) -> String {
    let ch = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", ch(c.0), ch(c.1), ch(c.2))
}
