use std::mem::swap;

use enumflags2::BitFlags;

use super::utils::i64_mul;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SegmentFlag {
    Dn = 1,
    UlDr = 2,
    ExactLeft = 4,
    ExactRight = 8,
    ExactTop = 16,
    ExactBottom = 32,
}

pub type SegmentFlags = BitFlags<SegmentFlag>;

#[derive(Debug, Default, Copy, Clone)]
pub struct Segment {
    pub c: i64,
    pub a: i32,
    pub b: i32,
    pub scale: i32,
    pub flags: SegmentFlags,
    pub x_min: i32,
    pub x_max: i32,
    pub y_min: i32,
    pub y_max: i32,
}

impl Segment {
    pub fn move_x(&mut self, x: i32) {
        self.x_min -= x;
        self.x_max -= x;
        self.x_min = self.x_min.max(0);
        self.c -= i64_mul(self.a, x);

        let test = SegmentFlag::ExactLeft | SegmentFlag::UlDr;
        if self.x_min == 0 && self.flags.contains(test) {
            self.flags.remove(SegmentFlag::ExactTop);
        }
    }

    pub fn move_y(&mut self, y: i32) {
        self.y_min -= y;
        self.y_max -= y;
        self.y_min = self.y_min.max(0);
        self.c -= i64_mul(self.b, y);

        let test = SegmentFlag::ExactTop | SegmentFlag::UlDr;
        if self.y_min == 0 && self.flags.contains(test) {
            self.flags.remove(SegmentFlag::ExactLeft);
        }
    }

    pub fn split_horz(mut self, x: i32) -> (Self, Self) {
        assert!(x > self.x_min && x < self.x_max);

        let mut next = self;
        next.c -= i64_mul(self.a, x);
        next.x_min = 0;
        next.x_max -= x;
        self.x_max = x;

        self.flags.remove(SegmentFlag::ExactTop);
        next.flags.remove(SegmentFlag::ExactBottom);
        if self.flags.contains(SegmentFlag::UlDr) {
            swap(&mut self.flags, &mut next.flags);
        }
        self.flags.insert(SegmentFlag::ExactRight);
        next.flags.insert(SegmentFlag::ExactLeft);
        (self, next)
    }

    pub fn split_vert(mut self, y: i32) -> (Self, Self) {
        assert!(y > self.y_min && y < self.y_max);

        let mut next = self;
        next.c -= i64_mul(self.a, y);
        next.y_min = 0;
        next.y_max -= y;
        self.y_max = y;

        self.flags.remove(SegmentFlag::ExactLeft);
        next.flags.remove(SegmentFlag::ExactRight);
        if self.flags.contains(SegmentFlag::UlDr) {
            swap(&mut self.flags, &mut next.flags);
        }
        self.flags.insert(SegmentFlag::ExactBottom);
        next.flags.insert(SegmentFlag::ExactTop);
        (self, next)
    }

    pub fn check_left(&self, x: i32) -> bool {
        if self.flags.contains(SegmentFlag::ExactLeft) {
            return self.x_min >= x;
        }
        let y = self.cond_uldr(self.y_min, self.y_max);
        let mut cc = self.c - i64_mul(self.a, x) - i64_mul(self.b, y);
        if self.a < 0 {
            cc = -cc;
        }
        cc >= 0
    }

    pub fn check_right(&self, x: i32) -> bool {
        if self.flags.contains(SegmentFlag::ExactRight) {
            return self.x_max <= x;
        }
        let y = self.cond_uldr(self.y_max, self.y_min);
        let mut cc = self.c - i64_mul(self.a, x) - i64_mul(self.b, y);
        if self.a > 0 {
            cc = -cc;
        }
        cc >= 0
    }

    pub fn check_top(&self, y: i32) -> bool {
        if self.flags.contains(SegmentFlag::ExactTop) {
            return self.y_min >= y;
        }
        let x = self.cond_uldr(self.x_min, self.x_max);
        let mut cc = self.c - i64_mul(self.b, y) - i64_mul(self.a, x);
        if self.b < 0 {
            cc = -cc;
        }
        cc >= 0
    }

    pub fn check_bottom(&self, y: i32) -> bool {
        if self.flags.contains(SegmentFlag::ExactBottom) {
            return self.y_max <= y;
        }
        let x = self.cond_uldr(self.x_max, self.x_min);
        let mut cc = self.c - i64_mul(self.b, y) - i64_mul(self.a, x);
        if self.b > 0 {
            cc = -cc;
        }
        cc >= 0
    }

    fn cond_uldr<T>(&self, a: T, b: T) -> T {
        if self.flags.contains(SegmentFlag::UlDr) {
            a
        } else {
            b
        }
    }
}
