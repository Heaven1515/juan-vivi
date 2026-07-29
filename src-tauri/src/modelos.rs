use serde::{Deserialize, Serialize};

// ─── Repertorios ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registro {
    pub repertorio: String,
    pub ot: String,
    pub fecha: String,
    pub cliente: String,
    pub materia: String,
    pub compareciente: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Duplicado {
    pub numero: String,
    pub existente: Registro,
    pub entrante: Registro,
}

// ─── Planilla ────────────────────────────────────────────────────────────────

/// Fila leída del Excel de entrada (uso interno, no serializable directamente)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FilaEntrada {
    pub numero: i64,
    pub nombre: String,
    pub rut: String,
    pub nombre_para: Option<String>,
    pub rut_para: String,
    pub operacion: String,
    pub patente: String,
    pub tipo: String,
    pub num_fila_excel: usize,
}

/// Fila transformada lista para escribir en el Excel de salida
#[derive(Debug, Clone, Serialize, Default)]
pub struct FilaSalida {
    pub c1_rut: String,
    pub c1_apellido_paterno: String,
    pub c1_apellido_materno: Option<String>,
    pub c1_nombres: Option<String>,
    pub c2_rut: String,
    pub c2_apellido_paterno: String,
    pub c2_apellido_materno: Option<String>,
    pub c2_nombres: Option<String>,
    pub tasacion_fiscal: Option<String>,
    pub precio_venta: Option<String>,
    pub patente: Option<String>,
    pub tipo_vehiculo: Option<String>,
    pub marca: Option<String>,
    pub modelo: Option<String>,
    pub anio: Option<String>,
    pub color: Option<String>,
    pub motor: Option<String>,
    pub chasis: Option<String>,
    pub serie: Option<String>,
    pub vin: Option<String>,
    pub codigo_operacion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilaResumen {
    pub n: String,
    pub nombre: String,
    pub rut: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoCargaPlanilla {
    pub ok: bool,
    pub nombre: String,
    pub filas: Vec<FilaResumen>,
    pub avisos: Vec<String>,
    pub error: Option<String>,
}

/// Estado en memoria de la planilla cargada (gestionado por Tauri)
#[derive(Debug, Default)]
pub struct EstadoPlanilla {
    pub filas: Vec<FilaSalida>,
    pub prefijo_sp: String,
}
