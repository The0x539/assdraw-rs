use thiserror::Error;

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("an unacceptable value was specified for an enumerated argument")]
    InvalidEnum = gl::INVALID_ENUM,
    #[error("a numeric argument was out of range")]
    InvalidValue = gl::INVALID_VALUE,
    #[error("the specified operation is not allowed in the current state")]
    InvalidOperation = gl::INVALID_OPERATION,
    #[error("the framebuffer object is not complete")]
    InvalidFramebufferOperation = gl::INVALID_FRAMEBUFFER_OPERATION,
    #[error("there is not enough memory left to execute the command")]
    OutOfMemory = gl::OUT_OF_MEMORY,
}

pub fn get_error() -> Option<Error> {
    let err_flag = unsafe { gl::GetError() };
    let err = match err_flag {
        gl::NO_ERROR => return None,

        gl::INVALID_ENUM => Error::InvalidEnum,
        gl::INVALID_VALUE => Error::InvalidValue,
        gl::INVALID_OPERATION => Error::InvalidOperation,
        gl::INVALID_FRAMEBUFFER_OPERATION => Error::InvalidFramebufferOperation,
        gl::OUT_OF_MEMORY => Error::OutOfMemory,

        other => panic!("Unrecognized  OpenGL error code: {}", other),
    };
    Some(err)
}

pub fn check_errors() -> Result<(), Vec<Error>> {
    let v: Vec<Error> = std::iter::from_fn(get_error).collect();
    if v.is_empty() {
        Ok(())
    } else {
        Err(v)
    }
}
