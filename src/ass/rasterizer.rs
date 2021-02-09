use std::convert::TryInto;

use aligned_box::AlignedBox;
use enumflags2::BitFlags;

use super::bitmap::BitmapEngine;
use super::outline::{Outline, Rect, Segment, Vector};
use super::utils::{i64_mul, u32_abs as abs};

pub use super::polyline::Segment as PolylineSegment;
use super::polyline::SegmentFlag as SegFlag;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
enum FillFlag {
    Solid = 1,
    Complex = 2,
    Reverse = 4,
    Generic = 8,
}

#[inline]
fn get_fill_flags(line: &[PolylineSegment], mut winding: i32) -> BitFlags<FillFlag> {
    if line.len() == 0 {
        return if winding != 0 {
            BitFlags::from_flag(FillFlag::Solid)
        } else {
            BitFlags::empty()
        };
    } else if line.len() > 1 {
        return FillFlag::Complex | FillFlag::Generic;
    }

    let line = &line[0];
    let test = SegFlag::UlDr | SegFlag::ExactLeft;
    if !line.flags.contains(test) == !line.flags.contains(SegFlag::Dn) {
        winding += 1;
    }

    match winding {
        0 => FillFlag::Complex | FillFlag::Reverse,
        1 => FillFlag::Complex.into(),
        _ => FillFlag::Solid.into(),
    }
}

fn polyline_split_horz(
    src: &[PolylineSegment],
    n_src: [usize; 2],
    mut winding: [i32; 2],
    x: i32,
) -> ([Vec<PolylineSegment>; 2], [[usize; 2]; 2], [i32; 2]) {
    let mut dst = [Vec::new(), Vec::new()];
    let mut n_dst = [[0; 2]; 2];

    for (i, seg) in src.iter().enumerate() {
        let group = (i >= n_src[0]) as usize;

        let mut delta = 0;
        if seg.y_min == 0 && seg.flags.contains(SegFlag::ExactTop) {
            delta = if seg.a < 0 { 1 } else { -1 };
        }
        if seg.check_right(x) {
            winding[group] += delta;
            if seg.x_min >= x {
                continue;
            }
            let mut new = *seg;
            new.x_max = new.x_max.min(x);
            dst[0].push(new);
            n_dst[0][group] += 1;
            continue;
        }
        if seg.check_left(x) {
            let mut new = *seg;
            new.move_x(x);
            dst[1].push(new);
            n_dst[1][group] += 1;
            continue;
        }
        if seg.flags.contains(SegFlag::UlDr) {
            winding[group] += delta;
        }
        let (a, b) = seg.split_horz(x);
        dst[0].push(a);
        n_dst[0][group] += 1;
        dst[1].push(b);
        n_dst[1][group] += 1;
    }

    (dst, n_dst, winding)
}

#[allow(unused_variables)]
fn polyline_split_vert(
    src: &[PolylineSegment],
    n_src: [usize; 2],
    winding: [i32; 2],
    t: i32,
) -> ([Vec<PolylineSegment>; 2], [[usize; 2]; 2], [i32; 2]) {
    todo!()
}

#[derive(Debug, Copy, Clone)]
struct OutlineSegment {
    r: Vector,
    r2: i64,
    er: i64,
}

impl OutlineSegment {
    fn new(beg: Vector, end: Vector, outline_error: i32) -> Self {
        let Vector { x, y } = (end - beg).checked_abs().unwrap();

        Self {
            r: Vector { x, y },
            r2: i64_mul(x, x) + i64_mul(y, y),
            er: i64_mul(outline_error, x.max(y)),
        }
    }

    fn subdivide(&self, beg: Vector, pt: Vector) -> bool {
        let Vector { x, y } = pt - beg;
        let pdr = i64_mul(self.r.x, x) + i64_mul(self.r.y, y);
        let pcr = i64_mul(self.r.x, y) + i64_mul(self.r.y, x);
        (pdr < -self.er) || (pdr > self.r2 + self.er) || (pcr.checked_abs().unwrap() > self.er)
    }
}

pub struct RasterizerData {
    outline_error: i32,

    bbox: Rect,

    linebuf: [Vec<PolylineSegment>; 2],
    n_first: usize,

    tile: AlignedBox<[u8]>,
}

#[inline]
fn upper_mul(a: u32, b: u32) -> u32 {
    ((a as u64 * b as u64) >> 32) as u32
}

#[inline]
fn compute_scale(max_ab: u32) -> u32 {
    let mut scale = upper_mul(0x5333_3333, upper_mul(max_ab, max_ab));
    scale += 0x8810_624D - upper_mul(0xBBC6A7EF, max_ab);
    scale
}

