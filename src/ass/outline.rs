use std::ops::{Add, AddAssign, Div, Shr, ShrAssign, Sub, SubAssign};

use itertools::Itertools;

#[derive(Debug, Default, Copy, Clone)]
pub struct Vector {
    pub x: i32,
    pub y: i32,
}

impl Vector {
    #[inline]
    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    #[inline]
    pub const fn checked_abs(self) -> Option<Self> {
        match (self.x.checked_abs(), self.y.checked_abs()) {
            (Some(x), Some(y)) => Some(Self { x, y }),
            _ => None,
        }
    }
}

impl Add for Vector {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<i32> for Vector {
    type Output = Self;
    #[inline]
    fn add(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl Sub for Vector {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<i32> for Vector {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl AddAssign<i32> for Vector {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.x += rhs;
        self.y += rhs;
    }
}

impl AddAssign<Self> for Vector {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl SubAssign<Self> for Vector {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Shr<i32> for Vector {
    type Output = Self;
    #[inline]
    fn shr(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x >> rhs,
            y: self.y >> rhs,
        }
    }
}

impl ShrAssign<i32> for Vector {
    #[inline]
    fn shr_assign(&mut self, rhs: i32) {
        *self = *self >> rhs;
    }
}

impl Div<i32> for Vector {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SegmentType {
    LineSegment,
    QuadSpline,
    CubicSpline,
}

#[derive(Debug, Default, Clone)]
pub struct Outline {
    points: Vec<Vector>,
    segments: Vec<SegmentType>,
    start_of_final_contour: Option<usize>,
}

impl Outline {
    pub const MAX_COORD: i32 = (1i32 << 28) - 1;

    pub fn points(&self) -> &[Vector] {
        &self.points[..]
    }

    pub fn segments(&self) -> Segments {
        Segments {
            points: self.points.iter(),
            segments: self.segments.iter(),
        }
    }

    pub fn new(n_points: usize, n_segments: usize) -> Self {
        Self {
            points: Vec::with_capacity(n_points),
            segments: Vec::with_capacity(n_segments),
            start_of_final_contour: None,
        }
    }

    pub fn add_point(&mut self, pt: Vector, segment: Option<SegmentType>) -> Result<(), ()> {
        if pt.x.saturating_abs() > Self::MAX_COORD || pt.y.saturating_abs() > Self::MAX_COORD {
            return Err(());
        }
        self.points.push(pt);
        if let Some(segment) = segment {
            self.segments.push(segment);
        }
        if self.start_of_final_contour.is_none() {
            self.start_of_final_contour = Some(self.points.len() - 1);
        }
        Ok(())
    }

    pub fn add_segment(&mut self, segment: SegmentType) {
        self.segments.push(segment);
    }

    pub fn close_contour(&mut self) {
        let p0 = match self.points.last() {
            Some(p) => *p,
            None => return,
        };

        let p1 = match self.start_of_final_contour.take() {
            Some(i) => self.points[i],
            None => return,
        };

        self.add_point(p0, Some(SegmentType::LineSegment)).unwrap();
        self.add_point(p1, None).unwrap();
    }

    pub fn update_cbox(&self, cbox: &mut Rect) {
        for point in &self.points {
            cbox.update(point.x, point.y, point.x, point.y);
        }
    }
}

#[derive(Debug)]
pub enum Segment {
    LineSegment(Vector, Vector),
    QuadSpline(Vector, Vector, Vector),
    CubicSpline(Vector, Vector, Vector, Vector),
}

pub struct Segments<'a> {
    points: std::slice::Iter<'a, Vector>,
    segments: std::slice::Iter<'a, SegmentType>,
}

impl std::iter::Iterator for Segments<'_> {
    type Item = Segment;
    fn next(&mut self) -> Option<Self::Item> {
        let s_ty = self.segments.next()?;
        let seg = match s_ty {
            SegmentType::LineSegment => {
                let (p0, p1) = self.points.next_tuple()?;
                Segment::LineSegment(*p0, *p1)
            }
            SegmentType::QuadSpline => {
                let (p0, p1, p2) = self.points.next_tuple()?;
                Segment::QuadSpline(*p0, *p1, *p2)
            }
            SegmentType::CubicSpline => {
                let (p0, p1, p2, p3) = self.points.next_tuple()?;
                Segment::CubicSpline(*p0, *p1, *p2, *p3)
            }
        };
        Some(seg)
    }
}
