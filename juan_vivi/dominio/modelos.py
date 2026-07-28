"""
Modelos de datos puros del dominio JUAN-VIVI.
Sin dependencias externas — solo dataclasses estándar.
"""
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class FilaEntrada:
    """
    Representa una fila del Excel AUTOFIN SP (fuente).
    Índices según el encabezado real del archivo.
    """
    numero: int                  # col A — N°
    nombre: str                  # col B — NOMBRE
    rut: str                     # col C — RUT  (puede traer \xa0)
    nombre_para: Optional[str]   # col D — NOMBRE PARA
    rut_para: Optional[str]      # col E — RUT COMPRA PARA
    operacion: Optional[str]     # col F — OPERACIÓN
    patente: Optional[str]       # col G — PATENTE
    tipo: Optional[str]          # col K — COMUNIDAD | COMPRA PARA | None
    num_fila_excel: int          # número real de fila en el Excel (para avisos)


@dataclass
class FilaSalida:
    """
    Representa una fila de la planilla masivos de salida.
    21 columnas según el formato AUTOFIN.
    """
    # Compareciente 1 / Vendedor (cols A–D)
    c1_rut: str = "76139506-8"
    c1_apellido_paterno: str = "AUTOFIN S.A."
    c1_apellido_materno: Optional[str] = None
    c1_nombres: Optional[str] = None

    # Compareciente 2 / Comprador (cols E–H)
    c2_rut: str = ""
    c2_apellido_paterno: str = ""
    c2_apellido_materno: Optional[str] = None
    c2_nombres: Optional[str] = None

    # Vehículo (cols I–T) — siempre vacíos; AUTOFIN SP no los entrega
    tasacion_fiscal: Optional[str] = None
    precio_venta: Optional[str] = None
    patente: Optional[str] = None
    tipo_vehiculo: Optional[str] = None
    marca: Optional[str] = None
    modelo: Optional[str] = None
    anio: Optional[str] = None
    color: Optional[str] = None
    motor: Optional[str] = None
    chasis: Optional[str] = None
    serie: Optional[str] = None
    vin: Optional[str] = None

    # Datos OT (col U)
    codigo_operacion: Optional[str] = None


@dataclass
class ResultadoProcesamiento:
    """Resultado completo de procesar uno o varios Excel de entrada."""
    filas: list = field(default_factory=list)          # lista[FilaSalida]
    avisos: list = field(default_factory=list)         # mensajes de filas ignoradas
    archivos_procesados: list = field(default_factory=list)  # nombres de archivos leídos
