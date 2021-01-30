use native_windows_gui as nwg;

#[rustfmt::skip]
use glutin::{
    ContextBuilder, GlRequest, GlProfile, PossiblyCurrent, RawContext, Api,
    dpi::PhysicalSize,
    platform::windows::RawContextExt,
};
use std::cell::{Cell, RefCell};

mod get;

type Ctx = RawContext<PossiblyCurrent>;

#[derive(Default, Copy, Clone)]
struct MiscState {
    program: gl::types::GLuint,
    vp_pos_loc: gl::types::GLint,
    vp_size_loc: gl::types::GLint,
    offset_loc: gl::types::GLint,
    delta_loc: gl::types::GLint,
}

#[derive(Default)]
pub struct OpenGlCanvas {
    ctx: RefCell<Option<Ctx>>,
    state: Cell<MiscState>,
    canvas: nwg::ExternCanvas,
}

unsafe fn single_shader_source(shader: gl::types::GLuint, source: &[u8]) -> () {
    let string = source.as_ptr().cast();
    let length = source.len() as i32;

    gl::ShaderSource(shader, 1, &string, &length);
}

#[allow(dead_code)]
unsafe fn shader_info_log(shader: gl::types::GLuint) -> String {
    let mut buf_len = 0;
    gl::GetShaderiv(shader, gl::SHADER_SOURCE_LENGTH, &mut buf_len);
    let mut buf = vec![0u8; buf_len as usize];

    let mut log_len = 0;
    gl::GetShaderInfoLog(shader, buf_len, &mut log_len, buf.as_mut_ptr().cast());

    String::from_utf8_lossy(&buf[..log_len as usize]).into_owned()
}

impl OpenGlCanvas {
    pub fn create_context(&self) {
        use std::{ffi::c_void, mem, ptr};

        unsafe {
            let ctx = ContextBuilder::new()
                .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
                .with_gl_profile(GlProfile::Core)
                .build_raw_context(self.canvas.handle.hwnd().unwrap() as *mut c_void)
                .expect("Failed to build opengl context")
                .make_current()
                .expect("Failed to set opengl context as current");

            gl::load_with(|s| ctx.get_proc_address(s) as *const c_void);

            gl::ClearColor(0.0, 0.0, 0.0, 1.0);

            const VS_SRC: &'static [u8] = include_bytes!("vs.glsl");
            let vs = gl::CreateShader(gl::VERTEX_SHADER);
            single_shader_source(vs, VS_SRC);
            gl::CompileShader(vs);

            const FS_SRC: &'static [u8] = include_bytes!("fs.glsl");
            let fs = gl::CreateShader(gl::FRAGMENT_SHADER);
            single_shader_source(fs, FS_SRC);
            gl::CompileShader(fs);

            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);
            gl::UseProgram(program);

            let get_uniform = |name: &[u8]| gl::GetUniformLocation(program, name.as_ptr().cast());
            self.state.set(MiscState {
                program,
                vp_pos_loc: get_uniform(b"u_vpPos\0"),
                vp_size_loc: get_uniform(b"u_vpSize\0"),
                offset_loc: get_uniform(b"u_Offset\0"),
                delta_loc: get_uniform(b"u_Delta\0"),
            });

            #[rustfmt::skip]
            let vertex_data: &[f32] = &[
                10.0, 10.0, 1.0, 1.0, 1.0,
                10.0, 90.0, 1.0, 0.5, 1.0,
                90.0, 10.0, 1.0, 1.0, 0.5,
            ];

            let vertex_size = vertex_data.len() * mem::size_of::<f32>();

            let mut vb = 0;
            gl::GenBuffers(1, &mut vb);
            gl::BindBuffer(gl::ARRAY_BUFFER, vb);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                vertex_size as _,
                vertex_data.as_ptr().cast(),
                gl::STATIC_DRAW,
            );

            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);

            let stride = mem::size_of::<f32>() * 5;
            let color_offset = 8 as *const c_void;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, stride as _, ptr::null());
            gl::VertexAttribPointer(1, 4, gl::FLOAT, 0, stride as _, color_offset);

            *self.ctx.borrow_mut() = Some(ctx);
        }
    }

    fn with_ctx<F: FnOnce(&Ctx) -> T, T>(&self, f: F) -> Option<T> {
        self.ctx.borrow().as_ref().map(f)
    }

    pub fn render(&self) {
        self.with_ctx(|ctx| unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            ctx.swap_buffers().unwrap();
        });
    }

    pub fn resize(&self) {
        self.with_ctx(|ctx| unsafe {
            let (w, h) = self.canvas.physical_size();
            let ((vx, vy), (vw, vh)) = get::get_viewport();
            let MiscState {
                vp_pos_loc,
                vp_size_loc,
                ..
            } = self.state.get();
            gl::Uniform2i(vp_pos_loc, vx, vy);
            gl::Uniform2i(vp_size_loc, vw, vh);
            gl::Viewport(0, 0, w as i32, h as i32);
            ctx.resize(PhysicalSize::new(w, h));
        });
    }

    pub fn set_delta(&self, (dx, dy): (i32, i32)) {
        unsafe {
            gl::Uniform2i(self.state.get().delta_loc, dx, dy);
        }
    }

    pub fn commit_delta(&self) {
        let state = self.state.get();
        unsafe {
            let mut offset = [0, 0];
            gl::GetUniformiv(state.program, state.offset_loc, offset.as_mut_ptr().cast());
            let mut delta = [0, 0];
            gl::GetUniformiv(state.program, state.delta_loc, delta.as_mut_ptr().cast());

            gl::Uniform2i(state.offset_loc, offset[0] + delta[0], offset[1] + delta[1]);
            gl::Uniform2i(state.delta_loc, 0, 0);
        }
    }
}

nwg::subclass_control!(OpenGlCanvas, ExternCanvas, canvas);
