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

use crate::point::Point;
use crate::undo::UndoStack;

use std::cell::{Cell, RefCell, RefMut};
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
    pub screen_dims: Point<f32>,
    pub scene_pos: Point<f32>,
    pub scale: GLfloat,
}

use crate::drawing::{Drawing, Segment};

pub struct OpenGlCanvas {
    ctx: Ctx,
    canvas: nwg::ExternCanvas,

    img_prgm: Program,
    draw_prgm: Program,
    shape_prgm: Program,

    img_vb: Buffer,
    points_vb: Buffer,
    lines_vb: Buffer,
    shape_vb: Buffer,

    img_vao: VertexArray,
    points_vao: VertexArray,
    lines_vao: VertexArray,
    shape_vao: VertexArray,

    img_tex: Texture,
    shape_tex: Texture,

    drawing: RefCell<DrawingData>,

    dimensions: Cell<Dimensions>,
    drawing_pos: Cell<Point<f32>>,

    drawing_color: Cell<[u8; 3]>,
    shape_color: Cell<[u8; 3]>,
    shape_alpha: Cell<u8>,
}

struct DrawingData {
    pixels: Vec<u8>,
    drawing: UndoStack<Drawing<Point<f32>>>,
    n_lines: usize,
    rasterizer: Rasterizer,
}

