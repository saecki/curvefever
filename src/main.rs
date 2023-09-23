use eframe::NativeOptions;

use app::CurvefeverApp;

pub mod app;
pub mod server;
pub mod world;

pub enum ServerEvent {
    Input {
        player_idx: u8,
        left_down: bool,
        right_down: bool,
    },
}
pub enum GameEvent {
    Exit,
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    std::thread::scope(|scope| {
        // start web server
        let (server_kill_signal, server_kill_receiver) = tokio::sync::oneshot::channel();
        let (server_sender, server_receiver) = crossbeam::channel::unbounded();
        let (game_sender, game_receiver) = crossbeam::channel::unbounded();
        let server_handle = scope.spawn(|| {
            server::start_server(&runtime, server_sender, game_receiver, server_kill_receiver);
        });

        // start game
        let options = NativeOptions {
            follow_system_theme: true,
            maximized: true,
            // fullscreen: true,
            ..Default::default()
        };

        let a_game_sender = game_sender.clone();
        let res = eframe::run_native(
            "curvefever",
            options,
            Box::new(|c| Box::new(CurvefeverApp::new(c, server_receiver, a_game_sender))),
        );
        if let Err(e) = res {
            println!("error running app: {e}");
        }

        // notify clients that the game is shutting down, and kill server
        game_sender.send(GameEvent::Exit).unwrap();
        server_kill_signal.send(()).unwrap();
        server_handle.join().unwrap();
    });
}
