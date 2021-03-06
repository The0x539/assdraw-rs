pub mod error;
pub use error::Result;

macro_rules! deref_wrap {
    ($ty:ty as $inner:ty) => {
        impl ::core::ops::Deref for $ty {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

pub mod shader;
pub use shader::Shader;

pub mod program;
pub use program::Program;

pub mod buffer;
pub use buffer::Buffer;

pub mod vertex_array;
pub use vertex_array::VertexArray;

pub mod texture;
pub use texture::Texture;
