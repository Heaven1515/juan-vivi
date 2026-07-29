use calamine::{open_workbook_auto, Data, Reader};

use crate::modelos::FilaEntrada;

// ─── Índices de columna ──────────────────────────────────────────────────────
const COL_NUM: usize = 0;
const COL_NOMBRE: usize = 1;
const COL_RUT: usize = 2;
const COL_NOMBRE_PARA: usize = 3;
const COL_RUT_PARA: usize = 4;
const COL_OPERACION: usize = 5;
const COL_PATENTE: usize = 6;
const COL_TIPO: usize = 10;
const MIN_COLS: usize = 11;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn str_o_none(v: &Data) -> Option<String> {
    match v {
        Data::String(s) => {
            let t = s.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        }
        Data::Float(f) => Some(format!("{}", *f as i64)),
        Data::Int(i) => Some(i.to_string()),
        Data::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn str_req(v: &Data) -> String {
    str_o_none(v).unwrap_or_default()
}

fn es_fila_vacia(fila: &[Data]) -> bool {
    fila.iter().all(|c| matches!(c, Data::Empty))
}

// ─── Lectura ─────────────────────────────────────────────────────────────────

pub fn leer_archivo(ruta: &str) -> anyhow::Result<Vec<FilaEntrada>> {
    let mut wb = open_workbook_auto(ruta)?;
    let rango = wb
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow::anyhow!("El archivo no tiene hojas"))??;

    if rango.width() < MIN_COLS {
        anyhow::bail!(
            "El archivo tiene {} columnas; se requieren al menos {}",
            rango.width(),
            MIN_COLS
        );
    }

    let mut filas: Vec<FilaEntrada> = Vec::new();

    for (idx, fila) in rango.rows().enumerate().skip(1) {
        if es_fila_vacia(fila) {
            continue;
        }

        let numero = match fila.get(COL_NUM) {
            Some(Data::Float(f)) => *f as i64,
            Some(Data::Int(i)) => *i,
            _ => 0,
        };

        let nombre = fila.get(COL_NOMBRE).map(str_req).unwrap_or_default();
        let rut = fila.get(COL_RUT).map(str_req).unwrap_or_default();
        let nombre_para = fila.get(COL_NOMBRE_PARA).and_then(str_o_none);
        let rut_para = fila.get(COL_RUT_PARA).map(str_req).unwrap_or_default();
        let operacion = fila.get(COL_OPERACION).map(str_req).unwrap_or_default();
        let patente = fila.get(COL_PATENTE).map(str_req).unwrap_or_default();
        let tipo = fila.get(COL_TIPO).map(str_req).unwrap_or_default();

        filas.push(FilaEntrada {
            numero,
            nombre,
            rut,
            nombre_para,
            rut_para,
            operacion,
            patente,
            tipo,
            num_fila_excel: idx + 1, // Excel es 1-based, idx es 0-based
        });
    }

    Ok(filas)
}

pub fn leer_multiples(rutas: &[String]) -> (Vec<FilaEntrada>, Vec<String>) {
    let mut todas: Vec<FilaEntrada> = Vec::new();
    let mut errores: Vec<String> = Vec::new();

    for ruta in rutas {
        match leer_archivo(ruta) {
            Ok(filas) => todas.extend(filas),
            Err(e) => errores.push(format!("{}: {}", ruta, e)),
        }
    }

    (todas, errores)
}
