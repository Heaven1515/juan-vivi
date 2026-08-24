use std::collections::HashSet;
use std::path::PathBuf;

pub fn dir_datos() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from("data")
    }
    #[cfg(not(debug_assertions))]
    {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("JUAN-VIVI")
    }
}

pub fn ruta_archivo(nombre: &str) -> PathBuf {
    dir_datos().join(nombre)
}

// ─── Registro persistente de PDFs enviados ────────────────────────────────────

const ENVIADOS_FILE: &str = "enviados.json";

/// Carga la lista de nombres de PDF ya enviados desde disco.
pub fn cargar_enviados() -> HashSet<String> {
    let ruta = ruta_archivo(ENVIADOS_FILE);
    let texto = std::fs::read_to_string(&ruta).unwrap_or_default();
    serde_json::from_str::<Vec<String>>(&texto)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// Marca un PDF como enviado — lo agrega al archivo en disco.
pub fn marcar_enviado(nombre: &str) {
    let ruta = ruta_archivo(ENVIADOS_FILE);
    if let Some(dir) = ruta.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut set = cargar_enviados();
    set.insert(nombre.to_string());
    let lista: Vec<&String> = set.iter().collect();
    if let Ok(json) = serde_json::to_string(&lista) {
        let _ = std::fs::write(&ruta, json);
    }
}
