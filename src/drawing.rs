use itertools::Itertools;

use crate::ass::outline::{Outline, Rect, SegmentType, Vector};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum TokenType {
    Move,
    MoveNc,
    Line,
    CubicBezier,
    // ConicBezier,
    BSpline,
    // ExtendBSpline,
    // Close,
}

#[derive(Debug, Copy, Clone)]
struct DrawingToken {
    token_type: TokenType,
    point: Vector,
}

fn strtod(p: &mut &[u8], val: &mut f64) -> bool {
    let mut i = 0;
    let mut seen_dot = false;
    loop {
        let c = match p.get(i) {
            Some(c) => c,
            None => break,
        };
        match c {
            b'0'..=b'9' => i += 1,
            b'-' if i == 0 => i += 1,
            b'.' if !seen_dot => {
                i += 1;
                seen_dot = true;
            }
            _ => break,
        }
    }
    let (left, right) = p.split_at(i);
    if let Some(n) = std::str::from_utf8(left)
        .ok()
        .map(|s| s.parse().ok())
        .flatten()
    {
        *val = n;
        *p = right;
        true
    } else {
        false
    }
}

#[inline]
fn double_to_d6(val: f64) -> i32 {
    (val * 64.0) as i32
}

fn add_curve(
    outline: &mut Outline,
    cbox: &mut Rect,
    mut p: [Vector; 4],
    spline: bool,
    started: bool,
) {
    for i in 0..4 {
        cbox.update(p[i].x, p[i].y, p[i].x, p[i].y);
    }

    if spline {
        let p01 = (p[1] - p[0]) / 3;
        let p12 = (p[2] - p[1]) / 3;
        let p23 = (p[3] - p[2]) / 3;

        p[0] = p[1] + ((p12 - p01) >> 1);
        p[3] = p[2] + ((p23 - p12) >> 1);
        p[1] += p12;
        p[2] -= p12;
    }

    outline
        .add_point(p[0], Some(SegmentType::CubicSpline))
        .unwrap();
    outline.add_point(p[1], None).unwrap();
    outline.add_point(p[2], None).unwrap();
    outline.add_point(p[3], None).unwrap();
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum CoordStatus {
    None,
    GotX,
    GotXY,
}

fn tokenize_drawing(text: impl AsRef<[u8]>) -> Vec<DrawingToken> {
    let mut p = text.as_ref();
    let mut token_type = None::<TokenType>;
    let mut is_set = CoordStatus::None;
    let mut val = 0.0_f64;
    let mut point = Vector::default();

    let mut tokens = Vec::<DrawingToken>::new();
    let mut spline_start = None::<usize>;

    while p != [] {
        if p[0] == b' ' {
            p = &p[1..];
            continue;
        }

        let mut got_coord = false;
        if let (b'c', Some(start)) = (p[0], &mut spline_start) {
            let mut should_finish_spline = true;
            for i in 0..2 {
                if tokens[(*start + i) % tokens.len()].token_type != TokenType::BSpline {
                    should_finish_spline = false;
                    break;
                }
            }
            if should_finish_spline {
                for _ in 0..3 {
                    let new = DrawingToken {
                        token_type: TokenType::BSpline,
                        point: tokens[*start].point,
                    };
                    tokens.push(new);
                    *start += 1;
                }
            }
            p = &p[1..];
        } else if is_set == CoordStatus::None && strtod(&mut p, &mut val) {
            point.x = double_to_d6(val);
            is_set = CoordStatus::GotX;
            got_coord = true;
            // p--;
        } else if is_set == CoordStatus::GotX && strtod(&mut p, &mut val) {
            point.y = double_to_d6(val);
            is_set = CoordStatus::GotXY;
            got_coord = true;
            // p--;
        } else {
            match p[0] {
                b'm' => token_type = Some(TokenType::Move),
                b'n' => token_type = Some(TokenType::MoveNc),
                b'l' => token_type = Some(TokenType::Line),
                b'b' => token_type = Some(TokenType::CubicBezier),
                // b'q' => token_type = Some(TokenType::ConicBezier),
                b's' => token_type = Some(TokenType::BSpline),
                // TokenType::ExtendBSpline is ignored for reasons briefly documented in libass
                _ => (),
            }
            p = &p[1..];
        }

        // Ignore the odd extra value, it makes no sense.
        if !got_coord {
            is_set = CoordStatus::None;
        }

        if let (Some(token_type), CoordStatus::GotXY) = (token_type, is_set) {
            let new = DrawingToken { token_type, point };
            tokens.push(new);
            is_set = CoordStatus::None;
            if token_type == TokenType::BSpline && spline_start == None {
                spline_start = Some(tokens.len().saturating_sub(1))
            }
        }
    }

    tokens
}

pub fn parse_drawing(text: impl AsRef<[u8]>) -> (Outline, Rect) {
    let mut cbox = Rect::default();
    cbox.reset();
    let mut outline = Outline::default();

    let mut started = false;
    let mut pen = Vector::default();

    let mut tokens = tokenize_drawing(text).into_iter().multipeek();
    while let Some(token) = tokens.next() {
        match token.token_type {
            TokenType::MoveNc => {
                pen = token.point;
                cbox.update(pen.x, pen.y, pen.x, pen.y);
            }
            TokenType::Move => {
                pen = token.point;
                cbox.update(pen.x, pen.y, pen.x, pen.y);
                if started {
                    outline.add_segment(SegmentType::LineSegment);
                    outline.close_contour();
                    started = false;
                }
            }
            TokenType::Line => {
                let to = token.point;
                cbox.update(to.x, to.y, to.x, to.y);
                outline
                    .add_point(pen, Some(SegmentType::LineSegment))
                    .unwrap();
                outline.add_point(to, None).unwrap();
                pen = to;
                started = true;
            }
            TokenType::CubicBezier | TokenType::BSpline => {
                let ty = token.token_type;
                match (token, tokens.peek().copied(), tokens.peek().copied()) {
                    (t1, Some(t2), Some(t3)) if t2.token_type == ty && t3.token_type == ty => {
                        tokens.next();
                        tokens.next();
                        let points = [pen, t1.point, t2.point, t3.point];
                        let is_spline = ty == TokenType::BSpline;
                        add_curve(&mut outline, &mut cbox, points, is_spline, started);
                    }
                    _ => {
                        tokens.reset_peek();
                    }
                }
                // consider doing this inside the conditional
                pen = token.point;
                started = true;
            }
        }
    }

    if started {
        //outline.add_segment(SegmentType::LineSegment);
        outline.close_contour();
    }

    (outline, cbox)
}
