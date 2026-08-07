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

/// Búsqueda flexible: primero exacta, luego comparando solo dígitos.
/// Maneja diferencias entre clave guardada ("3.090") y número OCR ("3090").
pub fn buscar_repertorio_flexible(numero: &str) -> Option<Registro> {
    let mapa = cargar();
    let clave = numero.trim();
    // 1. Exacta
    if let Some(r) = mapa.get(clave) {
        return Some(r.clone());
    }
    // 2. Comparar solo dígitos
    let norm: String = clave.chars().filter(|c| c.is_ascii_digit()).collect();
    mapa.into_values()
        .find(|r| r.repertorio.chars().filter(|ch| ch.is_ascii_digit()).collect::<String>() == norm)
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

/// Detecta si el rango corresponde al "MODELO PARA BASE DE DATOS":
/// sus encabezados reales están en fila 2 (índice 1) con "OT" en col 1 y "Repertorio" en col 2.
fn es_modelo_bd(rango: &calamine::Range<Data>) -> bool {
    if let Some(fila) = rango.rows().nth(1) {
        let col1 = fila.get(1).map(celda_como_str).unwrap_or_default();
        let col2 = fila.get(2).map(celda_como_str).unwrap_or_default();
        return col1.to_lowercase() == "ot" && col2.to_lowercase() == "repertorio";
    }
    false
}

fn intentar_cargar_excel(ruta: &str) -> anyhow::Result<ResultadoRepertorios> {
    let mut wb = open_workbook_auto(ruta)?;

    // Buscar hoja "Exportar" (case-insensitive) o "Resultado" o usar la primera
    let nombres = wb.sheet_names().to_vec();
    let nombre_hoja = nombres
        .iter()
        .find(|n| n.to_lowercase() == "exportar")
        .cloned()
        .or_else(|| nombres.iter().find(|n| n.to_lowercase() == "resultado").cloned())
        .or_else(|| nombres.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("El Excel no tiene hojas"))?;

    let rango = wb.worksheet_range(&nombre_hoja)?;

    let mut mapa = cargar();
    let mut cargados = 0usize;
    let mut duplicados: Vec<Duplicado> = vec![];
    let errores: Vec<String> = vec![];

    if es_modelo_bd(&rango) {
        // ── Formato MODELO PARA BASE DE DATOS ──────────────────────────────
        // Encabezados en fila 2 (índice 1), datos desde fila 3 (índice 2)
        // col 1=OT, col 2=Repertorio, col 3=Fecha, col 9=RUT C2,
        // col 10=Apellido Paterno C2, col 11=Apellido Materno C2, col 12=Nombre C2
        for fila in rango.rows().skip(2) {
            let repertorio = match fila.get(2) {
                Some(v) => {
                    let s = celda_como_str(v);
                    if s.is_empty() { continue; }
                    s
                }
                None => continue,
            };

            let ot = fila.get(1).map(celda_como_str).unwrap_or_default();
            let fecha = fila.get(3).map(celda_como_fecha).unwrap_or_default();
            let cliente = fila.get(9).map(celda_como_str).unwrap_or_default();
            let materia = String::new();
            // Compareciente 2: apellido paterno + apellido materno + nombre
            let ap = fila.get(10).map(celda_como_str).unwrap_or_default();
            let am = fila.get(11).map(celda_como_str).unwrap_or_default();
            let nm = fila.get(12).map(celda_como_str).unwrap_or_default();
            let compareciente = [ap, am, nm]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            let entrante = Registro { repertorio: repertorio.clone(), ot, fecha, cliente, materia, compareciente };

            if let Some(existente) = mapa.get(&repertorio) {
                duplicados.push(Duplicado { numero: repertorio.clone(), existente: existente.clone(), entrante });
            } else {
                mapa.insert(repertorio, entrante);
                cargados += 1;
            }
        }
    } else {
        // ── Formato AUTOFIN SP (original) ──────────────────────────────────
        // col 0=OT, col 2=Repertorio, col 3=Fecha, col 5=Cliente, col 7=Materia, col 8=Compareciente
        for fila in rango.rows().skip(1) {
            let repertorio = match fila.get(2) {
                Some(v) => {
                    let s = celda_como_str(v);
                    if s.is_empty() { continue; }
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

            let entrante = Registro { repertorio: repertorio.clone(), ot, fecha, cliente, materia, compareciente };

            if let Some(existente) = mapa.get(&repertorio) {
                duplicados.push(Duplicado { numero: repertorio.clone(), existente: existente.clone(), entrante });
            } else {
                mapa.insert(repertorio, entrante);
                cargados += 1;
            }
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
