use aligned_box::AlignedBox;

use super::outline::{Outline, Rect, Segment, Vector};

#[derive(Debug, Default, Copy, Clone)]
pub struct PolylineSegment {
    c: i64,
    a: i32,
    b: i32,
    scale: i32,
    flags: i32,
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
}

pub struct RasterizerData {
    outline_error: i32,

    bbox: Rect,

    linebuf: [Vec<PolylineSegment>; 2],
    n_first: usize,

    tile: AlignedBox<[u8]>,
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
            }
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

    fn add_line(&mut self, _pt0: Vector, _pt1: Vector) {
        todo!()
    }

    fn add_quadratic(&mut self, pts: [Vector; 3]) {
        let [_pt0, _pt1, _pt2] = pts;
    }

    fn add_cubic(&mut self, pts: [Vector; 4]) {
        let [_pt0, _pt1, _pt2, _pt3] = pts;
    }
}
