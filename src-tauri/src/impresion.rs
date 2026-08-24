use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use pdfium_render::prelude::*;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// CREATE_NO_WINDOW: evita que aparezca la ventana negra de consola al lanzar PowerShell
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ─── Structs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ParImpresion {
    pub nombre:   String,  // "13364-2026"
    pub notaria:  String,  // ruta absoluta al PDF extraído en temp
    pub firmados: String,  // ruta absoluta al PDF extraído en temp
}

#[derive(Debug, Serialize)]
pub struct ResultadoCargaZip {
    pub pares: Vec<ParImpresion>,
    pub total: usize,
}

// ─── Impresión silenciosa vía GDI (sin abrir ningún visor) ────────────────────

/// Renderiza cada página del PDF con pdfium y la envía directamente a la
/// impresora vía Windows GDI.  No se abre ningún visor de PDF.
#[cfg(target_os = "windows")]
fn imprimir_pdf_gdi(ruta: &Path, impresora: &str, pdfium: &Pdfium) -> Result<(), String> {
    use winapi::um::wingdi::{
        CreateDCW, DeleteDC, EndDoc, EndPage, GetDeviceCaps,
        StartDocW, StartPage, StretchDIBits,
        BITMAPINFO, BITMAPINFOHEADER, DOCINFOW, RGBQUAD,
        BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        HORZRES, VERTRES,
    };

    // ── Crear DC de la impresora ──────────────────────────────────────────────
    let imp_wide: Vec<u16> = impresora.encode_utf16().chain(std::iter::once(0)).collect();

    let hdc = unsafe {
        CreateDCW(
            std::ptr::null(),
            imp_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };

    if hdc.is_null() {
        return Err(format!("No se pudo crear DC para impresora '{}'", impresora));
    }

    // Dimensiones en píxeles de la hoja en la impresora
    let page_w = unsafe { GetDeviceCaps(hdc, HORZRES) };
    let page_h = unsafe { GetDeviceCaps(hdc, VERTRES) };

    // ── Iniciar trabajo de impresión ──────────────────────────────────────────
    let job_name: Vec<u16> = "JUAN-VIVI".encode_utf16().chain(std::iter::once(0)).collect();

    let doc_info = DOCINFOW {
        cbSize:       std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName:  job_name.as_ptr(),
        lpszOutput:   std::ptr::null::<u16>() as *mut u16,
        lpszDatatype: std::ptr::null::<u16>() as *mut u16,
        fwType:       0,
    };

    let job_id = unsafe { StartDocW(hdc, &doc_info) };
    if job_id <= 0 {
        unsafe { DeleteDC(hdc) };
        return Err(format!("StartDocW falló (code={})", job_id));
    }

    // ── Cargar PDF con pdfium ─────────────────────────────────────────────────
    let doc = pdfium
        .load_pdf_from_file(ruta, None)
        .map_err(|e| format!("pdfium load: {:?}", e))?;

    let total = doc.pages().len();

    for i in 0..total {
        unsafe { StartPage(hdc) };

        let page = match doc.pages().get(i) {
            Ok(p) => p,
            Err(e) => {
                // Si una página falla, terminamos el trabajo limpiamente
                unsafe { EndPage(hdc); EndDoc(hdc); DeleteDC(hdc); }
                return Err(format!("pdfium página {}: {:?}", i, e));
            }
        };

        // Renderizar a ~300 DPI (base PDF = 72 DPI → factor ≈ 4.17)
        let scale = 300.0_f32 / 72.0_f32;
        let config = PdfRenderConfig::new().scale_page_by_factor(scale);
        let bitmap = match page.render_with_config(&config) {
            Ok(b) => b,
            Err(e) => {
                unsafe { EndPage(hdc); EndDoc(hdc); DeleteDC(hdc); }
                return Err(format!("pdfium render {}: {:?}", i, e));
            }
        };

        let img = bitmap.as_image().to_rgba8();
        let (img_w, img_h) = img.dimensions();

        // Convertir RGBA → BGRA (Windows GDI espera BGR little-endian)
        let raw = img.into_raw();
        let mut bgra: Vec<u8> = Vec::with_capacity(raw.len());
        for chunk in raw.chunks_exact(4) {
            bgra.push(chunk[2]); // B
            bgra.push(chunk[1]); // G
            bgra.push(chunk[0]); // R
            bgra.push(chunk[3]); // A
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:          std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:         img_w as i32,
                biHeight:        -(img_h as i32), // negativo = top-down (el origen es arriba)
                biPlanes:        1,
                biBitCount:      32,
                biCompression:   BI_RGB,
                biSizeImage:     0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed:       0,
                biClrImportant:  0,
            },
            bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }],
        };

        unsafe {
            StretchDIBits(
                hdc,
                0, 0, page_w, page_h,           // destino: toda la hoja imprimible
                0, 0, img_w as i32, img_h as i32, // fuente: toda la imagen renderizada
                bgra.as_ptr() as *const _,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            EndPage(hdc);
        }
    }

    unsafe {
        EndDoc(hdc);
        DeleteDC(hdc);
    }

    Ok(())
}

