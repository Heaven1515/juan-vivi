use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

pub const RUTA_SCANNER: &str = r"\\Desktop-14rq1mp\escaner riho 550";

const INTERVALO: Duration = Duration::from_secs(8);
const ESPERA_COPIA: Duration = Duration::from_secs(12);

/// Inicia el vigilador de la carpeta `carpeta` en un task de Tokio.
/// `callback` se invoca por cada PDF nuevo detectado (tras la espera de 12s).
/// `on_error` se invoca cuando la carpeta no es accesible (para loguear en UI).
/// Retorna un `Arc<Notify>` — llamar `.notify_one()` para detener el vigilador.
pub fn iniciar(
    carpeta: String,
    callback: impl Fn(PathBuf) + Send + Sync + 'static,
    on_error: impl Fn(String) + Send + Sync + 'static,
) -> Arc<Notify> {
    let stop = Arc::new(Notify::new());
    let stop_rx = Arc::clone(&stop);
    let cb = Arc::new(callback);
    let on_err = Arc::new(on_error);

    tokio::spawn(async move {
        let mut vistos: HashSet<PathBuf> = HashSet::new();
        let mut ultimo_error: Option<String> = None;

        // Poblar el set inicial con los PDFs ya presentes
        if let Ok(entradas) = std::fs::read_dir(&carpeta) {
            for entrada in entradas.flatten() {
                let path = entrada.path();
                if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false) {
                    vistos.insert(path);
                }
            }
        }

        loop {
            tokio::select! {
                _ = stop_rx.notified() => {
                    break;
                }
                _ = sleep(INTERVALO) => {
                    let nuevos: Vec<PathBuf> = match std::fs::read_dir(&carpeta) {
                        Ok(entradas) => {
                            // Carpeta accesible — limpiar error previo
                            ultimo_error = None;
                            entradas
                                .flatten()
                                .map(|e| e.path())
                                .filter(|p| {
                                    p.extension().and_then(|e| e.to_str())
                                        .map(|e| e.eq_ignore_ascii_case("pdf"))
                                        .unwrap_or(false)
                                        && !vistos.contains(p)
                                })
                                .collect()
                        }
                        Err(e) => {
                            // Solo loguear si el error cambió (evitar spam)
                            let msg = format!("No se puede leer la carpeta '{}': {}", carpeta, e);
                            if ultimo_error.as_deref() != Some(&msg) {
                                on_err(msg.clone());
                                ultimo_error = Some(msg);
                            }
                            continue;
                        }
                    };

                    for nuevo in nuevos {
                        vistos.insert(nuevo.clone());
                        let cb_clone = Arc::clone(&cb);
                        tokio::spawn(async move {
                            sleep(ESPERA_COPIA).await;
                            cb_clone(nuevo);
                        });
                    }
                }
            }
        }
    });

    stop
}
