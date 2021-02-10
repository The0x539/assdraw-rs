use gl::types::{GLenum, GLsizei, GLuint};

use super::error::check_errors;

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum TextureTarget {
    Single1D = gl::TEXTURE_1D,
    Single2D = gl::TEXTURE_2D,
    Single3D = gl::TEXTURE_3D,
    Array1D = gl::TEXTURE_1D_ARRAY,
    Array2D = gl::TEXTURE_2D_ARRAY,
    Rectangle = gl::TEXTURE_RECTANGLE,
    CubeMap = gl::TEXTURE_CUBE_MAP,
    Buffer = gl::TEXTURE_BUFFER,
    Multisample = gl::TEXTURE_2D_MULTISAMPLE,
    MultisampleArray = gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
}

#[derive(Debug)]
pub struct Texture(GLuint);
deref_wrap!(Texture as GLuint);

impl Texture {
    pub fn new() -> Self {
        let mut n = 0;
        unsafe { gl::GenTextures(1, &mut n) };
        check_errors().unwrap();
        Self(n)
    }

    pub fn new_array(n: usize) -> Vec<Self> {
        let mut buf = vec![0; n];
        unsafe { gl::GenTextures(n as GLsizei, buf.as_mut_ptr()) };

        check_errors().unwrap();
        // TODO: transmute or something?
        buf.into_iter().map(Self).collect()
    }

    pub unsafe fn bind(&self, target: TextureTarget) {
        gl::BindTexture(target as GLenum, self.0);
        check_errors().unwrap();
    }
}
