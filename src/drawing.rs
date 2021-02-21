use either::Either;
use itertools::Itertools;

use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone)]
pub enum Command<P> {
    Move(P),
    Line(P),
    Bezier(P, P, P),
}

impl<P> Command<P> {
    #[inline]
    pub const fn kind(&self) -> CommandKind {
        match self {
            Self::Move(..) => CommandKind::Move,
            Self::Line(..) => CommandKind::Line,
            Self::Bezier(..) => CommandKind::Bezier,
        }
    }

    pub fn points(self) -> impl Iterator<Item = P> {
        use std::iter::once;
        let thrice = |a, b, c| once(a).chain(once(b)).chain(once(c));
        match self {
            Self::Move(p1) | Self::Line(p1) => Either::Left(once(p1)),
            Self::Bezier(p1, p2, p3) => Either::Right(thrice(p1, p2, p3)),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Move,
    Line,
    Bezier,
}

#[derive(Debug, Copy, Clone)]
pub enum Segment<P> {
    Line(P, P),
    ClosingLine(P, P),
    Bezier(P, P, P, P),
}

impl<P> Segment<P> {
    pub fn points(self) -> impl Iterator<Item = P> {
        use std::iter::once;
        let twice = |a, b| once(a).chain(once(b));
        let fourice = |a, b, c, d| twice(a, b).chain(twice(c, d));
        match self {
            Self::Line(p0, p1) | Self::ClosingLine(p0, p1) => Either::Left(twice(p0, p1)),
            Self::Bezier(p0, p1, p2, p3) => Either::Right(fourice(p0, p1, p2, p3)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Drawing<P> {
    segments: Vec<CommandKind>,
    points: Vec<P>,
}

impl<P> Drawing<P> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn push(&mut self, command: Command<P>) {
        self.segments.push(command.kind());
        self.points.extend(command.points());
    }

    pub fn clear(&mut self) {
        self.segments.clear();
        self.points.clear();
    }

    pub fn segments<'a>(&'a self) -> impl Iterator<Item = Segment<P>> + 'a
    where
        P: Default + Clone,
    {
        SegmentsIter {
            segments: self.segments.iter().cloned(),
            points: self.points.iter().cloned(),
            pen: P::default(),
            shape_start: None,
        }
    }

    pub fn commands<'a>(&'a self) -> impl Iterator<Item = Command<P>> + 'a
    where
        P: Clone,
    {
        CommandsIter {
            segments: self.segments.iter().cloned(),
            points: self.points.iter().cloned(),
        }
    }

    #[inline]
    pub fn points(&self) -> &[P] {
        &self.points[..]
    }

    #[inline]
    pub fn points_mut(&mut self) -> &mut [P] {
        &mut self.points[..]
    }
}

impl<P> Default for Drawing<P> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<P, I> Index<I> for Drawing<P>
where
    [P]: Index<I>,
{
    type Output = <[P] as Index<I>>::Output;
    fn index(&self, index: I) -> &Self::Output {
        &self.points[..][index]
    }
}

impl<P, I> IndexMut<I> for Drawing<P>
where
    [P]: IndexMut<I>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.points[..][index]
    }
}

pub struct CommandsIter<Si, Pi> {
    segments: Si,
    points: Pi,
}

impl<Si, Pi, P> Iterator for CommandsIter<Si, Pi>
where
    Si: Iterator<Item = CommandKind>,
    Pi: Iterator<Item = P>,
{
    type Item = Command<P>;
    fn next(&mut self) -> Option<Self::Item> {
        let seg_ty = self.segments.next()?;
        let cmd = match seg_ty {
            CommandKind::Move => {
                let p = self.points.next()?;
                Command::Move(p)
            }
            CommandKind::Line => {
                let p = self.points.next()?;
                Command::Line(p)
            }
            CommandKind::Bezier => {
                let (p1, p2, p3) = self.points.next_tuple()?;
                Command::Bezier(p1, p2, p3)
            }
        };
        Some(cmd)
    }
}

pub struct SegmentsIter<Si, Pi, P> {
    segments: Si,
    points: Pi,
    pen: P,
    shape_start: Option<P>,
}

impl<Si, Pi, P> Iterator for SegmentsIter<Si, Pi, P>
where
    Si: Iterator<Item = CommandKind>,
    Pi: Iterator<Item = P>,
    P: Clone,
{
    type Item = Segment<P>;
    fn next(&mut self) -> Option<Self::Item> {
        let seg_ty = match self.segments.next() {
            Some(ty) => ty,
            None => {
                // If there's an open shape, close it.
                return self
                    .shape_start
                    .take()
                    .map(|start| Segment::ClosingLine(self.pen.clone(), start));
            }
        };

        match seg_ty {
            CommandKind::Move => {
                let next_pen_pos = self.points.next()?;
                let prev_pen_pos = std::mem::replace(&mut self.pen, next_pen_pos);

                if let Some(start) = self.shape_start.take() {
                    // If there was an open shape, close it.
                    Some(Segment::ClosingLine(prev_pen_pos, start))
                } else {
                    // Otherwise, don't conclude iteration; try to get the next segment.
                    self.next()
                }
            }
            CommandKind::Line => {
                let next_pen_pos = self.points.next()?;
                let prev_pen_pos = std::mem::replace(&mut self.pen, next_pen_pos);

                if self.shape_start.is_none() {
                    self.shape_start = Some(prev_pen_pos.clone());
                }
                Some(Segment::Line(prev_pen_pos, self.pen.clone()))
            }
            CommandKind::Bezier => {
                let (p1, p2, p3) = self.points.next_tuple()?;
                let prev_pen_pos = std::mem::replace(&mut self.pen, p3);

                if self.shape_start.is_none() {
                    self.shape_start = Some(prev_pen_pos.clone());
                }
                Some(Segment::Bezier(prev_pen_pos, p1, p2, self.pen.clone()))
            }
        }
    }
}
