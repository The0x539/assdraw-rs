use native_windows_gui as nwg;

use nwg::NativeUi;

mod app;
//mod ass_outline;
//mod canvas;
mod drawing;
mod gl;
mod point;
mod undo;
//mod vk;

pub use crate::gl::abstraction;

fn main() {
    nwg::init().unwrap();
    let app = app::AppBuilder::build_ui(Default::default()).unwrap();
    nwg::dispatch_thread_events_with_callback(move || {
        if let Some(canvas) = app.canvas.get() {
            canvas.render();
        }
    });
}
