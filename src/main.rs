use eframe::NativeOptions;

use app::CurvefeverApp;

mod app;
mod world;

fn main() {
    let options = NativeOptions {
        follow_system_theme: true,
        maximized: true,
        ..Default::default()
    };
    let res = eframe::run_native(
        "minesweeper",
        options,
        Box::new(|c| Box::new(CurvefeverApp::new(c))),
    );
    if let Err(e) = res {
        println!("error running app: {e}");
    }
}
