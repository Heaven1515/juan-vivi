use anyhow::anyhow;
use once_cell::sync::Lazy;
use pdfium_render::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::path::Path;

// ─── Patrones regex ───────────────────────────────────────────────────────────

// Flexible: "REPERTORIO N° 3.090 - 2024", "REPERTORION°3090", "REPERTORIO 1234", etc.
static RE_REPERTORIO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)REPERTORIO\s*N?[°oº]?\s*([\d.]+)(?:\s*[-–]\s*(\d{4}))?").unwrap()
});

static RE_PROTOCOLIZADO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)PROTOCOLIZADO\s+N[°º]?\s*([\d.]+)").unwrap()
});

static RE_INICIO_TITULO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^PROTOCOLIZACION").unwrap()
});

static RE_FIN_TITULO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(En\s+Santiago|AUTOFIN|BANCO|GLOBAL|YUDILKA|En\s+la\s+ciudad)").unwrap()
});

const EXCLUIR_TITULO: &[&str] = &[
    "CAROLINA E. PINA CUEVAS",
    "CAROLINA E PINA CUEVAS",
    "NOTARIO PUBLICO INTERINO",
    "NOTARIO PUBLICO",
    "HUERFANOS 979 OF. 501 - SANTIAGO",
    "HUERFANOS 979 OF 501 SANTIAGO",
    "A",
];

// ─── Structs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DatosOcr {
    pub numero: Option<String>,
    pub anho: Option<String>,
    pub protocolizado: Option<String>,
    pub tipo_contrato: Option<String>,
}

// ─── API pública ─────────────────────────────────────────────────────────────

pub fn crear_pdfium(resource_dir: &Path) -> anyhow::Result<Pdfium> {
    let bindings = Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path(resource_dir)
    )
    .map_err(|e| anyhow!("pdfium: {:?}", e))?;
    Ok(Pdfium::new(bindings))
}

pub fn extraer_datos(
    ruta_pdf: &Path,
    tesseract_dir: &Path,
    pdfium: &Pdfium,
) -> anyhow::Result<DatosOcr> {
    let texto = ocr_primera_pagina(ruta_pdf, tesseract_dir, pdfium)?;

    let (numero, anho) = extraer_repertorio(&texto);
    let protocolizado = extraer_protocolizado(&texto);
    let tipo_contrato = extraer_tipo_contrato(&texto);

    Ok(DatosOcr {
        numero,
        anho,
        protocolizado,
        tipo_contrato,
    })
}

pub fn renderizar_pagina(
    ruta_pdf: &Path,
    pagina: u16,
    escala: f32,
    pdfium: &Pdfium,
) -> anyhow::Result<(String, u16)> {
    let doc = pdfium
        .load_pdf_from_file(ruta_pdf, None)
        .map_err(|e| anyhow!("{:?}", e))?;

    let total = doc.pages().len();

    let page = doc
        .pages()
        .get(pagina)
        .map_err(|e| anyhow!("{:?}", e))?;

    let config = PdfRenderConfig::new().scale_page_by_factor(escala);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| anyhow!("{:?}", e))?;

    let imagen = bitmap.as_image();

    let mut buf = Vec::new();
    imagen.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )?;

    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);

    Ok((b64, total))
}

// ─── Funciones privadas ───────────────────────────────────────────────────────

