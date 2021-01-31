use std::cell::Cell;

use native_windows_derive as nwd;
use native_windows_gui as nwg;

use nwd::NwgUi;

use crate::gl::OpenGlCanvas;

#[derive(Default, NwgUi)]
pub struct Canvas {
    #[nwg_control(size: (600, 500), position: (300, 300), title: "nwg/ogl", flags: "MAIN_WINDOW")]
    #[nwg_events(
        OnInit: [Canvas::show],
        OnWindowClose: [Canvas::exit],
        OnResize: [Canvas::resize_canvas],
        OnWindowMaximize: [Canvas::resize_canvas],
    )]
    pub window: nwg::Window,

    #[nwg_layout(parent: window, max_column: Some(4), max_row: Some(8))]
    pub grid: nwg::GridLayout,

    #[nwg_control(ty: nwg::ExternCanvas, parent: Some(&data.window))]
    #[nwg_events(
        OnMouseMove: [Canvas::mouse_move],
        OnMousePress: [Canvas::mouse_press(SELF, EVT)],
        OnMouseWheel: [Canvas::zoom(SELF, EVT_DATA)],
    )]
    #[nwg_layout_item(layout: grid, col: 0, row: 0, col_span: 3, row_span: 8)]
    pub canvas: OpenGlCanvas,

    #[nwg_resource]
    pub color_dialog: nwg::ColorDialog,

    #[nwg_control(parent: window, text: "bg")]
    #[nwg_events(OnButtonClick: [Canvas::select_bg_color])]
    #[nwg_layout_item(layout: grid, col: 3, row: 0)]
    pub choose_color_btn2: nwg::Button,

    #[nwg_control(parent: window, text: "tri")]
    #[nwg_events(OnButtonClick: [Canvas::select_tri_color])]
    #[nwg_layout_item(layout: grid, col: 3, row: 1)]
    pub choose_color_btn1: nwg::Button,

    #[nwg_control(parent: window, text: "paste image")]
    #[nwg_events(OnButtonClick: [Canvas::paste_image])]
    #[nwg_layout_item(layout: grid, col: 3, row: 2)]
    pub paste_image_btn: nwg::Button,

    dragging: Cell<bool>,
    drag_start: Cell<(i32, i32)>,
}

impl Canvas {
    fn cursor_pos(&self) -> (i32, i32) {
        nwg::GlobalCursor::local_position(&self.canvas, None)
    }

    pub fn mouse_move(&self) {
        if self.dragging.get() {
            let start = self.drag_start.get();
            let pos = self.cursor_pos();
            let delta = (pos.0 - start.0, pos.1 - start.1);
            self.canvas.set_delta(delta);
        }
    }

    pub fn mouse_press(&self, event: nwg::Event) {
        let ev = match event {
            nwg::Event::OnMousePress(ev) => ev,
            _ => return, // should be unreachable
        };
        match ev {
            nwg::MousePressEvent::MousePressRightDown => {
                nwg::GlobalCursor::set_capture(&self.canvas.handle);
                self.dragging.set(true);
                self.drag_start.set(self.cursor_pos());
            }
            nwg::MousePressEvent::MousePressRightUp => {
                nwg::GlobalCursor::release();
                self.dragging.set(false);
                self.canvas.commit_delta();
            }
            _ => (),
        }
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.set_focus();
    }

    pub fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    pub fn resize_canvas(&self) {
        self.canvas.resize();
    }

    pub fn select_bg_color(&self) {
        if self.color_dialog.run(Some(&self.window)) {
            let [r, g, b] = self.color_dialog.color();
            let [r, g, b] = [r as f32 / 225.0, g as f32 / 225.0, b as f32 / 225.0];

            unsafe {
                gl::ClearColor(r, g, b, 1.0);
            }
        }
        self.window.invalidate();
    }

    pub fn select_tri_color(&self) {
        if self.color_dialog.run(Some(&self.window)) {
            let [r, g, b] = self.color_dialog.color();
            let [r, g, b] = [r as f32 / 225.0, g as f32 / 225.0, b as f32 / 225.0];

            #[rustfmt::skip]
            let vertex_data: &[f32] = &[
                0.0,  1.0,   r, g, b,
               -1.0, -1.0,   r, g, b,
                1.0, -1.0,   r, g, b,
            ];
            let vertex_size = vertex_data.len() * std::mem::size_of::<f32>();

            unsafe {
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    vertex_size as _,
                    vertex_data.as_ptr().cast(),
                );
            }
        }
        self.window.invalidate();
    }

    pub fn paste_image(&self) {
        let buf = match clipboard_win::get_clipboard(clipboard_win::formats::Bitmap) {
            Ok(buf) => buf,
            e => {
                println!("{:?}", e);
                return;
            }
        };
        let cursor = std::io::Cursor::new(&buf[..]);
        let img = image::codecs::bmp::BmpDecoder::new(cursor).unwrap();
        self.canvas.set_image(img);
    }

    pub fn zoom(&self, data: &nwg::EventData) {
        let factor = match data {
            nwg::EventData::OnMouseWheel(i) => 1.25_f32.powf(*i as f32 / 120.0),
            _ => panic!(),
        };
        self.canvas.scale_by(factor);
    }
}
