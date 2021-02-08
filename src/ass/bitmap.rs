use std::marker::PhantomData;

use aligned_box::AlignedBox;

use super::rasterizer::PolylineSegment;

// for distinction, since libass uses both int and int32_t around here
type Int = i32;

type FillSolidTileFunc = fn(buf: &mut [u8], stride: isize, set: Int);
type FillHalfplaneTileFunc = fn(buf: &mut [u8], stride: isize, a: i32, b: i32, c: i64, scale: i32);
type FillGenericTileFunc =
    fn(buf: &mut [u8], stride: isize, line: &[PolylineSegment], winding: Int);

type BitmapBlendFunc = fn(
    dst: &mut [u8],
    dst_stride: isize,
    src: &mut [u8],
    src_stride: isize,
    height: isize,
    width: isize,
);
type BitmapMulFunc = fn(
    dst: &mut [u8],
    dst_stride: isize,
    src1: &mut [u8],
    src1_stride: isize,
    src2: &mut [u8],
    src2_stride: isize,
    width: isize,
    height: isize,
);

type BeBlurFunc = fn(buf: &mut [u8], w: isize, h: isize, stride: isize, tmp: &mut [u16]);

type Convert8to16Func =
    fn(dst: &mut [i16], src: &[u8], src_stride: isize, width: usize, height: usize);
type Convert16to8Func =
    fn(dst: &mut [u8], dst_stride: isize, src: &[i16], width: usize, height: usize);
type FilterFunc = fn(dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize);
type ParamFilterFunc =
    fn(dst: &mut [i16], src: &[i16], src_width: usize, src_height: usize, param: &[i16]);

#[allow(non_upper_case_globals)]
// TODO: be at least a little more idiomatic
pub trait BitmapEngine {
    const ALIGN_ORDER: Int;

    const TILE_ORDER: Int;
    const fill_solid: FillSolidTileFunc;
    const fill_halfplane: FillHalfplaneTileFunc;
    const fill_generic: FillGenericTileFunc;

    const add_bitmaps: BitmapBlendFunc;
    const sub_bitmaps: BitmapBlendFunc;
    const mul_bitmaps: BitmapMulFunc;

    const be_blur: BeBlurFunc;

    const stripe_unpack: Convert8to16Func;
    const stripe_pack: Convert16to8Func;
    const shrink_horz: FilterFunc;
    const shrink_vert: FilterFunc;
    const expand_horz: FilterFunc;
    const expand_vert: FilterFunc;
    const blur_horz: [ParamFilterFunc; 5];
    const blur_vert: [ParamFilterFunc; 5];
}

#[allow(dead_code)]
pub struct Bitmap<Engine> {
    left: i32,
    top: i32,
    w: i32,
    h: i32,
    stride: isize,
    buffer: AlignedBox<[u8]>,

    phantom: PhantomData<Engine>,
}

impl<Engine: BitmapEngine> Bitmap<Engine> {
    pub fn new(w: i32, h: i32, zero: bool) -> Self {
        assert!(zero, "unitialized memory is annoying",);

        let align = 1 << Engine::ALIGN_ORDER;
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
            phantom: PhantomData,
        }
    }
}

impl<Engine: BitmapEngine> Clone for Bitmap<Engine> {
    fn clone(&self) -> Self {
        let mut new = Self::new(self.w, self.h, true);
        new.buffer.copy_from_slice(&self.buffer[..]);
        new
    }
}
