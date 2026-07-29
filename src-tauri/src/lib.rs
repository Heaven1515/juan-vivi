// Prevents additional console window on Windows in release
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Listener, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

struct ApiPort(Mutex<u16>);
struct Sidecar(Mutex<Option<CommandChild>>);
struct ShuttingDown(AtomicBool);

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[tauri::command]
fn get_api_port(state: State<ApiPort>) -> u16 {
    *state.0.lock().unwrap()
}

/// Lanza el proceso backend Python y se queda escuchando su canal.
/// Si el proceso muere de forma inesperada, lo relanza automáticamente
/// tras 2 segundos. Solo se detiene cuando `ShuttingDown` está activo
/// (es decir, cuando el usuario cierra la app a propósito).
fn spawn_backend(app: AppHandle, port: u16) {
    let sidecar_cmd = app
        .shell()
        .sidecar("juan-vivi-backend")
        .unwrap()
        .args([port.to_string()]);

    match sidecar_cmd.spawn() {
        Ok((mut rx, child)) => {
            *app.state::<Sidecar>().0.lock().unwrap() = Some(child);

            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                // Escuchar stderr del sidecar
                while let Some(event) = rx.recv().await {
                    if let CommandEvent::Stderr(line) = event {
                        eprintln!("[backend] {}", String::from_utf8_lossy(&line));
                    }
                }
                // El canal se cerró → proceso terminó
                let shutting_down = app_clone.state::<ShuttingDown>();
                if !shutting_down.0.load(Ordering::SeqCst) {
                    eprintln!("[backend] proceso caído — reiniciando en 2s...");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if !shutting_down.0.load(Ordering::SeqCst) {
                        spawn_backend(app_clone, port);
                    }
                }
            });
        }
        Err(e) => {
            eprintln!("[backend] fallo al iniciar: {} — reintentando en 3s...", e);
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let shutting_down = app_clone.state::<ShuttingDown>();
                if !shutting_down.0.load(Ordering::SeqCst) {
                    spawn_backend(app_clone, port);
                }
            });
        }
    }
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
        .manage(Sidecar(Mutex::new(None)))
        .manage(ShuttingDown(AtomicBool::new(false)))
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            spawn_backend(app.handle().clone(), port);

            // Al cerrar la app: marcar shutdown intencional antes de matar el proceso
            let app_handle = app.handle().clone();
            app.listen("tauri://destroy", move |_| {
                app_handle
                    .state::<ShuttingDown>()
                    .0
                    .store(true, Ordering::SeqCst);
                let sidecar_state = app_handle.state::<Sidecar>();
                let mut guard = sidecar_state.0.lock().unwrap();
                if let Some(child) = guard.take() {
                    let _ = child.kill();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar JUAN-VIVI");
}
