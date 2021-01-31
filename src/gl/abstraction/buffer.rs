use gl::types::{GLenum, GLsizei, GLuint};

use super::error::{check_errors, Result};

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum BufferTarget {
    Array = gl::ARRAY_BUFFER,
    CopyRead = gl::COPY_READ_BUFFER,
    CopyWrite = gl::COPY_WRITE_BUFFER,
    ElementArray = gl::ELEMENT_ARRAY_BUFFER,
    PixelPack = gl::PIXEL_PACK_BUFFER,
    PixelUnpack = gl::PIXEL_UNPACK_BUFFER,
    TransformFeedback = gl::TRANSFORM_FEEDBACK_BUFFER,
    Uniform = gl::UNIFORM_BUFFER,
}

#[derive(Debug, Copy, Clone)]
pub enum Frequency {
    Stream,
    Static,
    Dynamic,
}

#[derive(Debug, Copy, Clone)]
pub enum Nature {
    Draw,
    Read,
    Copy,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum Usage {
    StreamDraw = gl::STREAM_DRAW,
    StreamRead = gl::STREAM_READ,
    StreamCopy = gl::STREAM_COPY,
    StaticDraw = gl::STATIC_DRAW,
    StaticRead = gl::STATIC_READ,
    StaticCopy = gl::STATIC_COPY,
    DynamicDraw = gl::DYNAMIC_DRAW,
    DynamicRead = gl::DYNAMIC_READ,
    DynamicCopy = gl::DYNAMIC_COPY,
}

impl From<(Frequency, Nature)> for Usage {
    fn from(value: (Frequency, Nature)) -> Self {
        match value {
            (Frequency::Stream, Nature::Draw) => Self::StreamDraw,
            (Frequency::Stream, Nature::Read) => Self::StreamRead,
            (Frequency::Stream, Nature::Copy) => Self::StreamCopy,
            (Frequency::Static, Nature::Draw) => Self::StaticDraw,
            (Frequency::Static, Nature::Read) => Self::StaticRead,
            (Frequency::Static, Nature::Copy) => Self::StaticCopy,
            (Frequency::Dynamic, Nature::Draw) => Self::DynamicDraw,
            (Frequency::Dynamic, Nature::Read) => Self::DynamicRead,
            (Frequency::Dynamic, Nature::Copy) => Self::DynamicCopy,
        }
    }
}

#[derive(Debug)]
pub struct Buffer(GLuint);
deref_wrap!(Buffer as GLuint);

impl Buffer {
    pub fn new() -> Self {
        let mut n = 0;
        unsafe { gl::GenBuffers(1, &mut n) };
        check_errors().unwrap();
        Self(n)
    }

    pub fn new_array(n: usize) -> Vec<Self> {
        let mut buf = vec![0; n];
        unsafe { gl::GenBuffers(n as GLsizei, buf.as_mut_ptr()) };

        check_errors().unwrap();
        // TODO: transmute or something?
        buf.into_iter().map(Self).collect()
    }

    pub unsafe fn bind(&self, target: BufferTarget) {
        gl::BindBuffer(target as GLenum, self.0);
        check_errors().unwrap();
    }

    pub unsafe fn buffer_data<T: Sized, U: Into<Usage>>(
        target: BufferTarget,
        data: &[T],
        usage: U,
    ) -> Result<()> {
        let size = std::mem::size_of::<T>() * data.len();
        gl::BufferData(
            target as GLenum,
            size as _,
            data.as_ptr() as *const _,
            usage.into() as GLenum,
        );
        check_errors()?;
        Ok(())
    }
}
