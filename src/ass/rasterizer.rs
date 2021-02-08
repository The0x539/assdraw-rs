use std::convert::TryInto;

use aligned_box::AlignedBox;
use enumflags2::BitFlags;

use super::bitmap::BitmapEngine;
use super::outline::{Outline, Rect, Segment, Vector};

#[derive(BitFlags, Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
enum SegFlag {
    Dn = 1,
    UlDr = 2,
    ExactLeft = 4,
    ExactRight = 8,
    ExactTop = 16,
    ExactBottom = 32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PolylineSegment {
    c: i64,
    a: i32,
    b: i32,
    scale: i32,
    flags: BitFlags<SegFlag>,
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
}

impl PolylineSegment {
    fn move_x(&mut self, x: i32) {
        self.x_min -= x;
        self.x_max -= x;
        self.x_min = self.x_min.max(0);
        self.c = i64_mul(self.a, x);

        // should be const
        let test: BitFlags<SegFlag> = SegFlag::ExactLeft | SegFlag::UlDr;
        if self.y_min == 0 && self.flags == test {
            self.flags.remove(SegFlag::ExactTop);
        }
    }

    fn split_horz(&self, x: i32) -> (Self, Self) {
        assert!(x > self.x_min && x < self.x_max);

        let (mut line, mut next) = (*self, *self);
        next.c = i64_mul(line.a, x);
        next.x_min = 0;
        next.x_max -= x;
        line.x_max = x;

        line.flags.remove(SegFlag::ExactTop);
        next.flags.remove(SegFlag::ExactBottom);
        if line.flags.contains(SegFlag::UlDr) {
            std::mem::swap(&mut line.flags, &mut next.flags);
        }
        line.flags.insert(SegFlag::ExactRight);
        next.flags.insert(SegFlag::ExactLeft);

        (line, next)
    }

    fn check_left(&self, x: i32) -> bool {
        if self.flags.contains(SegFlag::ExactLeft) {
            return self.x_min >= x;
        }
        let y = if self.flags.contains(SegFlag::UlDr) {
            self.y_min
        } else {
            self.y_max
        };
        let mut cc = self.c - i64_mul(self.a, x) - i64_mul(self.b, y);
        if self.a > 0 {
            cc = -cc;
        }
        cc >= 0
    }

    fn check_right(&self, x: i32) -> bool {
        if self.flags.contains(SegFlag::ExactRight) {
            return self.x_max <= x;
        }
        let y = if self.flags.contains(SegFlag::UlDr) {
            self.y_max
        } else {
            self.y_min
        };
        let mut cc = self.c - i64_mul(self.a, x) - i64_mul(self.b, y);
        if self.a > 0 {
            cc = -cc;
        }
        cc >= 0
    }
}

#[allow(dead_code)]
fn polyline_split_horz(
    src: &[PolylineSegment],
    n_src: [usize; 2],
    x: i32,
) -> ([Vec<PolylineSegment>; 2], [[usize; 2]; 2], [i32; 2]) {
    let mut dst = [Vec::new(), Vec::new()];
    let mut n_dst = [[0; 2]; 2];
    let mut winding = [0; 2];

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

#[derive(Debug, Copy, Clone)]
struct OutlineSegment {
    r: Vector,
    r2: i64,
    er: i64,
}

#[inline]
fn i64_mul(a: i32, b: i32) -> i64 {
    a as i64 * b as i64
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

    #[allow(dead_code)]
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

        #[inline]
        fn abs(n: i32) -> u32 {
            match n {
                -2147483648 => 2147483648,
                -2147483647..=-1 => -n as u32,
                0..=2147483647 => n as u32,
            }
        }
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

    #[allow(unused)]
    pub fn fill<Engine: BitmapEngine>(
        &mut self,
        buf: &mut [u8],
        x0: i32,
        y0: i32,
        width: i32,
        height: i32,
        stride: isize,
    ) -> bool {
        assert!(width > 0 && height > 0);
        assert_ne!(0, width & ((1 << Engine::TILE_ORDER) - 1));
        assert_ne!(0, height & ((1 << Engine::TILE_ORDER) - 1));
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

        let n_unused = [0; 2];
        let n_lines = [self.n_first, self.linebuf[0].len() - self.n_first];
        let winding = [0; 0];

        let size_x = width << 6;
        let size_y = height << 6;
        if self.bbox.x_max >= size_x {
            todo!()
        }

        todo!()
    }
}
