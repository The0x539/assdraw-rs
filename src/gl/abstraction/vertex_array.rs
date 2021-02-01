use gl::types::{GLsizei, GLuint};

use super::error::check_errors;

#[derive(Debug)]
pub struct VertexArray(GLuint);
deref_wrap!(VertexArray as GLuint);

impl VertexArray {
    pub fn new() -> Self {
        let mut n = 0;
        unsafe { gl::GenVertexArrays(1, &mut n) };
        check_errors().unwrap();
        Self(n)
    }

    pub fn new_array(n: usize) -> Vec<Self> {
        let mut buf = vec![0; n];
        unsafe { gl::GenVertexArrays(n as GLsizei, buf.as_mut_ptr()) };

        check_errors().unwrap();
        // TODO: transmute or something?
        buf.into_iter().map(Self).collect()
    }

    pub unsafe fn bind(&self) {
        gl::BindVertexArray(self.0);
        check_errors().unwrap();
    }
}
