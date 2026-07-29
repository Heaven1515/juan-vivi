use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

pub const RUTA_SCANNER: &str = r"\\Desktop-14rq1mp\escaner riho 550";

const INTERVALO: Duration = Duration::from_secs(8);
const ESPERA_COPIA: Duration = Duration::from_secs(12);

/// Inicia el vigilador de la carpeta `carpeta` en un task de Tokio.
/// El `callback` se invoca por cada PDF nuevo detectado (tras la espera de 12s).
/// Retorna un `Arc<Notify>` — llamar `.notify_one()` para detener el vigilador.
pub fn iniciar(
    carpeta: String,
    callback: impl Fn(PathBuf) + Send + Sync + 'static,
) -> Arc<Notify> {
    let stop = Arc::new(Notify::new());
    let stop_rx = Arc::clone(&stop);
    let cb = Arc::new(callback);

    tokio::spawn(async move {
        let mut vistos: HashSet<PathBuf> = HashSet::new();

        // Poblar el set inicial con los PDFs ya presentes
        if let Ok(entradas) = std::fs::read_dir(&carpeta) {
            for entrada in entradas.flatten() {
                let path = entrada.path();
                if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
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
                    // Buscar PDFs nuevos
                    let nuevos: Vec<PathBuf> = match std::fs::read_dir(&carpeta) {
                        Ok(entradas) => entradas
                            .flatten()
                            .map(|e| e.path())
                            .filter(|p| {
                                p.extension().and_then(|e| e.to_str()) == Some("pdf")
                                    && !vistos.contains(p)
                            })
                            .collect(),
                        Err(_) => continue,
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
