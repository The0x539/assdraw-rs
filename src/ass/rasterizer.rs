use aligned_box::AlignedBox;
use enumflags2::BitFlags;

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

        // wtb array slicing
        // let (a, b) = next.split_at(3);
        let (a, b) = ([next[0], next[1], next[2]], [next[2], next[3], next[4]]);
        self.add_quadratic(a) && self.add_quadratic(b)
    }

    fn add_cubic(&mut self, pts: [Vector; 4]) -> bool {
        let [p0, p1, p2, p3] = pts;
        let seg = OutlineSegment::new(p0, p3, self.outline_error);
        if !seg.subdivide(p0, p1) && !seg.subdivide(p0, p2) {
            return self.add_line(p0, p3);
        }

        let mut next = [Vector::default(); 7];
        let mut center = Vector::default();

        next[1] = p0 + p1;

        todo!()
    }
}
