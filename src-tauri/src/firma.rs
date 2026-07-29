use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::modelos::Caso;
use crate::{ocr, vigilador};

// ─── Structs públicos ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EntradaLog {
    pub nombre: String,
    pub repertorio: String,
    pub tipo: String,
    pub estado: String,  // "ok" | "error" | "sin_datos" | "procesando"
    pub mensaje: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosEnvio {
    pub nombre: String,
    pub numero: String,
    pub anho: String,
    pub tipo_contrato: String,
    pub fecha_dia: u32,
    pub fecha_mes: u32,
    pub fecha_anio: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfoPdf {
    pub nombre: String,
    pub ruta: String,
    pub ya_enviado: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginaResult {
    pub ok: bool,
    pub data: Option<String>,
    pub pagina: u32,
    pub total: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EstadoAutoResult {
    pub activo: bool,
    pub subidos: usize,
    pub errores: usize,
}

// ─── Estado ──────────────────────────────────────────────────────────────────

pub struct EstadoFirma {
    pub carpeta: String,
    pub log: Vec<EntradaLog>,
    pub auto_stop: Option<Arc<tokio::sync::Notify>>,
}

impl Default for EstadoFirma {
    fn default() -> Self {
        Self {
            carpeta: vigilador::RUTA_SCANNER.to_string(),
            log: Vec::new(),
            auto_stop: None,
        }
    }
}

// ─── Funciones privadas ───────────────────────────────────────────────────────

fn _buscar_datos(numero: &str) -> Option<Caso> {
    // Normalizar número OCR antes de buscar (quita O→0, l→1, puntos, espacios)
    let num_norm = crate::ocr::normalizar_numero(numero);
    let buscar = if num_norm.is_empty() { numero } else { &num_norm };

    // 1. Buscar en casos.json (flexible: exacto o solo-dígitos)
    if let Some(caso) = crate::casos::buscar_caso_flexible(buscar) {
        return Some(caso);
    }

    // 2. Buscar en repertorios.json y convertir
    let reg = crate::repertorios::buscar_repertorio_flexible(buscar)?;

    // Convertir fecha "YYYY-MM-DD"
    let partes: Vec<&str> = reg.fecha.splitn(3, '-').collect();
    if partes.len() != 3 {
        return None;
    }
    let anio_str = partes[0];
    let mes_str = partes[1];
    let dia_str = partes[2];

    let anio: u32 = anio_str.parse().ok()?;
    let mes: u32 = mes_str.parse().ok()?;
    let dia: u32 = dia_str.parse().ok()?;

    if anio == 0 || mes == 0 || dia == 0 {
        return None;
    }

    Some(Caso {
        numero: reg.repertorio,
        anho: anio_str.to_string(),
        tipo_contrato: reg.materia,
        fecha_dia: dia,
        fecha_mes: mes,
        fecha_anio: anio,
    })
}

async fn enviar_pdf_interno(
    carpeta: &str,
    datos: &DatosEnvio,
) -> Result<EntradaLog, String> {
    let ruta = PathBuf::from(carpeta).join(&datos.nombre);

    match crate::signplus::enviar_formulario(
        &ruta,
        &datos.numero,
        &datos.anho,
        &datos.tipo_contrato,
        datos.fecha_dia,
        datos.fecha_mes,
        datos.fecha_anio,
    )
    .await
    {
        Ok(()) => Ok(EntradaLog {
            nombre: datos.nombre.clone(),
            repertorio: format!("{}-{}", datos.numero, datos.anho),
            tipo: datos.tipo_contrato.clone(),
            estado: "ok".to_string(),
            mensaje: "Enviado correctamente".to_string(),
        }),
        Err(e) => Ok(EntradaLog {
            nombre: datos.nombre.clone(),
            repertorio: format!("{}-{}", datos.numero, datos.anho),
            tipo: datos.tipo_contrato.clone(),
            estado: "error".to_string(),
            mensaje: e.to_string(),
        }),
    }
}

pub async fn _callback_auto(
    ruta_pdf: PathBuf,
    nombre: String,
    resource_dir: PathBuf,
    estado: Arc<Mutex<EstadoFirma>>,
) {
    // 1. OCR
    let ruta_clone = ruta_pdf.clone();
    let res_dir_clone = resource_dir.clone();
    let datos_ocr = tokio::task::spawn_blocking(move || -> anyhow::Result<ocr::DatosOcr> {
        let pdfium = ocr::crear_pdfium(&res_dir_clone)?;
        ocr::extraer_datos(&ruta_clone, &res_dir_clone.join("tesseract"), &pdfium)
    })
    .await;

    let datos_ocr = match datos_ocr {
        Ok(Ok(d)) => d,
        Ok(Err(e)) => {
            let mut g = estado.lock().unwrap();
            g.log.push(EntradaLog {
                nombre: nombre.clone(),
                repertorio: "—".to_string(),
                tipo: "—".to_string(),
                estado: "error".to_string(),
                mensaje: format!("OCR falló: {}", e),
            });
            return;
        }
        Err(e) => {
            let mut g = estado.lock().unwrap();
            g.log.push(EntradaLog {
                nombre: nombre.clone(),
                repertorio: "—".to_string(),
                tipo: "—".to_string(),
                estado: "error".to_string(),
                mensaje: format!("Error interno OCR: {}", e),
            });
            return;
        }
    };

    // 2. Verificar que tengamos número
    let numero = match datos_ocr.numero {
        Some(ref n) => n.clone(),
        None => {
            let mut g = estado.lock().unwrap();
            g.log.push(EntradaLog {
                nombre: nombre.clone(),
                repertorio: "—".to_string(),
                tipo: "—".to_string(),
                estado: "error".to_string(),
                mensaje: "OCR: repertorio no detectado".to_string(),
            });
            return;
        }
    };

    let anho = datos_ocr.anho.clone().unwrap_or_default();

    // 3. Buscar datos en BD
    let caso = match _buscar_datos(&numero) {
        Some(c) => c,
        None => {
            let mut g = estado.lock().unwrap();
            g.log.push(EntradaLog {
                nombre: nombre.clone(),
                repertorio: format!("{}-{}", numero, anho),
                tipo: datos_ocr.tipo_contrato.clone().unwrap_or_default(),
                estado: "sin_datos".to_string(),
                mensaje: format!(
                    "Repertorio {}-{} no está en la base de datos — procesar manualmente",
                    numero, anho
                ),
            });
            return;
        }
    };

    // 4. Enviar
    let carpeta = {
        let g = estado.lock().unwrap();
        g.carpeta.clone()
    };

    let datos_envio = DatosEnvio {
        nombre: nombre.clone(),
        numero: caso.numero.clone(),
        anho: caso.anho.clone(),
        tipo_contrato: caso.tipo_contrato.clone(),
        fecha_dia: caso.fecha_dia,
        fecha_mes: caso.fecha_mes,
        fecha_anio: caso.fecha_anio,
    };

    let entrada = match enviar_pdf_interno(&carpeta, &datos_envio).await {
        Ok(e) => e,
        Err(e) => EntradaLog {
            nombre: nombre.clone(),
            repertorio: format!("{}-{}", caso.numero, caso.anho),
            tipo: caso.tipo_contrato.clone(),
            estado: "error".to_string(),
            mensaje: e,
        },
    };

    let mut g = estado.lock().unwrap();
    g.log.push(entrada);
}

// ─── Comandos Tauri ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_carpeta_scanner(state: State<'_, Arc<Mutex<EstadoFirma>>>) -> String {
    state.lock().unwrap().carpeta.clone()
}

#[tauri::command]
pub fn set_carpeta_scanner(ruta: String, state: State<'_, Arc<Mutex<EstadoFirma>>>) {
    state.lock().unwrap().carpeta = ruta;
}

#[tauri::command]
pub fn listar_pdfs(state: State<'_, Arc<Mutex<EstadoFirma>>>) -> Vec<InfoPdf> {
    let g = state.lock().unwrap();
    let carpeta = g.carpeta.clone();
    let log = g.log.clone();
    drop(g);

    let ya_enviados: std::collections::HashSet<String> = log
        .iter()
        .filter(|e| e.estado == "ok")
        .map(|e| e.nombre.clone())
        .collect();

    match std::fs::read_dir(&carpeta) {
        Ok(entradas) => {
            let mut lista: Vec<InfoPdf> = entradas
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.eq_ignore_ascii_case("pdf"))
                        .unwrap_or(false)
                })
                .map(|e| {
                    let nombre = e.file_name().to_string_lossy().to_string();
                    let ya_enviado = ya_enviados.contains(&nombre);
                    InfoPdf {
                        nombre: nombre.clone(),
                        ruta: carpeta.clone(),
                        ya_enviado,
                    }
                })
                .collect();
            lista.sort_by(|a, b| a.nombre.cmp(&b.nombre));
            lista
        }
        Err(_) => vec![],
    }
}

