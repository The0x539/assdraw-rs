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
    pub screen_dims: Point,
    pub scene_pos: Point,
    pub scale: GLfloat,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl From<Point> for [f32; 2] {
    fn from(value: Point) -> Self {
        [value.x, value.y]
    }
}

impl From<[f32; 2]> for Point {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

pub struct OpenGlCanvas {
    ctx: Ctx,
    canvas: nwg::ExternCanvas,

    img_prgm: Program,
    draw_prgm: Program,
    shape_prgm: Program,

    img_vb: Buffer,
    points_vb: Buffer,
    shape_vb: Buffer,

    img_vao: VertexArray,
    points_vao: VertexArray,
    shape_vao: VertexArray,

    img_tex: Texture,
    shape_tex: Texture,

    drawing: RefCell<DrawingData>,

    dimensions: Cell<Dimensions>,
    drawing_pos: Cell<Point>,
}

struct DrawingData {
    pixels: Vec<u8>,
    points: Vec<Point>,
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

fn make_extern_canvas<W: Into<nwg::ControlHandle>>(parent: W) -> nwg::ExternCanvas {
    let mut c = nwg::ExternCanvas::default();
    nwg::ExternCanvas::builder()
        .parent(Some(parent.into()))
        .build(&mut c)
        .expect("Failed to build nwg::ExternCanvas");
    c
}

#[allow(dead_code)]
impl OpenGlCanvas {
    pub fn handle(&self) -> &nwg::ControlHandle {
        &self.canvas.handle
    }

    pub fn new<W: Into<nwg::ControlHandle>>(parent: W) -> Self {
        use std::ffi::c_void;
        const NULL: *const c_void = std::ptr::null();

        let canvas = make_extern_canvas(parent);

        let ctx = unsafe {
            let ctx = ContextBuilder::new()
                .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
                .with_gl_profile(GlProfile::Core)
                .build_raw_context(canvas.handle.hwnd().unwrap() as *mut c_void)
                .expect("Failed to build opengl context")
                .make_current()
                .expect("Failed to set opengl context as current");

            gl::load_with(|s| ctx.get_proc_address(s) as *const c_void);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            ctx
        };

        let (img_prgm, draw_prgm, shape_prgm) = {
            let vs = Shader::build(ShaderType::Vertex, include_str!("vs.glsl"));
            let img_fs = Shader::build(ShaderType::Fragment, include_str!("fs.glsl"));
            let draw_fs = Shader::build(ShaderType::Fragment, include_str!("blue.glsl"));
            let shape_fs = Shader::build(ShaderType::Fragment, include_str!("draw.glsl"));

            let build = |fs| Program::build(&vs, fs);
            (build(&img_fs), build(&draw_fs), build(&shape_fs))
        };

        let drawing = RefCell::new(DrawingData::default());

        const VEC2_STRIDE: i32 = (std::mem::size_of::<f32>() * 2) as i32;

        let (points_vb, points_vao) = unsafe {
            let vb = Buffer::new();
            vb.bind(BufferTarget::Array);

            let vao = VertexArray::new();
            vao.bind();
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, VEC2_STRIDE, NULL);

            (vb, vao)
        };

        let (img_vb, img_vao, img_tex) = unsafe {
            let vb = Buffer::new();
            vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &[0_f32; 8], Usage::StaticDraw).unwrap();

            let vao = VertexArray::new();
            vao.bind();
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, VEC2_STRIDE, NULL);

            let tex = Texture::new();
            tex.bind(TextureTarget::Rectangle);

            (vb, vao, tex)
        };

        let (shape_vb, shape_vao, shape_tex) = unsafe {
            let vb = Buffer::new();
            vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &[0_f32; 8], Usage::StaticDraw).unwrap();

