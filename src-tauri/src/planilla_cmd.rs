use std::sync::Mutex;
use regex::Regex;
use tauri::State;

use crate::modelos::{EstadoPlanilla, FilaResumen, ResultadoCargaPlanilla};
use crate::{escritor, lector, transformador};

// ─── Extrae número SP del nombre del archivo ──────────────────────────────────

pub fn extraer_numero_sp(rutas: &[String]) -> String {
    // Elimina fechas en varios formatos y extrae el primer número restante
    let re_fecha1 = Regex::new(r"\d{1,2}[-/]\d{1,2}[-/]\d{4}").unwrap();
    let re_fecha2 = Regex::new(r"\d{4}[-/]\d{1,2}[-/]\d{1,2}").unwrap();
    let re_num = Regex::new(r"\d+").unwrap();

    let mut numeros: Vec<String> = Vec::new();

    for ruta in rutas {
        let nombre = std::path::Path::new(ruta)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let sin_fecha = re_fecha1.replace_all(&nombre, "");
        let sin_fecha = re_fecha2.replace_all(&sin_fecha, "");

        if let Some(m) = re_num.find(&sin_fecha) {
            numeros.push(m.as_str().to_string());
        }
    }

    numeros.join("_")
}

// ─── Comandos Tauri ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn cargar_excel(
    rutas: Vec<String>,
    estado: State<'_, Mutex<EstadoPlanilla>>,
) -> ResultadoCargaPlanilla {
    if rutas.is_empty() {
        return ResultadoCargaPlanilla {
            ok: false,
            nombre: String::new(),
            filas: vec![],
            avisos: vec![],
            error: Some("No se proporcionaron archivos".to_string()),
        };
    }

    let nombre = std::path::Path::new(&rutas[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let (filas_entrada, errores_lectura) = lector::leer_multiples(&rutas);

    if !errores_lectura.is_empty() && filas_entrada.is_empty() {
        return ResultadoCargaPlanilla {
            ok: false,
            nombre,
            filas: vec![],
            avisos: vec![],
            error: Some(errores_lectura.join("; ")),
        };
    }

    let (filas_salida, avisos) = transformador::transformar_lote(&filas_entrada);
    let prefijo_sp = extraer_numero_sp(&rutas);

    // Crear resumen
    let resumen: Vec<FilaResumen> = filas_salida
        .iter()
        .map(|f| FilaResumen {
            n: f.c2_rut.clone(),
            nombre: f.c2_apellido_paterno.clone(),
            rut: f.c2_rut.clone(),
        })
        .collect();

    // Guardar en estado
    {
        let mut estado_lock = estado.lock().unwrap();
        estado_lock.filas = filas_salida;
        estado_lock.prefijo_sp = prefijo_sp;
    }

    let mut todos_avisos = errores_lectura;
    todos_avisos.extend(avisos);

    ResultadoCargaPlanilla {
        ok: true,
        nombre,
        filas: resumen,
        avisos: todos_avisos,
        error: None,
    }
}

#[tauri::command]
pub fn nombre_planilla(estado: State<'_, Mutex<EstadoPlanilla>>) -> String {
    let estado_lock = estado.lock().unwrap();
    escritor::nombre_planilla(&estado_lock.prefijo_sp)
}

#[tauri::command]
pub fn generar_planilla(
    ruta_destino: String,
    estado: State<'_, Mutex<EstadoPlanilla>>,
) -> serde_json::Value {
    let (filas, nombre_archivo) = {
        let estado_lock = estado.lock().unwrap();
        if estado_lock.filas.is_empty() {
            return serde_json::json!({ "ok": false, "error": "No hay datos cargados" });
        }
        let filas = estado_lock.filas.clone();
        let nombre = std::path::Path::new(&ruta_destino)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        (filas, nombre)
    };

    match escritor::escribir_planilla(&filas, &ruta_destino) {
        Ok(()) => serde_json::json!({ "ok": true, "nombre": nombre_archivo }),
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
    }
}
