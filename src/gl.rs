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
    texture::{Texture, TextureTarget},
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
    points_vb: OnceCell<Buffer>,
    shape_vb: OnceCell<Buffer>,

    img_vao: OnceCell<VertexArray>,
    points_vao: OnceCell<VertexArray>,
    shape_vao: OnceCell<VertexArray>,

    img_tex: OnceCell<Texture>,
    shape_tex: OnceCell<Texture>,

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

            self.drawing.replace(vec![]);

            let points_vb = Buffer::new();
            self.points_vb.set(points_vb).unwrap();
            self.update_drawing();

            let points_vao = VertexArray::new();
            points_vao.bind();
            self.points_vao.set(points_vao).unwrap();
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

            let img_tex = Texture::new();
            img_tex.bind(TextureTarget::Rectangle);
            self.img_tex.set(img_tex).unwrap();

            let shape_vb = Buffer::new();
            shape_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &[0_f32; 8], Usage::StaticDraw).unwrap();
            self.shape_vb.set(shape_vb).unwrap();

            let shape_vao = VertexArray::new();
            shape_vao.bind();
            self.shape_vao.set(shape_vao).unwrap();
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, stride as _, ptr::null());

            let shape_tex = Texture::new();
            shape_tex.bind(TextureTarget::Rectangle);
            self.shape_tex.set(shape_tex).unwrap();

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
            self.img_tex.get().unwrap().bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.shape_vao.get().unwrap().bind();
            self.update_dimension_uniforms();
            self.shape_tex.get().unwrap().bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.points_vao.get().unwrap().bind();
            gl::UseProgram(**self.draw_prgm.get().unwrap());
            self.update_dimension_uniforms();
            gl::DrawArrays(gl::POINTS, 0, self.drawing.borrow().len() as i32 / 2);
            gl::DrawArrays(gl::LINE_STRIP, 0, self.drawing.borrow().len() as i32 / 2);

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
            self.img_tex.get().unwrap().bind(TextureTarget::Rectangle);
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

    pub fn add_point(&self, x: f32, y: f32) {
        let mut drawing = self.drawing.borrow_mut();
        drawing.push(x);
        drawing.push(y);
        drop(drawing);
        self.update_drawing();
    }

    pub fn update_drawing(&self) {
        unsafe {
            self.points_vb.get().unwrap().bind(BufferTarget::Array);
            Buffer::buffer_data(
                BufferTarget::Array,
                self.drawing.borrow().as_slice(),
                Usage::StaticDraw,
            )
            .unwrap();
        }

        let points = self.drawing.borrow();
        if points.len() < 6 {
            return;
        }

        let mut text = format!("m {} {} l", points[0], points[1]);
        for point in &points[2..] {
            use std::fmt::Write;
            write!(&mut text, " {}", point).unwrap();
        }

        let (outline, bbox) = crate::drawing::parse_drawing(text);
        let x0 = bbox.x_min as f32 / 64.0;
        let x1 = bbox.x_max as f32 / 64.0;
        let y0 = bbox.y_min as f32 / 64.0;
        let y1 = bbox.y_max as f32 / 64.0;
        let (width, height) = dbg!((x1 - x0, y1 - y0));

        let mut rasterizer = ab_glyph_rasterizer::Rasterizer::new(width as _, height as _);

        println!("{:?}", outline);

        for segment in outline.segments() {
            use crate::ass::outline::{Segment, Vector};
            use ab_glyph_rasterizer::Point;
            let cnv = |p: Vector| -> Point {
                let x = p.x as f32 / 64.0;
                let y = p.y as f32 / 64.0;
                (x - x0, y - y0).into()
            };
            match segment {
                Segment::LineSegment(p0, p1) => {
                    let (p_0, p_1) = (cnv(p0), cnv(p1));
                    println!("{:?} {:?}", p_0, p_1);
                    rasterizer.draw_line(cnv(p0), cnv(p1));
                }
                Segment::QuadSpline(p0, p1, p2) => {
                    rasterizer.draw_quad(cnv(p0), cnv(p1), cnv(p2));
                }
                Segment::CubicSpline(p0, p1, p2, p3) => {
                    rasterizer.draw_cubic(cnv(p0), cnv(p1), cnv(p2), cnv(p3));
                }
            }
        }

        let mut img_buf = vec![0u32; width as usize * height as usize];
        rasterizer.for_each_pixel(|i, v| {
            let v2 = if v == 0.0 { 0x5C94C87F_u32 } else { 0 };
            img_buf[i] = v2;
        });

        unsafe {
            #[rustfmt::skip]
            let vertex_data = &[
                x0, y0,
                x1, y0,
                x0, y1,
                x1, y1,
            ];

            self.shape_tex.get().unwrap().bind(TextureTarget::Rectangle);
            gl::TexImage2D(
                gl::TEXTURE_RECTANGLE,
                0,
                gl::RGB8 as _,
                width as _,
                height as _,
                0,
                gl::BGRA,
                gl::UNSIGNED_INT_8_8_8_8,
                img_buf.as_ptr().cast(),
            );

            self.shape_vb.get().unwrap().bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, vertex_data, Usage::StaticDraw).unwrap();
        }
    }
}

nwg::subclass_control!(OpenGlCanvas, ExternCanvas, canvas);
