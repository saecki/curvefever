use eframe::NativeOptions;

use app::CurvefeverApp;

pub mod app;
pub mod world;

fn main() {
    let options = NativeOptions {
        follow_system_theme: true,
        maximized: true,
        // fullscreen: true,
        ..Default::default()
    };
    let res = eframe::run_native(
        "curvefever",
        options,
        Box::new(|c| Box::new(CurvefeverApp::new(c))),
    );
    if let Err(e) = res {
        println!("error running app: {e}");
    }
}
