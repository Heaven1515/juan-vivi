use std::collections::HashMap;
use std::path::PathBuf;
use serde::Serialize;

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

        // zip 0.6 usa name() que devuelve &str directamente
        let nombre_zip = entry.name().to_string();

        // Determinar a qué carpeta pertenece la entrada
        let (mapa, dir_destino) = if nombre_zip.starts_with("notaria/") {
            (&mut notaria, dir_temp.join("notaria"))
        } else if nombre_zip.starts_with("firmados/") {
            (&mut firmados, dir_temp.join("firmados"))
        } else {
            continue;
        };

        // Solo archivos PDF (extensión case-insensitive)
        let nombre_archivo = match nombre_zip.split('/').last() {
            Some(n) if n.to_lowercase().ends_with(".pdf") => n.to_string(),
            _ => continue,
        };

        // Clave en minúsculas para deduplicar (mismo archivo repetido en el ZIP)
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

/// Envía un PDF a imprimir:
/// 1. Guarda la impresora predeterminada actual
/// 2. Pone la impresora elegida como predeterminada via WMI
/// 3. Usa ShellExecute "print" (funciona con cualquier visor, incluido Edge)
/// 4. Restaura la impresora original
fn imprimir_archivo(ruta_pdf: &str, impresora: &str) -> Result<(), String> {
    let ruta_seg = ruta_pdf.replace('\'', "''");
    let imp_seg  = impresora.replace('\'', "''").replace('"', "");

    let script = format!(
        r#"
# Guardar impresora predeterminada actual
$anterior = (Get-Printer | Where-Object Default -eq $true | Select-Object -First 1).Name

# Poner la impresora elegida como predeterminada
$wmi = Get-WmiObject -Query "SELECT * FROM Win32_Printer WHERE Name='{imp}'"
if ($wmi) {{ $wmi.SetDefaultPrinter() | Out-Null }} else {{ Write-Error "Impresora no encontrada: {imp}"; exit 1 }}
Start-Sleep -Milliseconds 500

# Imprimir con ShellExecute "print" — funciona con Edge y cualquier visor
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Shell {{
    [DllImport("shell32.dll", CharSet = CharSet.Auto)]
    public static extern IntPtr ShellExecute(
        IntPtr hwnd, string lpOperation, string lpFile,
        string lpParameters, string lpDirectory, int nShowCmd);
}}
"@ -ErrorAction SilentlyContinue
[Shell]::ShellExecute([IntPtr]::Zero, 'print', '{ruta}', '', '', 0)

# Dar tiempo al spooler para recibir el trabajo antes de restaurar
Start-Sleep -Milliseconds 3000

# Restaurar impresora original
if ($anterior -and $anterior -ne '{imp}') {{
    $orig = Get-WmiObject -Query "SELECT * FROM Win32_Printer WHERE Name='$anterior'"
    if ($orig) {{ $orig.SetDefaultPrinter() | Out-Null }}
}}
"#,
        imp  = imp_seg,
        ruta = ruta_seg,
    );

    let salida = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive",
            "-WindowStyle", "Hidden",
            "-Command", &script,
        ])
        .output()
        .map_err(|e| format!("Error lanzando impresión: {}", e))?;

    if !salida.status.success() {
        let err = String::from_utf8_lossy(&salida.stderr);
        return Err(format!("Error de impresión: {}", err.trim()));
    }
    Ok(())
}

/// Imprime un par completo: primero el PDF de notaria, luego el de firmados.
/// La pausa entre ambos asegura que el spooler los mantenga en orden.
#[tauri::command]
pub fn imprimir_par(notaria: String, firmados: String, impresora: String) -> Result<(), String> {
    imprimir_archivo(&notaria, &impresora)?;
    std::thread::sleep(std::time::Duration::from_millis(800));
    imprimir_archivo(&firmados, &impresora)?;
    Ok(())
}
