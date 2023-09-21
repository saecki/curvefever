use eframe::NativeOptions;

use app::CurvefeverApp;

pub mod app;
pub mod web;
pub mod world;

pub enum ServerEvent {
    Input { player_idx: u8, left_down: bool, right_down: bool },
}
pub enum GameEvent {}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    std::thread::scope(|scope| {
        // start web server
        let (server_send, server_recv) = std::sync::mpsc::channel();
        let (game_send, game_recv) = std::sync::mpsc::channel();
        let server_handle = scope.spawn(|| {
            web::start_server(&runtime, server_send, game_recv);
        });

        // start game
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
    });
}