fn ocr_primera_pagina(
    ruta_pdf: &Path,
    tesseract_dir: &Path,
    pdfium: &Pdfium,
) -> anyhow::Result<String> {
    let doc = pdfium
        .load_pdf_from_file(ruta_pdf, None)
        .map_err(|e| anyhow!("{:?}", e))?;

    let page = doc
        .pages()
        .get(0u16)
        .map_err(|e| anyhow!("{:?}", e))?;

    let config = PdfRenderConfig::new().scale_page_by_factor(2.5f32);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| anyhow!("{:?}", e))?;
    let imagen = bitmap.as_image();

    // Guardar PNG temporal
    let tmp_png = std::env::temp_dir().join(format!(
        "jv_ocr_{}.png",
        ruta_pdf
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    imagen.save(&tmp_png)?;

    // Ejecutar Tesseract
    let tesseract_exe = tesseract_dir.join("tesseract.exe");
    let tessdata_dir = tesseract_dir.join("tessdata");

    let output_base = std::env::temp_dir().join(format!(
        "jv_ocr_{}_out",
        ruta_pdf
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
    ));

    let status = std::process::Command::new(&tesseract_exe)
        .arg(&tmp_png)
        .arg(&output_base)
        .arg("eng")
        .env("TESSDATA_PREFIX", &tessdata_dir)
        .output()?;

    // Limpiar PNG temporal
    let _ = std::fs::remove_file(&tmp_png);

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        return Err(anyhow!("Tesseract falló: {}", stderr));
    }

    let txt_path = output_base.with_extension("txt");
    let texto = std::fs::read_to_string(&txt_path).unwrap_or_default();
    let _ = std::fs::remove_file(&txt_path);

    Ok(texto)
}

/// Limpia un número de repertorio extraído por OCR:
/// quita puntos/espacios y corrige confusiones típicas de Tesseract (O→0, l/I→1).
pub fn normalizar_numero(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'O' | 'o' => '0',
            'l' | 'I' => '1',
            _ => c,
        })
        .filter(|c| c.is_ascii_digit())
        .collect()
}

fn extraer_repertorio(texto: &str) -> (Option<String>, Option<String>) {
    // Solo buscar en el primer cuarto del texto — el repertorio siempre aparece arriba
    let lineas: Vec<&str> = texto.lines().collect();
    let limite = (lineas.len() / 4).max(10).min(lineas.len());
    let zona = lineas[..limite].join("\n");

    if let Some(cap) = RE_REPERTORIO.captures(&zona) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        // Normalizar: quitar puntos, corregir O→0 y l/I→1
        let numero = normalizar_numero(raw);
        let anho = cap.get(2).map(|m| m.as_str().to_string());
        if numero.is_empty() { (None, anho) } else { (Some(numero), anho) }
    } else {
        (None, None)
    }
}

fn extraer_protocolizado(texto: &str) -> Option<String> {
    RE_PROTOCOLIZADO.captures(texto).map(|cap| {
        cap.get(1)
            .map(|m| m.as_str().replace('.', ""))
            .unwrap_or_default()
    })
}

fn limpiar_titulo(s: &str) -> String {
    s.chars()
        .filter(|c| !"°º#@|".contains(*c))
        .collect::<String>()
        .trim()
        .to_string()
}

fn ratio_mayusculas(s: &str) -> f64 {
    let letras: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    if letras.is_empty() {
        return 0.0;
    }
    let mayus = letras.iter().filter(|c| c.is_uppercase()).count();
    mayus as f64 / letras.len() as f64
}

fn extraer_tipo_contrato(texto: &str) -> Option<String> {
    let lineas: Vec<&str> = texto.lines().collect();
    let mut en_zona = false;

    for linea in &lineas {
        let trimmed = linea.trim();

        if !en_zona {
            if RE_INICIO_TITULO.is_match(trimmed) {
                en_zona = true;
            }
            continue;
        }

        // Detectar fin de zona
        if RE_FIN_TITULO.is_match(trimmed) {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Verificar exclusión
        let upper = trimmed.to_uppercase();
        if EXCLUIR_TITULO.iter().any(|e| *e == upper.as_str()) {
            continue;
        }

        // Verificar ratio mayúsculas > 0.75, ≥ 2 palabras, ≥ 5 letras
        let palabras: Vec<&str> = trimmed.split_whitespace().collect();
        let letras: usize = trimmed.chars().filter(|c| c.is_alphabetic()).count();

        if palabras.len() >= 2 && letras >= 5 && ratio_mayusculas(trimmed) > 0.75 {
            return Some(limpiar_titulo(trimmed));
        }
    }

    None
}