impl RasterizerData {
    pub fn new(tile_order: usize, outline_error: i32) -> Self {
        Self {
            outline_error,
            bbox: Rect::default(),
            linebuf: [Vec::new(), Vec::new()],
            n_first: 0,
            tile: AlignedBox::slice_from_default(32, 1 << (2 * tile_order)).unwrap(),
        }
    }

    pub fn set_outline(&mut self, path: &Outline, extra: bool) {
        if !extra {
            self.bbox.reset();
            self.n_first = 0;
        }
        // self.size[0] = self.n_first;

        // #[cfg(debug)]
        for point in path.points() {
            debug_assert!(point.x.abs() <= Outline::MAX_COORD);
            debug_assert!(point.y.abs() <= Outline::MAX_COORD);
        }

        for segment in path.segments() {
            match segment {
                Segment::LineSegment(pt0, pt1) => self.add_line(pt0, pt1),
                Segment::QuadSpline(pt0, pt1, pt2) => self.add_quadratic([pt0, pt1, pt2]),
                Segment::CubicSpline(pt0, pt1, pt2, pt3) => self.add_cubic([pt0, pt1, pt2, pt3]),
            };
        }

        for k in self.n_first..self.linebuf[0].len() {
            let line = &self.linebuf[0][k];
            self.bbox
                .update(line.x_min, line.y_min, line.x_max, line.y_max);
        }

        if !extra {
            self.n_first = self.linebuf[0].len();
        }
    }

    fn add_line(&mut self, pt0: Vector, pt1: Vector) -> bool {
        let Vector { x, y } = pt1 - pt0;
        if x == 0 && y == 0 {
            return true;
        }

        let mut line = PolylineSegment::default();

        line.flags =
            SegFlag::ExactLeft | SegFlag::ExactRight | SegFlag::ExactTop | SegFlag::ExactBottom;

        if x < 0 {
            line.flags ^= SegFlag::UlDr;
        }
        if y >= 0 {
            line.flags ^= SegFlag::Dn | SegFlag::UlDr;
        }

        line.x_min = pt0.x.min(pt0.x);
        line.x_max = pt0.x.max(pt1.x);
        line.y_min = pt0.y.min(pt0.y);
        line.y_max = pt0.y.max(pt1.y);

        line.a = y;
        line.b = -x;
        line.c = y as i64 * pt0.x as i64 - x as i64 * pt0.y as i64;

        // halfplane normalization
        let mut max_ab: u32 = abs(x).max(abs(y));
        let shift: u32 = max_ab.leading_zeros() - 1;
        max_ab <<= shift + 1;
        line.a *= 1 << shift;
        line.b *= 1 << shift;
        line.c *= 1 << shift;
        line.scale = compute_scale(max_ab) as i32;

        self.linebuf[0].push(line);

        true
    }

    fn add_quadratic(&mut self, pts: [Vector; 3]) -> bool {
        let [p0, p1, p2] = pts;
        let seg = OutlineSegment::new(p0, p2, self.outline_error);
        if !seg.subdivide(p0, p1) {
            return self.add_line(p0, p2);
        }

        let mut next = [Vector::default(); 5];
        next[1] = p0 + p1;
        next[3] = p1 + p2;

        next[2] = (next[1] + next[3] + 2) >> 2;

        next[1] >>= 1;
        next[3] >>= 1;

        next[0] = p0;
        next[4] = p2;

        let a = next[..3].try_into().unwrap();
        let b = next[3..].try_into().unwrap();
        self.add_quadratic(a) && self.add_quadratic(b)
    }

    fn add_cubic(&mut self, pts: [Vector; 4]) -> bool {
        let [p0, p1, p2, p3] = pts;
        let seg = OutlineSegment::new(p0, p3, self.outline_error);
        if !seg.subdivide(p0, p1) && !seg.subdivide(p0, p2) {
            return self.add_line(p0, p3);
        }

        let mut next = [Vector::default(); 7];

        next[1] = p0 + p1;
        let center = p1 + p2 + 2;
        next[5] = p2 + p3;
        next[2] = next[1] + center;
        next[4] = center + next[5];
        next[3] = (next[2] + next[4] - 1) >> 3;
        next[2] >>= 2;
        next[4] >>= 2;
        next[1] >>= 1;
        next[5] >>= 1;
        next[0] = p0;
        next[1] = p3;

        let a = next[..4].try_into().unwrap();
        let b = next[4..].try_into().unwrap();
        self.add_cubic(a) && self.add_cubic(b)
    }

