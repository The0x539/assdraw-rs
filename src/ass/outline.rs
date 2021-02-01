#[derive(Debug, Default, Copy, Clone)]
pub struct Vector {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct DVector {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
    pub x_min: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub y_max: i32,
}

impl Rect {
    #[inline]
    pub fn reset(&mut self) {
        self.x_min = i32::MAX;
        self.y_min = i32::MAX;
        self.x_max = i32::MIN;
        self.y_max = i32::MIN;
    }

    #[inline]
    pub fn update(&mut self, x_min: i32, y_min: i32, x_max: i32, y_max: i32) {
        self.x_min = self.x_min.min(x_min);
        self.y_min = self.y_min.min(y_min);
        self.x_max = self.x_max.max(x_max);
        self.y_max = self.y_max.max(y_max);
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct DRect {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Segment {
    LineSegment = 1,
    QuadSpline = 2,
    CubicSpline = 3,

    ContourEnd = 4,

    LineSegmentEnd = Self::LineSegment as u8 | 4,
    QuadSplineEnd = Self::QuadSpline as u8 | 4,
    CubicSplineEnd = Self::CubicSpline as u8 | 4,
}

impl Segment {
    #[inline]
    pub const fn outline_count(self) -> u8 {
        // a line segment has 1 point, a quad spline has 2, a cubic spline has 3
        // (and a contour end has 0)
        (self as u8) & 3
    }

    #[inline]
    pub const fn is_end(self) -> bool {
        (self as u8) & 4 != 0
    }

    #[inline]
    pub const fn as_end(self) -> Self {
        match self {
            Self::LineSegment => Self::LineSegmentEnd,
            Self::QuadSpline => Self::QuadSplineEnd,
            Self::CubicSpline => Self::CubicSplineEnd,
            x => x,
        }
    }
}

pub struct Outline {
    points: Vec<Vector>,
    segments: Vec<Segment>,
}

impl Outline {
    pub const MAX_COORD: i32 = (1i32 << 28) - 1;

    pub fn new(n_points: usize, n_segments: usize) -> Self {
        Self {
            points: Vec::with_capacity(n_points),
            segments: Vec::with_capacity(n_segments),
        }
    }

    pub fn add_point(&mut self, pt: Vector, segment: Option<Segment>) -> Result<(), ()> {
        if pt.x.saturating_abs() > Self::MAX_COORD || pt.y.saturating_abs() > Self::MAX_COORD {
            return Err(());
        }
        self.points.push(pt);
        if let Some(segment) = segment {
            self.segments.push(segment);
        }
        Ok(())
    }

    pub fn add_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn close_contour(&mut self) {
        let seg = self
            .segments
            .last_mut()
            .expect("tried to close contour of empty outline");
        assert!(!seg.is_end(), "tried to close already-closed contour",);
        *seg = seg.as_end();
    }

    pub fn update_cbox(&self, cbox: &mut Rect) {
        for point in &self.points {
            cbox.update(point.x, point.y, point.x, point.y);
        }
    }
}
