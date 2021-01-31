use std::ffi::CStr;

use gl::types::{GLchar, GLenum, GLint, GLuint};

use super::error::{check_errors, Result};

pub struct AttributeLocation(GLuint);
deref_wrap!(AttributeLocation as GLuint);

pub struct UniformLocation(GLuint);
deref_wrap!(UniformLocation as GLuint);

pub struct Program(GLuint);
deref_wrap!(Program as GLuint);

impl Program {
    pub fn new() -> Self {
        let p = unsafe { gl::CreateProgram() };
        check_errors().unwrap();
        assert_ne!(p, 0);
        Self(p)
    }

    pub fn attach_shader(&self, shader: &super::Shader) -> Result<()> {
        unsafe { gl::AttachShader(self.0, shader.0) };
        check_errors()?;
        Ok(())
    }

    fn get(&self, pname: GLenum) -> Result<GLint> {
        let mut params = 0;
        unsafe { gl::GetProgramiv(self.0, pname, &mut params) };
        check_errors()?;
        Ok(params)
    }

    pub fn link_status(&self) -> bool {
        self.get(gl::LINK_STATUS).unwrap() != 0
    }

    pub fn info_log_length(&self) -> usize {
        self.get(gl::INFO_LOG_LENGTH).unwrap() as _
    }

    pub fn info_log(&self) -> String {
        let mut buf = vec![0; self.info_log_length()];
        let buf_ptr = buf.as_mut_ptr() as *mut GLchar;

        let mut log_len = 0;
        unsafe {
            gl::GetProgramInfoLog(
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

    pub fn link(&self) -> bool {
        unsafe { gl::LinkProgram(self.0) };
        check_errors().unwrap();
        self.link_status()
    }

    pub fn get_attrib_location(&self, name: &CStr) -> Result<Option<AttributeLocation>> {
        let loc = unsafe { gl::GetAttribLocation(self.0, name.as_ptr().cast()) };
        check_errors()?;
        if loc < 0 {
            Ok(None)
        } else {
            Ok(Some(AttributeLocation(loc as _)))
        }
    }

    pub fn get_uniform_location(&self, name: &CStr) -> Result<Option<UniformLocation>> {
        let loc = unsafe { gl::GetUniformLocation(self.0, name.as_ptr().cast()) };
        check_errors()?;
        if loc < 0 {
            Ok(None)
        } else {
            Ok(Some(UniformLocation(loc as _)))
        }
    }
}
