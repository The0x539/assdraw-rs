use std::ops::{Add, AddAssign, Div, Shr, ShrAssign, Sub, SubAssign};

#[derive(Debug, Default, Copy, Clone)]
pub struct Vector {
    pub x: i32,
    pub y: i32,
}

impl Vector {
    #[inline]
    #[allow(dead_code)]
    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    #[inline]
    #[allow(dead_code)]
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

#[derive(Debug)]
#[allow(dead_code)]
pub enum Segment {
    LineSegment(Vector, Vector),
    QuadSpline(Vector, Vector, Vector),
    CubicSpline(Vector, Vector, Vector, Vector),
}
