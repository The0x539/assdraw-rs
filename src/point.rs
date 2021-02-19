use std::convert::{TryFrom, TryInto};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Shl, ShlAssign, Shr,
    ShrAssign, Sub, SubAssign,
};

use num_traits::Signed;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn from<U>(point: Point<U>) -> Self
    where
        T: From<U>,
    {
        Point::new(T::from(point.x), T::from(point.y))
    }

    #[inline]
    pub fn into<U>(self) -> Point<U>
    where
        T: Into<U>,
    {
        Point::new(self.x.into(), self.y.into())
    }

    #[inline]
    pub fn try_from<U>(point: Point<U>) -> Result<Self, T::Error>
    where
        T: TryFrom<U>,
    {
        let (x, y) = (T::try_from(point.x)?, T::try_from(point.y)?);
        Ok(Point::new(x, y))
    }

    #[inline]
    pub fn try_into<U>(self) -> Result<Point<U>, T::Error>
    where
        T: TryInto<U>,
    {
        let (x, y) = (self.x.try_into()?, self.y.try_into()?);
        Ok(Point::new(x, y))
    }

    #[inline]
    pub fn abs(self) -> Self
    where
        T: Signed,
    {
        Point::new(self.x.abs(), self.y.abs())
    }
}

impl<T> From<(T, T)> for Point<T> {
    #[inline]
    fn from((x, y): (T, T)) -> Self {
        Self { x, y }
    }
}

impl<T> From<[T; 2]> for Point<T> {
    #[inline]
    fn from([x, y]: [T; 2]) -> Self {
        Self { x, y }
    }
}

impl<T> From<Point<T>> for (T, T) {
    #[inline]
    fn from(point: Point<T>) -> Self {
        (point.x, point.y)
    }
}

impl<T> From<Point<T>> for [T; 2] {
    #[inline]
    fn from(point: Point<T>) -> Self {
        [point.x, point.y]
    }
}

macro_rules! binary_op {
    ($trait:ident, $func:ident, $op:tt) => {
        impl<T: $trait> $trait for Point<T> {
            type Output = Point<T::Output>;
            #[inline]
            fn $func(self, rhs: Self) -> Self::Output {
                Point {
                    x: self.x $op rhs.x,
                    y: self.y $op rhs.y,
                }
            }
        }
        impl<T: $trait + Clone> $trait<T> for Point<T> {
            type Output = Point<T::Output>;
            #[inline]
            fn $func(self, rhs: T) -> Self::Output {
                Point {
                    x: self.x $op rhs.clone(),
                    y: self.y $op rhs,
                }
            }
        }
    }
}

binary_op!(Add, add, +);
binary_op!(Sub, sub, -);
binary_op!(Mul, mul, *);
binary_op!(Div, div, /);
binary_op!(Rem, rem, %);
binary_op!(Shl, shl, <<);
binary_op!(Shr, shr, >>);

macro_rules! compound_op {
    ($trait:ident, $func:ident, $op:tt) => {
        impl<T: $trait> $trait for Point<T> {
            #[inline]
            fn $func(&mut self, rhs: Self) {
                self.x $op rhs.x;
                self.y $op rhs.y;
            }
        }
        impl<T: $trait + Clone> $trait<T> for Point<T> {
            #[inline]
            fn $func(&mut self, rhs: T) {
                self.x $op rhs.clone();
                self.y $op rhs;
            }
        }
    };
}

compound_op!(AddAssign, add_assign, +=);
compound_op!(SubAssign, sub_assign, -=);
compound_op!(MulAssign, mul_assign, *=);
compound_op!(DivAssign, div_assign, /=);
compound_op!(RemAssign, rem_assign, %=);
compound_op!(ShlAssign, shl_assign, <<=);
compound_op!(ShrAssign, shr_assign, >>=);

impl<T: Neg> Neg for Point<T> {
    type Output = Point<T::Output>;
    #[inline]
    fn neg(self) -> Self::Output {
        Point {
            x: -self.x,
            y: -self.y,
        }
    }
}
