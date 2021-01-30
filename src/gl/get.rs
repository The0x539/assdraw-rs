use bytemuck::Pod;
use gl::types::{GLboolean, GLdouble, GLenum, GLfloat, GLint, GLint64};

type Getter<T> = unsafe fn(pname: GLenum, params: *mut T);

pub unsafe trait GlGet<T: Pod = Self>: Pod {
    const F: Getter<T>;
}

unsafe impl GlGet for GLboolean {
    const F: Getter<Self> = gl::GetBooleanv;
}

unsafe impl GlGet for GLdouble {
    const F: Getter<Self> = gl::GetDoublev;
}

unsafe impl GlGet for GLfloat {
    const F: Getter<Self> = gl::GetFloatv;
}

unsafe impl GlGet for GLint {
    const F: Getter<Self> = gl::GetIntegerv;
}

unsafe impl GlGet for GLint64 {
    const F: Getter<Self> = gl::GetInteger64v;
}

unsafe impl<T: GlGet, const N: usize> GlGet<T> for [T; N]
where
    [T; N]: Pod,
{
    const F: Getter<T> = T::F;
}

pub unsafe fn get<T: GlGet<U>, U: Pod>(pname: GLenum) -> T {
    let mut val = T::zeroed();
    T::F(pname, &mut val as *mut T as *mut U);
    loop {
        match gl::GetError() {
            gl::INVALID_ENUM => panic!("invalid enum"),
            gl::INVALID_VALUE => panic!("invalid value"),
            gl::NO_ERROR => break,
            e => panic!("Unexpected error: {}", e),
        }
    }
    val
}

pub fn get_viewport() -> ((i32, i32), (i32, i32)) {
    let [x, y, w, h]: [i32; 4] = unsafe { get(gl::VIEWPORT) };
    ((x, y), (w, h))
}
