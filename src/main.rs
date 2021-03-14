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
mod nwg_util;

pub use crate::gl::abstraction;

fn main() {
    nwg::init().unwrap();
    let _app = app::AppBuilder::build_ui(Default::default()).unwrap();
    nwg::dispatch_thread_events();
}
