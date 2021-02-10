use native_windows_gui as nwg;

use nwg::NativeUi;

pub mod ass;
mod canvas;
pub mod drawing;
mod gl;

pub use crate::gl::abstraction;

fn main() {
    nwg::init().expect("Failed to init NWG");
    use canvas::Canvas;
    let app = Canvas::build_ui(Canvas::default()).expect("Failed to build UI");
    app.canvas.create_context();
    app.canvas.render();

    nwg::dispatch_thread_events_with_callback(move || {
        app.canvas.render();
    });
}
