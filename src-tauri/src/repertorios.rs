use std::collections::HashMap;
use calamine::{open_workbook_auto, Data, Reader};
use chrono::NaiveDate;
use serde::Serialize;
use tauri::command;

use crate::almacenamiento::ruta_archivo;
use crate::modelos::{Duplicado, Registro};

// ─── Persistencia ────────────────────────────────────────────────────────────

fn cargar() -> HashMap<String, Registro> {
    let ruta = ruta_archivo("repertorios.json");
    let contenido = std::fs::read_to_string(&ruta).unwrap_or_default();
    serde_json::from_str(&contenido).unwrap_or_default()
}

fn guardar(mapa: &HashMap<String, Registro>) -> anyhow::Result<()> {
    let ruta = ruta_archivo("repertorios.json");
    if let Some(padre) = ruta.parent() {
        std::fs::create_dir_all(padre)?;
    }
    let json = serde_json::to_string_pretty(mapa)?;
    std::fs::write(&ruta, json)?;
    Ok(())
}

// ─── Utilidades ──────────────────────────────────────────────────────────────

pub fn extraer_compareciente(texto: &str) -> String {
    texto
        .split(['\n', '\r'])
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter(|l| {
            let lc = l.to_lowercase();
            !lc.contains("autofin") && !lc.contains("global soluciones")
        })
        .next()
        .unwrap_or("")
        .to_string()
}

/// Convierte un serial de Excel (base 1899-12-30) a "YYYY-MM-DD"
pub fn serial_a_fecha(serial: f64) -> String {
    if serial < 1.0 {
        return String::new();
    }
    // Excel base date: 1899-12-30
    let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let dias = serial as i64;
    match base.checked_add_signed(chrono::Duration::days(dias)) {
        Some(fecha) => fecha.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

pub fn celda_como_str(v: &Data) -> String {
    match v {
        Data::Float(f) => format!("{}", *f as i64),
        Data::Int(i) => i.to_string(),
        Data::String(s) => s.trim().to_string(),
        Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

pub fn celda_como_fecha(v: &Data) -> String {
    match v {
        Data::String(s) => s.split_whitespace().next().unwrap_or("").to_string(),
        Data::Float(f) => serial_a_fecha(*f),
        Data::Int(i) => serial_a_fecha(*i as f64),
        _ => String::new(),
    }
}

// ─── Resultado ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ResultadoRepertorios {
    pub ok: bool,
    pub cargados: usize,
    pub duplicados: Vec<Duplicado>,
    pub errores: Vec<String>,
    pub error: Option<String>,
}

// ─── Comandos Tauri ──────────────────────────────────────────────────────────

#[command]
pub fn buscar_repertorio(numero: String) -> Option<Registro> {
    cargar().remove(&numero)
}

#[command]
pub fn reemplazar_repertorio(numero: String, datos: Registro) -> Result<(), String> {
    let mut mapa = cargar();
    mapa.insert(numero, datos);
    guardar(&mapa).map_err(|e| e.to_string())
}

#[command]
pub fn cargar_repertorios_excel(ruta: String) -> ResultadoRepertorios {
    match intentar_cargar_excel(&ruta) {
        Ok(r) => r,
        Err(e) => ResultadoRepertorios {
            ok: false,
            cargados: 0,
            duplicados: vec![],
            errores: vec![],
            error: Some(e.to_string()),
        },
    }
}

fn intentar_cargar_excel(ruta: &str) -> anyhow::Result<ResultadoRepertorios> {
    let mut wb = open_workbook_auto(ruta)?;

    // Buscar hoja "Exportar" (case-insensitive) o usar la primera
    let nombres = wb.sheet_names().to_vec();
    let nombre_hoja = nombres
        .iter()
        .find(|n| n.to_lowercase() == "exportar")
        .cloned()
        .or_else(|| nombres.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("El Excel no tiene hojas"))?;

    let rango = wb.worksheet_range(&nombre_hoja)?;

    let mut mapa = cargar();
    let mut cargados = 0usize;
    let mut duplicados: Vec<Duplicado> = vec![];
    let errores: Vec<String> = vec![];

    for fila in rango.rows().skip(1) {
        // col 2 = Repertorio; si vacía, saltar
        let repertorio = match fila.get(2) {
            Some(v) => {
                let s = celda_como_str(v);
                if s.is_empty() {
                    continue;
                }
                s
            }
            None => continue,
        };

        let ot = fila.get(0).map(celda_como_str).unwrap_or_default();
        let fecha = fila.get(3).map(celda_como_fecha).unwrap_or_default();
        let cliente = fila.get(5).map(celda_como_str).unwrap_or_default();
        let materia = fila.get(7).map(celda_como_str).unwrap_or_default();
        let comp_raw = fila.get(8).map(celda_como_str).unwrap_or_default();
        let compareciente = extraer_compareciente(&comp_raw);

        let entrante = Registro {
            repertorio: repertorio.clone(),
            ot,
            fecha,
            cliente,
            materia,
            compareciente,
        };

        if let Some(existente) = mapa.get(&repertorio) {
            duplicados.push(Duplicado {
                numero: repertorio.clone(),
                existente: existente.clone(),
                entrante,
            });
        } else {
            mapa.insert(repertorio, entrante);
            cargados += 1;
        }
    }

    guardar(&mapa)?;

    Ok(ResultadoRepertorios {
        ok: true,
        cargados,
        duplicados,
        errores,
        error: None,
    })
}