// ─── Comandos ─────────────────────────────────────────────────────────────────

/// Extrae el ZIP a una carpeta temporal, empareja los PDFs de notaria/ y firmados/
/// por nombre, deduplica entradas repetidas y retorna los pares ordenados por
/// número de repertorio.
#[tauri::command]
pub fn cargar_zip(ruta: String) -> Result<ResultadoCargaZip, String> {
    let archivo = std::fs::File::open(&ruta)
        .map_err(|e| format!("No se pudo abrir el ZIP: {}", e))?;
    let mut zip = zip::ZipArchive::new(archivo)
        .map_err(|e| format!("ZIP inválido: {}", e))?;

    // Limpiar y recrear carpeta temporal exclusiva
    let dir_temp = std::env::temp_dir().join("juan-vivi-impresion");
    if dir_temp.exists() {
        std::fs::remove_dir_all(&dir_temp)
            .map_err(|e| format!("Error limpiando temp: {}", e))?;
    }
    std::fs::create_dir_all(dir_temp.join("notaria"))
        .map_err(|e| format!("Error creando temp/notaria: {}", e))?;
    std::fs::create_dir_all(dir_temp.join("firmados"))
        .map_err(|e| format!("Error creando temp/firmados: {}", e))?;

    let mut notaria:  HashMap<String, PathBuf> = HashMap::new();
    let mut firmados: HashMap<String, PathBuf> = HashMap::new();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)
            .map_err(|e| format!("Error leyendo entrada ZIP {}: {}", i, e))?;

        let nombre_zip = entry.name().to_string();

        let (mapa, dir_destino) = if nombre_zip.starts_with("notaria/") {
            (&mut notaria, dir_temp.join("notaria"))
        } else if nombre_zip.starts_with("firmados/") {
            (&mut firmados, dir_temp.join("firmados"))
        } else {
            continue;
        };

        let nombre_archivo = match nombre_zip.split('/').last() {
            Some(n) if n.to_lowercase().ends_with(".pdf") => n.to_string(),
            _ => continue,
        };

        let clave = nombre_archivo.to_lowercase();
        if mapa.contains_key(&clave) {
            continue;
        }

        let destino = dir_destino.join(&nombre_archivo);
        let mut dest_file = std::fs::File::create(&destino)
            .map_err(|e| format!("Error creando {}: {}", nombre_archivo, e))?;
        std::io::copy(&mut entry, &mut dest_file)
            .map_err(|e| format!("Error extrayendo {}: {}", nombre_archivo, e))?;

        mapa.insert(clave, destino);
    }

    // Emparejar: solo los que tienen par en ambas carpetas
    let mut pares: Vec<ParImpresion> = notaria
        .iter()
        .filter_map(|(clave, ruta_n)| {
            firmados.get(clave).map(|ruta_f| {
                let nombre = ruta_n
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| clave.clone());
                ParImpresion {
                    nombre,
                    notaria:  ruta_n.to_string_lossy().to_string(),
                    firmados: ruta_f.to_string_lossy().to_string(),
                }
            })
        })
        .collect();

    // Ordenar numéricamente por la parte antes del guión (ej: 13364 de "13364-2026")
    pares.sort_by(|a, b| {
        let n_a: u64 = a.nombre.split('-').next().unwrap_or("0").parse().unwrap_or(0);
        let n_b: u64 = b.nombre.split('-').next().unwrap_or("0").parse().unwrap_or(0);
        n_a.cmp(&n_b)
    });

    let total = pares.len();
    Ok(ResultadoCargaZip { pares, total })
}

/// Lista las impresoras disponibles en Windows vía PowerShell Get-Printer.
#[tauri::command]
pub fn listar_impresoras() -> Result<Vec<String>, String> {
    let salida = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive", "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Error listando impresoras: {}", e))?;

    let texto = String::from_utf8_lossy(&salida.stdout);
    let lista: Vec<String> = texto
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(lista)
}

/// Imprime un par completo vía GDI: notaria primero, luego firmados.
/// Completamente silencioso — no abre ningún visor de PDF.
#[tauri::command]
pub async fn imprimir_par(
    notaria: String,
    firmados: String,
    impresora: String,
    app: AppHandle,
) -> Result<(), String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || {
        let pdfium = crate::ocr::crear_pdfium(&resource_dir)
            .map_err(|e| e.to_string())?;

        imprimir_pdf_gdi(Path::new(&notaria), &impresora, &pdfium)?;
        std::thread::sleep(std::time::Duration::from_millis(800));
        imprimir_pdf_gdi(Path::new(&firmados), &impresora, &pdfium)?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
