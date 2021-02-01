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
