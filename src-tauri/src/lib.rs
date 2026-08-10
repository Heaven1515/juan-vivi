use std::sync::{Arc, Mutex};
use tauri::Manager;

mod almacenamiento;
mod modelos;
mod casos;
mod licencia;
mod repertorios;
mod lector;
mod transformador;
mod escritor;
mod planilla_cmd;
mod signplus;
mod vigilador;
mod ocr;
mod firma;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                let win: tauri::WebviewWindow = win;
                win.open_devtools();
            }
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(modelos::EstadoPlanilla::default()))
        .manage(Arc::new(Mutex::new(firma::EstadoFirma::default())))
        .invoke_handler(tauri::generate_handler![
            // Licencia
            licencia::verificar_licencia,
            // Repertorios
            repertorios::buscar_repertorio,
            repertorios::reemplazar_repertorio,
            repertorios::cargar_repertorios_excel,
            repertorios::vaciar_repertorios,
            // Planilla
            planilla_cmd::cargar_excel,
            planilla_cmd::nombre_planilla,
            planilla_cmd::generar_planilla,
            // Firma
            firma::get_carpeta_scanner,
            firma::set_carpeta_scanner,
            firma::listar_pdfs,
            firma::pdf_pagina,
            firma::enviar_pdf,
            firma::toggle_auto,
            firma::estado_auto,
            firma::get_log_firma,
            firma::buscar_datos_firma,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar JUAN-VIVI");
}
