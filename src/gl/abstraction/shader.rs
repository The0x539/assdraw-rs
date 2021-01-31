use gl::types::{GLchar, GLenum, GLint, GLuint};
use num_enum::TryFromPrimitive;

use super::error::{check_errors, Result};

#[repr(u32)]
#[derive(Debug, Copy, Clone, TryFromPrimitive)]
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER,
    Geometry = gl::GEOMETRY_SHADER,
    Fragment = gl::FRAGMENT_SHADER,
}

pub struct Shader(pub(super) GLuint);

impl Shader {
    pub fn new(shader_type: ShaderType) -> Self {
        let s = unsafe { gl::CreateShader(shader_type as GLenum) };
        check_errors().unwrap();
        Self(s)
    }

    fn get(&self, pname: GLenum) -> Result<GLint> {
        let mut params = 0;
        unsafe { gl::GetShaderiv(self.0, pname, &mut params) };
        check_errors()?;
        Ok(params)
    }

    pub fn shader_type(&self) -> ShaderType {
        ShaderType::try_from_primitive(self.get(gl::SHADER_TYPE).unwrap() as _).unwrap()
    }

    pub fn delete_status(&self) -> bool {
        self.get(gl::DELETE_STATUS).unwrap() != 0
    }

    pub fn compile_status(&self) -> bool {
        self.get(gl::COMPILE_STATUS).unwrap() != 0
    }

    pub fn info_log_length(&self) -> usize {
        self.get(gl::INFO_LOG_LENGTH).unwrap() as _
    }

    pub fn shader_source_length(&self) -> usize {
        self.get(gl::SHADER_SOURCE_LENGTH).unwrap() as _
    }

    pub fn source(&self, code: impl AsRef<[u8]>) {
        let code = code.as_ref();
        let length = code.len() as GLint;
        let code_ptr = code.as_ptr() as *const GLchar;

        unsafe { gl::ShaderSource(self.0, 1, &code_ptr, &length) };
        check_errors().unwrap();
    }

    pub fn compile(&self) -> bool {
        unsafe { gl::CompileShader(self.0) };
        check_errors().unwrap();
        self.compile_status()
    }

    pub fn info_log(&self) -> String {
        let mut buf = vec![0; self.info_log_length()];
        let buf_ptr = buf.as_mut_ptr() as *mut GLchar;

        let mut log_len = 0;
        unsafe {
            gl::GetShaderInfoLog(
                self.0,
                buf.len() as GLint,
                &mut log_len as *mut usize as *mut GLint,
                buf_ptr,
            );
        }
        check_errors().unwrap();
        buf.truncate(log_len);

        String::from_utf8(buf).unwrap()
    }
}