    pub fn fill(
        &mut self,
        engine: &impl BitmapEngine,
        buf: &mut [u8],
        x0: i32,
        y0: i32,
        width: i32,
        height: i32,
        stride: isize,
    ) {
        assert!(width > 0 && height > 0);
        assert_ne!(0, width & ((1 << engine.tile_order()) - 1));
        assert_ne!(0, height & ((1 << engine.tile_order()) - 1));
        let (x0, y0) = (x0 * 1 << 6, y0 * 1 << 6);

        for line in &mut self.linebuf[0] {
            line.x_min -= x0;
            line.x_max -= x0;
            line.y_min -= y0;
            line.y_max -= y0;
            line.c -= i64_mul(line.a, x0) + i64_mul(line.b, y0);
        }
        self.bbox.x_min -= x0;
        self.bbox.x_max -= x0;
        self.bbox.y_min -= y0;
        self.bbox.y_max -= y0;

        self.linebuf[1].resize(self.linebuf[0].len(), PolylineSegment::default());

        let mut n_lines = [self.n_first, self.linebuf[0].len() - self.n_first];

        let size_x = width << 6;
        let size_y = height << 6;
        if self.bbox.x_max >= size_x {
            let ([buf, _], [new_n, _], _) =
                polyline_split_horz(&self.linebuf[0], n_lines, [0, 0], size_x);
            self.linebuf[0] = buf;
            n_lines = new_n;
        }
        if self.bbox.y_max >= size_y {
            let ([buf, _], [new_n, _], _) =
                polyline_split_vert(&self.linebuf[0], n_lines, [0, 0], size_y);
            self.linebuf[0] = buf;
            n_lines = new_n;
        }

        let mut winding = [0, 0];
        if self.bbox.x_min <= 0 {
            let ([_, buf], [_, new_n], new_winding) =
                polyline_split_horz(&self.linebuf[0], n_lines, winding, 0);
            self.linebuf[0] = buf;
            winding = new_winding;
            n_lines = new_n;
        }
        if self.bbox.y_min <= 0 {
            let ([_, buf], [_, new_n], new_winding) =
                polyline_split_vert(&self.linebuf[0], n_lines, winding, 0);
            self.linebuf[0] = buf;
            winding = new_winding;
            n_lines = new_n;
        }

        assert_eq!(self.linebuf[0].len(), n_lines[0] + n_lines[1]); // ???
        self.linebuf[1].clear(); // okay, I now understand how the linebuf works

        self.fill_level(engine, buf, width, height, stride, 0, n_lines, winding);
    }

    fn fill_solid(
        engine: &impl BitmapEngine,
        mut buf: &mut [u8],
        width: i32,
        height: i32,
        stride: isize,
        set: i32,
    ) {
        assert_ne!(0, width & ((1 << engine.tile_order()) - 1));
        assert_ne!(0, height & ((1 << engine.tile_order()) - 1));

        let step: isize = 1 << engine.tile_order();
        let tile_stride: isize = stride * step;
        let width = width >> engine.tile_order();
        let height = height >> engine.tile_order();
        for _y in 0..height {
            for x in 0..width {
                let i = x as usize * step as usize;
                engine.fill_solid(&mut buf[i..], stride, set);
            }
            buf = &mut buf[tile_stride as usize..];
        }
    }

    fn fill_halfplane(
        engine: &impl BitmapEngine,
        mut buf: &mut [u8],
        width: i32,
        height: i32,
        stride: isize,
        a: i32,
        b: i32,
        c: i64,
        scale: i32,
    ) {
        assert_ne!(0, width & ((1 << engine.tile_order()) - 1));
        assert_ne!(0, height & ((1 << engine.tile_order()) - 1));

        if width == 1 << engine.tile_order() && height == 1 << engine.tile_order() {
            engine.fill_halfplane(buf, stride, a, b, c, scale);
            return;
        }

        let size: i64 = i64::from(abs(a) + abs(b)) << (engine.tile_order() + 5);
        let offs: i64 = (a as i64 + b as i64) * (1 << (engine.tile_order() + 5));

        let step: isize = 1 << engine.tile_order();
        let tile_stride: isize = stride * (1 << engine.tile_order());
        let width = width >> engine.tile_order();
        let height = height >> engine.tile_order();

        for y in 0..height {
            for x in 0..width {
                let cc: i64 =
                    c - (i64_mul(a, x) + i64_mul(b, y)) * (1 << (engine.tile_order() + 6));
                let offs_c: i64 = offs - cc;
                let abs_c = offs_c.abs();

                let i = x as usize * step as usize;
                if abs_c < size {
                    engine.fill_halfplane(&mut buf[i..], stride, a, b, cc, scale);
                } else {
                    let set = ((offs_c >> 32) as i32 ^ scale) as u32 & 0x8000_0000;
                    engine.fill_solid(&mut buf[i..], stride, set as i32);
                }
            }
            buf = &mut buf[tile_stride as usize..];
        }
    }