            let vao = VertexArray::new();
            vao.bind();
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, VEC2_STRIDE, NULL);

            let tex = Texture::new();
            tex.bind(TextureTarget::Rectangle);

            gl::PointSize(5.0);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            check_errors().unwrap();

            (vb, vao, tex)
        };

        let dimensions = Dimensions {
            screen_dims: [100.0, 100.0].into(),
            scene_pos: [0.0, 0.0].into(),
            scale: 1.0,
        };

        Self {
            ctx,
            canvas,

            img_prgm,
            draw_prgm,
            shape_prgm,

            img_vb,
            points_vb,
            shape_vb,

            img_vao,
            points_vao,
            shape_vao,

            img_tex,
            shape_tex,

            drawing,

            dimensions: Cell::new(dimensions),
            drawing_pos: Cell::new(Point::default()),
        }
    }

    pub fn drawing_points(&self) -> Ref<Vec<Point>> {
        Ref::map(self.drawing.borrow(), |x| &x.points)
    }

    fn drawing_points_mut(&self) -> RefMut<Vec<Point>> {
        RefMut::map(self.drawing.borrow_mut(), |x| &mut x.points)
    }

    pub fn render(&self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            self.img_vao.bind();
            gl::UseProgram(*self.img_prgm);
            self.update_dimension_uniforms();
            self.img_tex.bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.shape_vao.bind();
            gl::UseProgram(*self.shape_prgm);
            self.update_dimension_uniforms();
            let pos_loc = self
                .img_prgm
                .get_uniform_location(cstr!("drawing_pos"))
                .unwrap()
                .unwrap();
            let pos = self.drawing_pos.get();
            gl::Uniform2f(*pos_loc, pos.x, pos.y);
            self.shape_tex.bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::Uniform2f(*pos_loc, 0.0, 0.0);

            self.points_vao.bind();
            gl::UseProgram(*self.draw_prgm);
            self.update_dimension_uniforms();
            gl::DrawArrays(gl::POINTS, 0, self.drawing_points().len() as i32);
            gl::DrawArrays(gl::LINE_STRIP, 0, self.drawing_points().len() as i32);

            check_errors().unwrap();

            self.ctx.swap_buffers().unwrap();
        }
    }

    pub fn resize(&self) {
        let (w, h) = self.canvas.physical_size();
        self.update_dimensions(|dims| dims.screen_dims = [w as f32, h as f32].into());
        unsafe {
            gl::Viewport(0, 0, w as _, h as _);
        }
        self.ctx.resize(PhysicalSize::new(w, h));
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
        let prog = &self.img_prgm;

        let uniform = |name| prog.get_uniform_location(name).unwrap().unwrap();
        let screen_dims_loc = uniform(cstr!("screen_dims"));
        let scene_pos_loc = uniform(cstr!("scene_pos"));
        let scale_loc = uniform(cstr!("scale"));

        unsafe {
            gl::Uniform2f(*screen_dims_loc, dims.screen_dims.x, dims.screen_dims.y);
            gl::Uniform2f(*scene_pos_loc, dims.scene_pos.x, dims.scene_pos.y);
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
            self.img_tex.bind(TextureTarget::Rectangle);
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

            self.img_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, vertex_data, Usage::StaticDraw).unwrap();
        }
    }

    pub fn add_point(&self, point: Point) {
        self.drawing_points_mut().push(point);
        self.update_drawing();
    }

    pub fn update_point(&self, i: usize, point: Point) {
        self.drawing_points_mut()[i] = point;
        self.update_drawing();
    }

    pub fn pop_point(&self) -> Option<Point> {
        let mut points = self.drawing_points_mut();
        let p = points.pop()?;
        drop(points);
        self.update_drawing();
        Some(p)
    }

    pub fn clear_drawing(&self) {
        let mut drawing = self.drawing.borrow_mut();
        drawing.points.clear();
        drawing.pixels.clear();
        unsafe {
            self.points_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &drawing.points, Usage::StaticDraw).unwrap();

            let vertex_data = [0.0; 8];
            self.shape_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &vertex_data, Usage::StaticDraw).unwrap();

            self.shape_tex.bind(TextureTarget::Rectangle);
            gl::TexImage2D(
                gl::TEXTURE_RECTANGLE,
                0,
                gl::RGBA8 as _,
                0,
                0,
                0,
                gl::BGRA,
                gl::UNSIGNED_INT_8_8_8_8,
                drawing.pixels.as_ptr().cast(),
            );
        }
    }

    pub fn update_drawing(&self) {
        let drawing = self.drawing.borrow_mut();

        unsafe {
            self.points_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(
                BufferTarget::Array,
                drawing.points.as_slice(),
                Usage::StaticDraw,
            )
            .unwrap();
        }

        let points = &drawing.points;
        if points.len() < 3 {
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
        fn point_to_vector(p: Point) -> Vector {
            Vector {
                x: (p.x * 64.0) as i32,
                y: (p.y * 64.0) as i32,
            }
        }
        for ps in points.windows(2) {
            let v0 = point_to_vector(ps[0]);
            let v1 = point_to_vector(ps[1]);
            bbox.update_point(v0);
            bbox.update_point(v1);
            segments.push(Segment::LineSegment(v0, v1));
        }
        segments.push(Segment::LineSegment(
            point_to_vector(points[points.len() - 1]),
            point_to_vector(points[0]),
        ));

        let x0 = bbox.x_min as f32 / 64.0;
        let x1 = bbox.x_max as f32 / 64.0;
        let y0 = bbox.y_min as f32 / 64.0;
        let y1 = bbox.y_max as f32 / 64.0;
        let (width, height) = (x1 - x0, y1 - y0);

        // If I don't do this, then using GL_R8/GL_RED seems to break in a weird way.
        // I wish I knew why. Something about stride/alignment, maybe?
        let width = {
            if width % 4.0 == 0.0 {
                width
            } else {
                4.0 + (width - (width % 4.0))
            }
        };

        let (mut rasterizer, mut img_buf) =
            RefMut::map_split(drawing, |r| (&mut r.rasterizer, &mut r.pixels));
        rasterizer.reset(width as usize, height as usize);

        for segment in segments {
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
        rasterizer.for_each_pixel(|i, v| {
            let px = (v * 127.0) as u8;
            img_buf[i] = px;
        });

        self.drawing_pos.set([x0, y0].into());

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

            self.shape_tex.bind(TextureTarget::Rectangle);
            gl::TexImage2D(
                gl::TEXTURE_RECTANGLE,
                0,
                gl::R8 as _,
                width as _,
                height as _,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                img_buf.as_ptr().cast(),
            );

            self.shape_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, vertex_data, Usage::StaticDraw).unwrap();
        }
    }
}
