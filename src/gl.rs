use native_windows_gui as nwg;

#[rustfmt::skip]
use glutin::{
    ContextBuilder, GlRequest, GlProfile, PossiblyCurrent, RawContext, Api,
    dpi::PhysicalSize,
    platform::windows::RawContextExt,
};
use ab_glyph_rasterizer::Rasterizer;
use cstr::cstr;
use image::ImageDecoder;
use once_cell::unsync::OnceCell;

use std::cell::{Cell, Ref, RefCell, RefMut};
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
    pub(crate) canvas: nwg::ExternCanvas,

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

    drawing: RefCell<DrawingData>,

    dimensions: Cell<Dimensions>,
    drawing_pos: Cell<[f32; 2]>,
}

struct DrawingData {
    // TODO: alpha-only data. should be faster
    pixels: Vec<u32>,
    points: Vec<f32>,
    rasterizer: Rasterizer,
}

impl Default for DrawingData {
    fn default() -> Self {
        Self {
            pixels: Vec::new(),
            points: Vec::new(),
            rasterizer: Rasterizer::new(0, 0),
        }
    }
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

            self.drawing.replace(Default::default());

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
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            check_errors().unwrap();

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

    fn drawing_points(&self) -> Ref<Vec<f32>> {
        Ref::map(self.drawing.borrow(), |x| &x.points)
    }

    fn drawing_points_mut(&self) -> RefMut<Vec<f32>> {
        RefMut::map(self.drawing.borrow_mut(), |x| &mut x.points)
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
            let pos_loc = self
                .img_prgm
                .get()
                .unwrap()
                .get_uniform_location(cstr!("drawing_pos"))
                .unwrap()
                .unwrap();
            let pos = self.drawing_pos.get();
            gl::Uniform2f(*pos_loc, pos[0], pos[1]);
            self.shape_tex.get().unwrap().bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::Uniform2f(*pos_loc, 0.0, 0.0);

            self.points_vao.get().unwrap().bind();
            gl::UseProgram(**self.draw_prgm.get().unwrap());
            self.update_dimension_uniforms();
            gl::DrawArrays(gl::POINTS, 0, self.drawing_points().len() as i32 / 2);
            gl::DrawArrays(gl::LINE_STRIP, 0, self.drawing_points().len() as i32 / 2);

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
        let mut points = self.drawing_points_mut();
        points.push(x);
        points.push(y);
        drop(points);
        self.update_drawing();
    }

    pub fn pop_point(&self) -> Option<(f32, f32)> {
        let mut points = self.drawing_points_mut();
        let x = points.pop()?;
        let y = points.pop().unwrap();
        drop(points);
        self.update_drawing();
        Some((x, y))
    }

    pub fn update_drawing(&self) {
        let drawing = self.drawing.borrow_mut();

        unsafe {
            self.points_vb.get().unwrap().bind(BufferTarget::Array);
            Buffer::buffer_data(
                BufferTarget::Array,
                drawing.points.as_slice(),
                Usage::StaticDraw,
            )
            .unwrap();
        }

        let points = &drawing.points;
        if points.len() < 6 {
            return;
        }

        /*
        let mut text = format!("m {} {} l", points[0], points[1]);
        for point in &points[2..] {
            use std::fmt::Write;
            write!(&mut text, " {}", point).unwrap();
        }

        let (segments, bbox) = crate::drawing::parse_drawing(text);
        */

        use crate::ass_outline::{Segment, Vector};

        let mut bbox = crate::ass_outline::Rect::default();
        bbox.reset();
        let mut segments = vec![];
        for i in (0..points.len()).step_by(2) {
            let p0 = Vector {
                x: (points[i + 0] * 64.0) as i32,
                y: (points[i + 1] * 64.0) as i32,
            };
            let p1 = if i + 3 < points.len() {
                Vector {
                    x: (points[i + 2] * 64.0) as i32,
                    y: (points[i + 3] * 64.0) as i32,
                }
            } else {
                Vector {
                    x: (points[0] * 64.0) as i32,
                    y: (points[1] * 64.0) as i32,
                }
            };
            bbox.update_point(p0);
            bbox.update_point(p1);
            segments.push(Segment::LineSegment(p0, p1));
        }

        let x0 = bbox.x_min as f32 / 64.0;
        let x1 = bbox.x_max as f32 / 64.0;
        let y0 = bbox.y_min as f32 / 64.0;
        let y1 = bbox.y_max as f32 / 64.0;
        let (width, height) = (x1 - x0, y1 - y0);

        let (mut rasterizer, mut img_buf) =
            RefMut::map_split(drawing, |r| (&mut r.rasterizer, &mut r.pixels));
        rasterizer.reset(width as usize, height as usize);

        for segment in segments {
            println!("{:?}", segment);
            use ab_glyph_rasterizer::Point;
            let cnv = |p: Vector| -> Point {
                let x = p.x as f32 / 64.0;
                let y = p.y as f32 / 64.0;
                (x - x0, y - y0).into()
            };
            match segment {
                Segment::LineSegment(p0, p1) => {
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

        //img_buf.clear();
        img_buf.resize(width as usize * height as usize, 0);
        rasterizer.for_each_pixel_2d(|x, y, v| {
            let i = x as usize + (y as usize * width as usize);
            let px = (v * 127.0) as u8;
            img_buf[i] = u32::from_ne_bytes([px, 127, 127, 127]);
        });

        self.drawing_pos.set([x0, y0]);

        unsafe {
            /*
            #[rustfmt::skip]
            let vertex_data = &[
                x0, y0,
                x1, y0,
                x0, y1,
                x1, y1,
            ];
            */
            #[rustfmt::skip]
            let vertex_data = &[
                0.0, 0.0,
                width, 0.0,
                0.0, height,
                width, height,
            ];

            self.shape_tex.get().unwrap().bind(TextureTarget::Rectangle);
            gl::TexImage2D(
                gl::TEXTURE_RECTANGLE,
                0,
                gl::RGBA8 as _,
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
