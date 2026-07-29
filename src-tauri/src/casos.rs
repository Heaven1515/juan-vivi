use std::collections::HashMap;

use crate::almacenamiento::ruta_archivo;
use crate::modelos::Caso;

// ─── Persistencia ────────────────────────────────────────────────────────────

fn cargar() -> HashMap<String, Caso> {
    let ruta = ruta_archivo("casos.json");
    let contenido = std::fs::read_to_string(&ruta).unwrap_or_default();
    serde_json::from_str(&contenido).unwrap_or_default()
}

fn guardar(mapa: &HashMap<String, Caso>) -> anyhow::Result<()> {
    let ruta = ruta_archivo("casos.json");
    if let Some(padre) = ruta.parent() {
        std::fs::create_dir_all(padre)?;
    }
    let json = serde_json::to_string_pretty(mapa)?;
    std::fs::write(&ruta, json)?;
    Ok(())
}

// ─── Comandos Tauri ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn buscar_caso(numero: String) -> Option<Caso> {
    cargar().remove(&numero)
}

#[tauri::command]
pub fn agregar_caso(caso: Caso) -> Result<(), String> {
    let mut mapa = cargar();
    mapa.insert(caso.numero.clone(), caso);
    guardar(&mapa).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_casos() -> Vec<Caso> {
    let mapa = cargar();
    let mut lista: Vec<Caso> = mapa.into_values().collect();
    lista.sort_by_key(|c| c.numero.parse::<u64>().unwrap_or(0));
    lista
}

/// Búsqueda interna (no es comando Tauri) usada por firma.rs
pub fn buscar_caso_interno(numero: &str) -> Option<Caso> {
    cargar().get(numero.trim()).cloned()
}

#[tauri::command]
pub fn eliminar_caso(numero: String) -> bool {
    let mut mapa = cargar();
    if mapa.remove(&numero).is_some() {
        guardar(&mapa).is_ok()
    } else {
        false
    }
}