#[tauri::command]
pub async fn pdf_pagina(
    nombre: String,
    pagina: u32,
    state: State<'_, Arc<Mutex<EstadoFirma>>>,
    app: AppHandle,
) -> Result<PaginaResult, String> {
    let carpeta = state.lock().unwrap().carpeta.clone();
    let ruta_pdf = PathBuf::from(&carpeta).join(&nombre);

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;

    let resultado = tokio::task::spawn_blocking(move || {
        let pdfium = ocr::crear_pdfium(&resource_dir).map_err(|e| e.to_string())?;
        ocr::renderizar_pagina(&ruta_pdf, pagina as u16, 1.5f32, &pdfium)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?;

    match resultado {
        Ok((b64, total)) => Ok(PaginaResult {
            ok: true,
            data: Some(b64),
            pagina,
            total: total as u32,
            error: None,
        }),
        Err(e) => Ok(PaginaResult {
            ok: false,
            data: None,
            pagina,
            total: 0,
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn enviar_pdf(
    datos: DatosEnvio,
    state: State<'_, Arc<Mutex<EstadoFirma>>>,
) -> Result<EntradaLog, String> {
    let carpeta = state.lock().unwrap().carpeta.clone();
    let entrada = enviar_pdf_interno(&carpeta, &datos).await?;
    state.lock().unwrap().log.push(entrada.clone());
    Ok(entrada)
}

#[tauri::command]
pub async fn toggle_auto(
    state: State<'_, Arc<Mutex<EstadoFirma>>>,
    app: AppHandle,
) -> Result<EstadoAutoResult, String> {
    let activo_ahora = state.lock().unwrap().auto_stop.is_some();

    if activo_ahora {
        // Detener
        let stop = state.lock().unwrap().auto_stop.take();
        if let Some(s) = stop {
            s.notify_one();
        }
    } else {
        // Iniciar
        let carpeta = state.lock().unwrap().carpeta.clone();
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?;

        let estado_arc = Arc::clone(&*state);
        let app_clone = app.clone();
        let res_dir = resource_dir.clone();

        let stop = vigilador::iniciar(carpeta, move |ruta_pdf| {
            let estado = Arc::clone(&estado_arc);
            let _app = app_clone.clone();
            let rd = res_dir.clone();
            let nombre = ruta_pdf
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            tokio::spawn(async move {
                _callback_auto(ruta_pdf, nombre, rd, estado).await;
            });
        });

        state.lock().unwrap().auto_stop = Some(stop);
    }

    Ok(_estado_auto_result(&state.lock().unwrap()))
}

#[tauri::command]
pub fn estado_auto(state: State<'_, Arc<Mutex<EstadoFirma>>>) -> EstadoAutoResult {
    _estado_auto_result(&state.lock().unwrap())
}

#[tauri::command]
pub fn get_log_firma(state: State<'_, Arc<Mutex<EstadoFirma>>>) -> Vec<EntradaLog> {
    let g = state.lock().unwrap();
    g.log.iter().cloned().rev().collect()
}

fn _estado_auto_result(g: &EstadoFirma) -> EstadoAutoResult {
    let activo = g.auto_stop.is_some();
    let subidos = g.log.iter().filter(|e| e.estado == "ok").count();
    let errores = g
        .log
        .iter()
        .filter(|e| e.estado == "error" || e.estado == "sin_datos")
        .count();
    EstadoAutoResult {
        activo,
        subidos,
        errores,
    }
}