    fn fill_level(
        &mut self,
        engine: &impl BitmapEngine,
        buf: &mut [u8],
        width: i32,
        height: i32,
        stride: isize,
        index: usize,
        n_lines: [usize; 2],
        winding: [i32; 2],
    ) {
        let total_lines = n_lines[0] + n_lines[1];

        assert!(width > 0 && height > 0);
        assert!(index < 2 && total_lines <= self.linebuf[index].len());
        assert_ne!(0, width & ((1 << engine.tile_order()) - 1));
        assert_ne!(0, height & ((1 << engine.tile_order()) - 1));

        let (linebuf, other_linebuf) = {
            let (a, b) = self.linebuf.split_at_mut(1);
            if index == 0 {
                (&mut a[0], &mut b[0])
            } else {
                (&mut b[0], &mut a[0])
            }
        };

        let offs: usize = linebuf.len() - total_lines;
        let (line, line1) = linebuf[offs..].split_at_mut(n_lines[0]);
        assert_eq!([line.len(), line1.len()], n_lines);

        macro_rules! done {
            () => {
                linebuf.truncate(offs);
                assert_eq!(linebuf.len(), offs);
                return;
            };
        }

        macro_rules! line_fields {
            ($line:expr, $flags:expr) => {{
                let PolylineSegment {
                    a, b, c, mut scale, ..
                } = $line;
                if $flags.contains(FillFlag::Reverse) {
                    scale = -scale;
                }
                (a, b, c, scale)
            }};
        }

        let flags0 = get_fill_flags(line, winding[0]);
        let flags1 = get_fill_flags(line1, winding[1]);
        let flags = (flags0 | flags1) ^ FillFlag::Complex;
        if flags.intersects(FillFlag::Solid | FillFlag::Complex) {
            Self::fill_solid(
                engine,
                buf,
                width,
                height,
                stride,
                (flags & FillFlag::Solid).bits() as _,
            );
            done!();
        }
        if !flags.contains(FillFlag::Generic) && (flags0 ^ flags1).contains(FillFlag::Complex) {
            let l = if flags1.contains(FillFlag::Complex) {
                line1[0]
            } else {
                line[0]
            };
            let (a, b, c, scale) = line_fields!(l, flags);
            Self::fill_halfplane(engine, buf, width, height, stride, a, b, c, scale);
            done!();
        }
        if width == 1 << engine.tile_order() && height == 1 << engine.tile_order() {
            if !flags1.contains(FillFlag::Complex) {
                // we checked earlier that line's bounds are correct
                engine.fill_generic(buf, stride, &line[..], winding[0]);
                done!();
            }
            if !flags0.contains(FillFlag::Complex) {
                engine.fill_generic(buf, stride, &line1[..], winding[1]);
                done!();
            }

            if flags0.contains(FillFlag::Generic) {
                engine.fill_generic(buf, stride, &line[..], winding[0]);
            } else {
                let (a, b, c, scale) = line_fields!(line[0], flags0);
                engine.fill_halfplane(buf, stride, a, b, c, scale);
            }

            let tile = &mut self.tile[..];

            if flags1.contains(FillFlag::Generic) {
                engine.fill_generic(tile, width as _, &line1[..], winding[1]);
            } else {
                let (a, b, c, scale) = line_fields!(line1[0], flags1);
                engine.fill_halfplane(tile, width as _, a, b, c, scale);
            }

            engine.add_bitmaps(buf, stride, tile, width as _, height as _, width as _);
            done!();
        }

        let offs1 = other_linebuf.len();
        //let dst0 = linebuf;
        //let dst1 = &mut other_linebuf[offs1..];

        let (mut width, mut width1) = (width, width);
        let (mut height, mut height1) = (height, height);

        // TODO: replace with more understandable code
        #[inline]
        fn ilog2(n: i32) -> i32 {
            n.leading_zeros() as i32 ^ 31
        }

        let split_idx: usize;

        let ([lines_a, lines_b], n_next, winding1) = if width > height {
            width = 1 << ilog2(width - 1);
            width1 -= width;
            split_idx = width as usize;
            polyline_split_horz(&line[..], n_lines, winding, width << 6)
        } else {
            height = 1 << ilog2(height - 1);
            height1 -= height;
            split_idx = height as usize * stride as usize;
            polyline_split_vert(&line[..], n_lines, winding, height << 6)
        };
        linebuf.extend(lines_a);
        other_linebuf.extend(lines_b);

        let (buf, buf1) = buf.split_at_mut(split_idx);

        self.fill_level(
            engine, buf, width, height, stride, index, n_next[0], winding,
        );
        assert_eq!(self.linebuf[index].len(), offs);

        self.fill_level(
            engine,
            buf1,
            width1,
            height1,
            stride,
            index ^ 1,
            n_next[1],
            winding1,
        );
        assert_eq!(self.linebuf[index ^ 1].len(), offs1);
    }
}
