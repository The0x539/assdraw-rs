use crate::ass::outline::Rect;
use ab_glyph_rasterizer::Point;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenType {
    Move,
    MoveNc,
    Line,
    CubicBezier,
    ConicBezier,
    BSpline,
    ExtendBSpline,
    Close,
}

#[derive(Debug, Copy, Clone)]
pub struct DrawingToken {
    token_type: TokenType,
    point: Point,
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

fn double_to_d6(_val: f64) -> f32 {
    todo!()
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
    let mut point = Point::default();

    let mut tokens = Vec::<DrawingToken>::new();
    let mut spline_start = None::<usize>;

    while p != [] {
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
                b'q' => token_type = Some(TokenType::ConicBezier),
                b's' => token_type = Some(TokenType::BSpline),
                // TokenType::ExtendBSpline is ignored for reasons briefly documented in libass
                _ => (),
            }
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

pub struct Segment;

#[allow(dead_code)]
pub fn parse_drawing(text: &str) -> (Vec<Segment>, Rect) {
    let _tokens = tokenize_drawing(text);
    (vec![Segment], Rect::default())
}
