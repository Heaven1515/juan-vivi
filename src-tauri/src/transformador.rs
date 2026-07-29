use crate::modelos::{FilaEntrada, FilaSalida};

// ─── Constantes ──────────────────────────────────────────────────────────────
const AUTOFIN_RUT: &str = "76139506-8";
const AUTOFIN_NOMBRE: &str = "AUTOFIN S.A.";

// ─── Funciones internas ───────────────────────────────────────────────────────

fn limpiar_rut(s: &str) -> String {
    // Eliminar NBSP (\u{00A0}) y espacios, convertir a uppercase
    s.chars()
        .filter(|c| *c != '\u{00A0}' && !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

fn limpiar_nombre(s: &str) -> String {
    // Eliminar NBSP, colapsar espacios dobles, uppercase
    let sin_nbsp: String = s.chars().map(|c| if c == '\u{00A0}' { ' ' } else { c }).collect();
    // Colapsar múltiples espacios en uno
    let mut resultado = String::new();
    let mut prev_espacio = false;
    for c in sin_nbsp.chars() {
        if c == ' ' {
            if !prev_espacio {
                resultado.push(c);
            }
            prev_espacio = true;
        } else {
            resultado.push(c);
            prev_espacio = false;
        }
    }
    resultado.trim().to_uppercase()
}

/// Extrae el primer RUT de una cadena con formato "RUT1/RUT2/..."
fn primer_rut(s: &str) -> String {
    s.split('/').next().unwrap_or(s).trim().to_string()
}

/// Extrae el primer nombre de "NOMBRE1 Y NOMBRE2 Y ..." → "NOMBRE1 Y OTRO"
fn primer_nombre(s: &str) -> String {
    // Busca " Y " (con espacios) y toma la parte anterior
    if let Some(pos) = s.find(" Y ") {
        format!("{} Y OTRO", &s[..pos].trim())
    } else {
        s.to_string()
    }
}

fn es_vacia(s: &str) -> bool {
    s.trim().is_empty()
}

// ─── Transformación ───────────────────────────────────────────────────────────

pub fn transformar_fila(fila: &FilaEntrada) -> (Option<FilaSalida>, Option<String>) {
    let tipo = fila.tipo.trim().to_uppercase();

    // Compareciente 1: siempre AUTOFIN
    let c1_rut = AUTOFIN_RUT.to_string();
    let c1_apellido_paterno = AUTOFIN_NOMBRE.to_string();

    // Compareciente 2 según tipo
    let (c2_rut, c2_apellido_paterno) = match tipo.as_str() {
        "COMPRA PARA" => {
            let rut = limpiar_rut(&fila.rut_para);
            let nombre_raw = fila
                .nombre_para
                .as_deref()
                .unwrap_or("")
                .to_string();
            let nombre = limpiar_nombre(&nombre_raw);

            if es_vacia(&rut) || es_vacia(&nombre) {
                let aviso = format!(
                    "Fila {}: COMPRA PARA sin NOMBRE_PARA o RUT_PARA — omitida",
                    fila.num_fila_excel
                );
                return (None, Some(aviso));
            }
            (rut, nombre)
        }
        "COMUNIDAD" => {
            let rut = limpiar_rut(&primer_rut(&fila.rut));
            let nombre = limpiar_nombre(&primer_nombre(&fila.nombre));
            (rut, nombre)
        }
        _ => {
            // Caso normal (vacío / None / cualquier otro)
            let rut = limpiar_rut(&fila.rut);
            let nombre = limpiar_nombre(&fila.nombre);

            if es_vacia(&rut) || es_vacia(&nombre) {
                let aviso = format!(
                    "Fila {}: NOMBRE o RUT vacío — omitida",
                    fila.num_fila_excel
                );
                return (None, Some(aviso));
            }
            (rut, nombre)
        }
    };

    let patente = if es_vacia(&fila.patente) {
        None
    } else {
        Some(fila.patente.clone())
    };

    let codigo_operacion = if es_vacia(&fila.operacion) {
        None
    } else {
        Some(fila.operacion.clone())
    };

    let salida = FilaSalida {
        c1_rut,
        c1_apellido_paterno,
        c1_apellido_materno: None,
        c1_nombres: None,
        c2_rut,
        c2_apellido_paterno,
        c2_apellido_materno: None,
        c2_nombres: None,
        tasacion_fiscal: None,
        precio_venta: None,
        patente,
        tipo_vehiculo: None,
        marca: None,
        modelo: None,
        anio: None,
        color: None,
        motor: None,
        chasis: None,
        serie: None,
        vin: None,
        codigo_operacion,
    };

    (Some(salida), None)
}

pub fn transformar_lote(filas: &[FilaEntrada]) -> (Vec<FilaSalida>, Vec<String>) {
    let mut salidas: Vec<FilaSalida> = Vec::new();
    let mut avisos: Vec<String> = Vec::new();

    for fila in filas {
        let (salida, aviso) = transformar_fila(fila);
        if let Some(s) = salida {
            salidas.push(s);
        }
        if let Some(a) = aviso {
            avisos.push(a);
        }
    }

    (salidas, avisos)
}
