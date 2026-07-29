use chrono::Local;
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};

use crate::modelos::FilaSalida;

// ─── Anchos de columna (índice 0-based, ancho) ───────────────────────────────
const ANCHOS_COLUMNA: &[(u16, f64)] = &[
    (0, 13.0),
    (1, 41.14),
    (2, 20.57),
    (3, 33.57),
    (4, 13.0),
    (5, 41.14),
    (6, 20.57),
    (7, 33.57),
    (8, 13.86),
    (9, 15.0),
    (10, 12.86),
    (11, 8.0),
    (12, 10.0),
    (13, 14.0),
    (14, 6.0),
    (15, 8.0),
    (16, 10.0),
    (17, 12.0),
    (18, 10.0),
    (19, 10.0),
    (20, 19.57),
];

// ─── Encabezados individuales (fila 2) ───────────────────────────────────────
const ENCABEZADOS: &[&str] = &[
    "RUT",
    "Apellido Paterno",
    "Apellido Materno",
    "Nombres",
    "RUT",
    "Apellido Paterno",
    "Apellido Materno",
    "Nombres",
    "Tasación Fiscal",
    "Precio Venta",
    "Patente",
    "Tipo Vehículo",
    "Marca",
    "Modelo",
    "Año",
    "Color",
    "N° Motor",
    "N° Chasis",
    "N° Serie",
    "VIN",
    "Código Operación",
];

// ─── Escribir planilla ────────────────────────────────────────────────────────

pub fn escribir_planilla(filas: &[FilaSalida], ruta: &str) -> anyhow::Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();

    // Aplicar anchos
    for &(col, ancho) in ANCHOS_COLUMNA {
        ws.set_column_width(col, ancho)?;
    }

    // Formato grupo (fila 0): centrado, wrap
    let fmt_grupo = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap();

    // Fila 0: grupos combinados
    ws.set_row_height(0, 18.0)?;
    ws.merge_range(0, 0, 0, 3, "Compareciente 1 / Vendedor", &fmt_grupo)?;
    ws.merge_range(0, 4, 0, 7, "Compareciente 2 / Comprador", &fmt_grupo)?;
    ws.merge_range(0, 8, 0, 19, "Vehículo", &fmt_grupo)?;
    ws.write_with_format(0, 20, "Datos OT", &fmt_grupo)?;

    // Formato encabezado (fila 1): borde thin top+bottom, centrado
    let fmt_enc = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(FormatBorder::Thin)
        .set_border_bottom(FormatBorder::Thin);

    // Fila 1: encabezados
    ws.set_row_height(1, 30.0)?;
    for (col, &enc) in ENCABEZADOS.iter().enumerate() {
        ws.write_with_format(1, col as u16, enc, &fmt_enc)?;
    }

    // Formato datos: Calibri 11, izquierda
    let fmt_datos = Format::new()
        .set_font_name("Calibri")
        .set_font_size(11.0)
        .set_align(FormatAlign::Left);

    // Filas de datos (empiezan en fila 2)
    for (i, fila) in filas.iter().enumerate() {
        let row = (i + 2) as u32;
        let cols: Vec<String> = vec![
            fila.c1_rut.clone(),
            fila.c1_apellido_paterno.clone(),
            fila.c1_apellido_materno.clone().unwrap_or_default(),
            fila.c1_nombres.clone().unwrap_or_default(),
            fila.c2_rut.clone(),
            fila.c2_apellido_paterno.clone(),
            fila.c2_apellido_materno.clone().unwrap_or_default(),
            fila.c2_nombres.clone().unwrap_or_default(),
            fila.tasacion_fiscal.clone().unwrap_or_default(),
            fila.precio_venta.clone().unwrap_or_default(),
            fila.patente.clone().unwrap_or_default(),
            fila.tipo_vehiculo.clone().unwrap_or_default(),
            fila.marca.clone().unwrap_or_default(),
            fila.modelo.clone().unwrap_or_default(),
            fila.anio.clone().unwrap_or_default(),
            fila.color.clone().unwrap_or_default(),
            fila.motor.clone().unwrap_or_default(),
            fila.chasis.clone().unwrap_or_default(),
            fila.serie.clone().unwrap_or_default(),
            fila.vin.clone().unwrap_or_default(),
            fila.codigo_operacion.clone().unwrap_or_default(),
        ];

        for (col, valor) in cols.iter().enumerate() {
            ws.write_with_format(row, col as u16, valor.as_str(), &fmt_datos)?;
        }
    }

    // Crear directorio padre si no existe
    let path = std::path::Path::new(ruta);
    if let Some(padre) = path.parent() {
        if !padre.as_os_str().is_empty() {
            std::fs::create_dir_all(padre)?;
        }
    }

    wb.save(ruta)?;
    Ok(())
}

// ─── Nombre de planilla ───────────────────────────────────────────────────────

pub fn nombre_planilla(prefijo_sp: &str) -> String {
    if !prefijo_sp.is_empty() {
        format!("Repertorios Masivo SP {}.xlsx", prefijo_sp)
    } else {
        format!(
            "planilla_masivos_{}.xlsx",
            Local::now().format("%Y-%m-%d_%H-%M")
        )
    }
}
