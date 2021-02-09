use aligned_box::AlignedBox;

use super::rasterizer::PolylineSegment;

// for distinction, since libass uses both int and int32_t around here
type Int = i32;

type ParamFilterFunc =
    fn(dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize, param: &[i16]);

pub trait BitmapEngine {
    fn align_order(&self) -> Int;

    fn tile_order(&self) -> Int;

    fn fill_solid(&self, buf: &mut [u8], stride: isize, set: Int);
    fn fill_halfplane(&self, buf: &mut [u8], stride: isize, a: i32, b: i32, c: i64, scale: i32);
    fn fill_generic(&self, buf: &mut [u8], stride: isize, line: &[PolylineSegment], winding: Int);

    fn add_bitmaps(
        &self,
        dst: &mut [u8],
        dst_stride: isize,
        src: &mut [u8],
        src_stride: isize,
        height: isize,
        width: isize,
    );
    fn sub_bitmaps(
        &self,
        dst: &mut [u8],
        dst_stride: isize,
        src: &mut [u8],
        src_stride: isize,
        height: isize,
        width: isize,
    );
    fn mul_bitmaps(
        &self,
        dst: &mut [u8],
        dst_stride: isize,
        src: &mut [u8],
        src_stride: isize,
        height: isize,
        width: isize,
    );

    fn be_blur(&self, buf: &mut [u8], w: isize, h: isize, stride: isize, tmp: &mut [u16]);

    fn stripe_unpack(
        &self,
        dst: &mut [i16],
        src: &[u8],
        src_stride: isize,
        width: usize,
        height: usize,
    );
    fn stripe_pack(
        &self,
        dst: &mut [i8],
        dst_stride: isize,
        src: &[i16],
        width: usize,
        height: usize,
    );
    fn shrink_horz(&self, dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize);
    fn shrink_vert(&self, dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize);
    fn expand_horz(&self, dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize);
    fn expand_vert(&self, dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize);

    fn blur_horz(&self) -> [ParamFilterFunc; 5];
    fn blur_vert(&self) -> [ParamFilterFunc; 5];
}

#[allow(dead_code)]
pub struct Bitmap<Engine> {
    left: i32,
    top: i32,
    w: i32,
    h: i32,
    stride: isize,
    buffer: AlignedBox<[u8]>,
    engine: Engine,
}

impl<E: BitmapEngine> Bitmap<E> {
    pub fn new(engine: E, w: i32, h: i32, zero: bool) -> Self {
        assert!(zero, "unitialized memory is annoying");

        let align = 1 << engine.align_order();
        let stride = super::utils::align(align, w as usize);
        assert!(stride <= ((i32::MAX - 32) / h.max(1)) as usize);

        let size = stride * (h as usize) + 32;
        let buffer = AlignedBox::slice_from_default(align, size).unwrap();

        Self {
            left: 0,
            top: 0,
            w,
            h,
            stride: stride as isize,
            buffer,
            engine,
        }
    }
}

impl<E: BitmapEngine + Clone> Clone for Bitmap<E> {
    fn clone(&self) -> Self {
        let mut new = Self::new(self.engine.clone(), self.w, self.h, true);
        new.buffer.copy_from_slice(&self.buffer[..]);
        new
    }
}
