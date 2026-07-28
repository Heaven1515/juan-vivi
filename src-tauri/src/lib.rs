use std::sync::Mutex;
use tauri::State;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

struct ApiPort(Mutex<u16>);

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[tauri::command]
fn get_api_port(state: State<ApiPort>) -> u16 {
    *state.0.lock().unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let port = find_free_port();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(ApiPort(Mutex::new(port)))
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            let sidecar = app.shell()
                .sidecar("juan-vivi-backend")
                .unwrap()
                .args([&port.to_string()]);

            let (mut rx, _child) = sidecar.spawn().unwrap();

            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let CommandEvent::Stderr(line) = event {
                        eprintln!("[backend] {}", String::from_utf8_lossy(&line));
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar JUAN-VIVI");
}
