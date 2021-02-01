use native_windows_gui as nwg;

#[rustfmt::skip]
use glutin::{
    ContextBuilder, GlRequest, GlProfile, PossiblyCurrent, RawContext, Api,
    dpi::PhysicalSize,
    platform::windows::RawContextExt,
};
use cstr::cstr;
use image::ImageDecoder;
use once_cell::unsync::OnceCell;

use std::cell::{Cell, RefCell};
use std::convert::TryInto;

pub mod abstraction;
use abstraction::{
    buffer::{Buffer, BufferTarget, Usage},
    error::check_errors,
    program::Program,
    shader::{Shader, ShaderType},
    vertex_array::VertexArray,
};

mod get;

type Ctx = RawContext<PossiblyCurrent>;

use gl::types::{GLfloat, GLint};

#[derive(Default, Copy, Clone, Debug)]
pub struct Dimensions {
    pub screen_dims: [GLfloat; 2],
    pub scene_pos: [GLfloat; 2],
    pub scale: GLfloat,
}

#[derive(Default)]
pub struct OpenGlCanvas {
    ctx: OnceCell<Ctx>,
    canvas: nwg::ExternCanvas,

    img_prgm: OnceCell<Program>,
    draw_prgm: OnceCell<Program>,

    img_vb: OnceCell<Buffer>,
    draw_vb: OnceCell<Buffer>,

    img_vao: OnceCell<VertexArray>,
    draw_vao: OnceCell<VertexArray>,

    drawing: RefCell<Vec<f32>>,

    dimensions: Cell<Dimensions>,
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

            let vs = Shader::build(ShaderType::Vertex, include_str!("vs.glsl"));
            let img_fs = Shader::build(ShaderType::Fragment, include_str!("fs.glsl"));
            let draw_fs = Shader::build(ShaderType::Fragment, include_str!("blue.glsl"));

            self.img_prgm.set(Program::build(&vs, &img_fs)).unwrap();
            self.draw_prgm.set(Program::build(&vs, &draw_fs)).unwrap();

            self.drawing
                .replace(vec![0.0, 0.0, 100.0, 50.0, 200.0, 200.0]);

            let draw_vb = Buffer::new();
            draw_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &[0_f32; 6], Usage::StaticDraw).unwrap();
            self.draw_vb.set(draw_vb).unwrap();

            let draw_vao = VertexArray::new();
            draw_vao.bind();
            self.draw_vao.set(draw_vao).unwrap();
            gl::EnableVertexAttribArray(0);
            let stride = mem::size_of::<f32>() * 2;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, stride as _, ptr::null());

            let img_vb = Buffer::new();
            img_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &[0_f32; 8], Usage::StaticDraw).unwrap();
            self.img_vb.set(img_vb).unwrap();

            let img_vao = VertexArray::new();
            img_vao.bind();
            self.img_vao.set(img_vao).unwrap();
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, stride as _, ptr::null());

            let mut tex = 0;
            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_RECTANGLE, tex);

            gl::PointSize(5.0);

            let default_dims = Dimensions {
                screen_dims: [100.0, 100.0],
                scene_pos: [0.0, 0.0],
                scale: 1.0,
            };
            self.set_dimensions(default_dims);

            self.ctx.set(ctx).expect("context was already created");
        }
    }

    fn with_ctx<F: FnOnce(&Ctx) -> T, T>(&self, f: F) -> Option<T> {
        self.ctx.get().map(f)
    }

    pub fn render(&self) {
        self.with_ctx(|ctx| unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            self.img_vao.get().unwrap().bind();
            gl::UseProgram(**self.img_prgm.get().unwrap());
            self.update_dimension_uniforms();
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.draw_vao.get().unwrap().bind();
            gl::UseProgram(**self.draw_prgm.get().unwrap());
            self.update_dimension_uniforms();
            self.update_drawing();
            gl::DrawArrays(gl::POINTS, 0, self.drawing.borrow().len() as i32 / 2);
            gl::DrawArrays(gl::LINES, 0, self.drawing.borrow().len() as i32 / 2);

            check_errors().unwrap();

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
    }

    fn update_dimension_uniforms(&self) {
        let dims = self.get_dimensions();
        let prog = self.img_prgm.get().unwrap();

        let uniform = |name| prog.get_uniform_location(name).unwrap().unwrap();
        let screen_dims_loc = uniform(cstr!("screen_dims"));
        let scene_pos_loc = uniform(cstr!("scene_pos"));
        let scale_loc = uniform(cstr!("scale"));

        unsafe {
            gl::Uniform2f(*screen_dims_loc, dims.screen_dims[0], dims.screen_dims[1]);
            gl::Uniform2f(*scene_pos_loc, dims.scene_pos[0], dims.scene_pos[1]);
            gl::Uniform1f(*scale_loc, dims.scale);
        }
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

            self.img_vb.get().unwrap().bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, vertex_data, Usage::StaticDraw).unwrap();
        }
    }

    pub fn update_drawing(&self) {
        unsafe {
            self.draw_vb.get().unwrap().bind(BufferTarget::Array);
            Buffer::buffer_data(
                BufferTarget::Array,
                self.drawing.borrow().as_slice(),
                Usage::StaticDraw,
            )
            .unwrap();
        }
    }
}

nwg::subclass_control!(OpenGlCanvas, ExternCanvas, canvas);