impl Default for DrawingData {
    fn default() -> Self {
        Self {
            pixels: Vec::new(),
            drawing: UndoStack::new(Drawing::new()),
            rasterizer: Rasterizer::new(0, 0),
            n_lines: 0,
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

        let (lines_vb, lines_vao) = unsafe {
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
            lines_vb,
            shape_vb,

            img_vao,
            points_vao,
            lines_vao,
            shape_vao,

            img_tex,
            shape_tex,

            drawing,

            dimensions: Cell::new(dimensions),
            drawing_pos: Cell::new(Point::default()),

            drawing_color: Cell::new([0, 0, 255]),
            shape_color: Cell::new([127, 127, 127]),
            shape_alpha: Cell::new(50),
        }
    }

    pub fn render(&self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            let uniform = |prog: &Program, name| prog.get_uniform_location(name).unwrap().unwrap();

            self.img_vao.bind();
            gl::UseProgram(*self.img_prgm);
            self.update_dimension_uniforms(&self.img_prgm);
            self.img_tex.bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.shape_vao.bind();
            gl::UseProgram(*self.shape_prgm);

            self.update_dimension_uniforms(&self.img_prgm);

            let pos_loc = uniform(&self.shape_prgm, cstr!("drawing_pos"));
            let pos = self.drawing_pos.get();
            gl::Uniform2f(*pos_loc, pos.x, pos.y);

            {
                let color_loc = uniform(&self.shape_prgm, cstr!("u_Color"));
                let [r, g, b] = self.shape_color.get();
                gl::Uniform3ui(*color_loc, r as _, g as _, b as _);
            }

            {
                let alpha_loc = uniform(&self.shape_prgm, cstr!("u_Alpha"));
                gl::Uniform1ui(*alpha_loc, self.shape_alpha.get() as _);
            }

            self.shape_tex.bind(TextureTarget::Rectangle);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::Uniform2f(*pos_loc, 0.0, 0.0);

            self.points_vao.bind();
            gl::UseProgram(*self.draw_prgm);

            self.update_dimension_uniforms(&self.draw_prgm);

            {
                let color_loc = uniform(&self.draw_prgm, cstr!("u_Color"));
                let [r, g, b] = self.drawing_color.get();
                gl::Uniform3ui(*color_loc, r as _, g as _, b as _);
            }

            let n_points = self.drawing.borrow().drawing.points().len() as i32;
            gl::DrawArrays(gl::POINTS, 0, n_points);

            self.lines_vao.bind();
            let n_lines = self.drawing.borrow().n_lines as i32;
            gl::DrawArrays(gl::LINES, 0, n_lines * 4);

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

    fn update_dimension_uniforms(&self, prog: &Program) {
        let dims = self.get_dimensions();

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

    pub fn with_drawing<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut UndoStack<Drawing<Point<f32>>>) -> T,
    {
        let mut drawing_data = self.drawing.borrow_mut();
        let ret = f(&mut drawing_data.drawing);
        drop(drawing_data);
        self.update_drawing();
        ret
    }

    pub fn commit_drawing(&self) {
        self.drawing.borrow_mut().drawing.commit();
    }

    pub fn undo(&self) {
        self.with_drawing(UndoStack::undo);
    }

    pub fn redo(&self) {
        self.with_drawing(UndoStack::redo);
    }

    pub fn clear_drawing(&self) {
        let mut drawing = self.drawing.borrow_mut();
        drawing.drawing.clear();
        unsafe {
            self.points_vb.bind(BufferTarget::Array);
            let points = drawing.drawing.points();
            Buffer::buffer_data(BufferTarget::Array, points, Usage::StaticDraw).unwrap();

            drawing.n_lines = 0;

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
        let mut data = self.drawing.borrow_mut();

        unsafe {
            self.points_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(
                BufferTarget::Array,
                data.drawing.points(),
                Usage::StaticDraw,
            )
            .unwrap();
        }

        let (mut x_min, mut y_min, mut x_max, mut y_max) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let mut segments = vec![];
        let mut line_data = vec![];
        for seg in data.drawing.segments() {
            for pt in seg.points() {
                x_min = x_min.min(pt.x);
                y_min = y_min.min(pt.y);
                x_max = x_max.max(pt.x);
                y_max = y_max.max(pt.y);
            }
            segments.push(seg);

            match seg {
                Segment::Line(p0, p1) => {
                    line_data.push((p0, p1));
                }
                // Don't draw a line for a shape's closing line.
                Segment::ClosingLine(..) => (),
                Segment::Bezier(p0, p1, p2, p3) => {
                    line_data.push((p0, p1));
                    line_data.push((p2, p3));
                }
            }
        }

        if segments.is_empty() {
            return;
        }

        data.n_lines = line_data.len();
        unsafe {
            self.lines_vb.bind(BufferTarget::Array);
            Buffer::buffer_data(BufferTarget::Array, &line_data, Usage::StaticDraw).unwrap();
        }

        assert_ne!(x_min, f32::MAX);
        assert_ne!(y_min, f32::MAX);
        let (width, height) = (x_max - x_min, y_max - y_min);
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let top_left = Point::new(x_min, y_min);

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
            RefMut::map_split(data, |r| (&mut r.rasterizer, &mut r.pixels));
        rasterizer.reset(width as usize, height as usize);

        let cnv = |p| ab_glyph_rasterizer::Point::from(p - top_left);
        for segment in segments {
            match segment {
                Segment::Line(p0, p1) | Segment::ClosingLine(p0, p1) => {
                    rasterizer.draw_line(cnv(p0), cnv(p1));
                }
                Segment::Bezier(p0, p1, p2, p3) => {
                    rasterizer.draw_cubic(cnv(p0), cnv(p1), cnv(p2), cnv(p3))
                }
            }
        }

        img_buf.clear();
        let buf_size = width as usize * height as usize;
        img_buf.reserve(buf_size);
        rasterizer.for_each_pixel(|i, v| {
            debug_assert_eq!(i, img_buf.len());
            let px = (v * 512.0) as u8;
            img_buf.push(px);
        });
        assert_eq!(img_buf.len(), buf_size);

        self.drawing_pos.set(Point::new(x_min, y_min));

        unsafe {
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

    pub fn recolor_drawing(&self, rgb: [u8; 3]) {
        self.drawing_color.set(rgb);
    }

    pub fn recolor_shape(&self, rgb: [u8; 3]) {
        self.shape_color.set(rgb);
    }

    pub fn set_shape_alpha(&self, alpha: u8) {
        self.shape_alpha.set(alpha);
    }
}
