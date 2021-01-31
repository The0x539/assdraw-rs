use native_windows_gui as nwg;

#[rustfmt::skip]
use glutin::{
    ContextBuilder, GlRequest, GlProfile, PossiblyCurrent, RawContext, Api,
    dpi::PhysicalSize,
    platform::windows::RawContextExt,
};
use image::ImageDecoder;

use std::cell::{Cell, RefCell};
use std::convert::TryInto;

pub mod abstraction;
use abstraction::{
    program::Program,
    shader::{Shader, ShaderType},
};

mod get;

type Ctx = RawContext<PossiblyCurrent>;

use gl::types::{GLfloat, GLint, GLuint};

#[derive(Default, Copy, Clone, Debug)]
pub struct Dimensions {
    pub screen_dims: [GLfloat; 2],
    pub scene_pos: [GLfloat; 2],
    pub scale: GLfloat,
}

#[derive(Default)]
pub struct OpenGlCanvas {
    ctx: RefCell<Option<Ctx>>,
    program: Cell<GLuint>,
    dimensions: Cell<Dimensions>,
    canvas: nwg::ExternCanvas,
}

fn slice_size<T: Sized>(s: &[T]) -> usize {
    s.len() * std::mem::size_of::<T>()
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
            let vs = Shader::new(ShaderType::Vertex);
            vs.source(VS_SRC);

            let did_compile = vs.compile();
            print!("{}", vs.info_log());
            assert!(did_compile);

            const FS_SRC: &'static [u8] = include_bytes!("fs.glsl");
            let fs = Shader::new(ShaderType::Fragment);
            fs.source(FS_SRC);

            let did_compile = fs.compile();
            print!("{}", fs.info_log());
            assert!(did_compile);

            let program = Program::new();
            program.attach_shader(&vs).unwrap();
            program.attach_shader(&fs).unwrap();

            let did_link = program.link();
            print!("{}", program.info_log());
            assert!(did_link);

            gl::UseProgram(*program);
            self.program.set(*program);

            #[rustfmt::skip]
            let vertex_data: &[f32] = &[
                0.0, 0.0,
                0.0, 0.0,
                0.0, 0.0,
                0.0, 0.0,
            ];

            let mut vb = 0;
            gl::GenBuffers(1, &mut vb);
            gl::BindBuffer(gl::ARRAY_BUFFER, vb);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                slice_size(vertex_data) as _,
                vertex_data.as_ptr().cast(),
                gl::STATIC_DRAW,
            );

            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::EnableVertexAttribArray(0);

            let stride = mem::size_of::<f32>() * 2;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, stride as _, ptr::null());

            let mut tex = 0;
            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_RECTANGLE, tex);

            let default_dims = Dimensions {
                screen_dims: [100.0, 100.0],
                scene_pos: [0.0, 0.0],
                scale: 1.0,
            };
            self.set_dimensions(default_dims);

            *self.ctx.borrow_mut() = Some(ctx);
        }
    }

    fn with_ctx<F: FnOnce(&Ctx) -> T, T>(&self, f: F) -> Option<T> {
        self.ctx.borrow().as_ref().map(f)
    }

    pub fn render(&self) {
        self.with_ctx(|ctx| unsafe {
            get::get_errors().unwrap();
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            ctx.swap_buffers().unwrap();
        });
    }

    pub fn resize(&self) {
        let (w, h) = self.canvas.physical_size();
        self.update_dimensions(|dims| dims.screen_dims = [w as _, h as _]);
        unsafe {
            gl::Viewport(0, 0, w as _, h as _);
        }
        self.with_ctx(|ctx| ctx.resize(PhysicalSize::new(w, h)));
    }

    pub fn get_dimensions(&self) -> Dimensions {
        self.dimensions.get()
    }

    pub fn update_dimensions<F: FnOnce(&mut Dimensions)>(&self, f: F) {
        let mut dims = self.dimensions.get();
        f(&mut dims);
        self.set_dimensions(dims);
    }

    pub fn set_dimensions(&self, dims: Dimensions) {
        self.dimensions.set(dims);
        unsafe {
            let prog = self.program.get();
            let get_uniform_loc = |name: &[u8]| gl::GetUniformLocation(prog, name.as_ptr().cast());
            gl::Uniform2f(
                get_uniform_loc(b"u_Dims.screen_dims\0"),
                dims.screen_dims[0],
                dims.screen_dims[1],
            );
            gl::Uniform2f(
                get_uniform_loc(b"u_Dims.scene_pos\0"),
                dims.scene_pos[0],
                dims.scene_pos[1],
            );
            gl::Uniform1f(get_uniform_loc(b"u_Dims.scale\0"), dims.scale);
        }
        get::get_errors().unwrap();
    }

    pub fn set_image<'a>(&self, img: impl ImageDecoder<'a>) {
        let (width, height) = img.dimensions();

        if img.color_type() != image::ColorType::Rgb8 {
            println!("unexpected color format: {:?}", img.color_type());
            return;
        }

        let buf_len: usize = img.total_bytes().try_into().expect("image too large");
        let mut buf = vec![0; buf_len];
        img.read_image(&mut buf[..]).unwrap();

        let buf2 = buf
            .chunks_exact(3)
            .map(|rgb| {
                let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
                vec![127, r, g, b]
            })
            .flatten()
            .collect::<Vec<u8>>();

        #[rustfmt::skip]
        let vertex_data = &[
            0.0, 0.0,
            width as f32, 0.0,
            0.0, height as f32,
            width as f32, height as f32,
        ];

        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_RECTANGLE,
                0,
                gl::RGB8 as _,
                width as GLint,
                height as GLint,
                0,
                gl::BGRA,
                gl::UNSIGNED_INT_8_8_8_8,
                buf2.as_ptr().cast(),
            );

            gl::BufferData(
                gl::ARRAY_BUFFER,
                slice_size(vertex_data) as _,
                vertex_data.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
        }
    }
}

nwg::subclass_control!(OpenGlCanvas, ExternCanvas, canvas);
