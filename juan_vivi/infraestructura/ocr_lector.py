"""
OCR Lector — extrae datos de los PDFs de JUAN-VIVI.

Diferencia clave vs SAGE:
  - SAGE busca el título después de la línea "WF XXXXXX"
  - JUAN-VIVI no tiene línea WF; el título está en las primeras líneas
    justo después de "PROTOCOLIZACION..." o "PROTOCOLIZACION DE LEY..."

Datos extraídos de la primera página:
  - numero_repertorio  → ej: "9739"
  - anio_repertorio    → ej: "2026"
  - numero_protocolizado → ej: "3966"
  - tipo_contrato      → título limpio de la escritura
"""

import io
import logging
import re
from pathlib import Path

import os
import sys

import fitz
import pytesseract
from PIL import Image

# Ruta de Tesseract: si está empaquetado con PyInstaller usa la copia interna;
# si no, busca la instalación estándar de Windows.
if getattr(sys, "frozen", False):
    _base = sys._MEIPASS
    pytesseract.pytesseract.tesseract_cmd = os.path.join(_base, "tesseract", "tesseract.exe")
    os.environ["TESSDATA_PREFIX"] = os.path.join(_base, "tesseract", "tessdata")
else:
    pytesseract.pytesseract.tesseract_cmd = r"C:\Program Files\Tesseract-OCR\tesseract.exe"

logger = logging.getLogger(__name__)

# ── Patrones de extracción ────────────────────────────────────────

# Detecta "REPERTORIO N° 9739-2026" (con o sin punto, con o sin espacio)
_PAT_REPERTORIO = re.compile(
    r"REPERTORIO\s+N[°º]?\s*([\d.]+)\s*[-–]\s*(\d{4})",
    re.IGNORECASE,
)

# Detecta "PROTOCOLIZADO N° 3966" o "PROTOCOLIZADO N°3966"
_PAT_PROTOCOLIZADO = re.compile(
    r"PROTOCOLIZADO\s+N[°º]?\s*([\d.]+)",
    re.IGNORECASE,
)

# Líneas que indican dónde empieza el título (variantes que aparecen en los PDFs)
_PAT_INICIO_TITULO = re.compile(
    r"^PROTOCOLIZACI[OÓ]N",
    re.IGNORECASE,
)

# Líneas que terminan el bloque del título (inicio del cuerpo del acto)
_PAT_FIN_TITULO = re.compile(
    r"^(En\s+Santiago|AUTOFIN|BANCO|GLOBAL|YUDILKA|En\s+la\s+ciudad)",
    re.IGNORECASE,
)

# Palabras de encabezado a ignorar aunque sean mayúsculas
_EXCLUIR_TITULO = {
    "CAROLINA E. PINA CUEVAS", "CAROLINA E PINA CUEVAS",
    "NOTARIO PUBLICO INTERINO", "NOTARIO PUBLICO",
    "HUERFANOS 979 OF. 501 - SANTIAGO", "HUERFANOS 979 OF 501 SANTIAGO",
    "A",
}


def extraer_datos_pdf(ruta_pdf: str) -> dict:
    """
    Lee la primera página del PDF con OCR y retorna:
      {
        "numero":             str | None,   # N° repertorio
        "anho":               str | None,   # año repertorio
        "protocolizado":      str | None,   # N° protocolizado
        "tipo_contrato":      str | None,   # título de la escritura
      }
    """
    ruta = Path(ruta_pdf)
    if not ruta.exists():
        raise FileNotFoundError(f"PDF no encontrado: {ruta_pdf}")

    texto = _ocr_primera_pagina(ruta_pdf)
    numero, anho        = _extraer_repertorio(texto)
    protocolizado       = _extraer_protocolizado(texto)
    tipo_contrato       = _extraer_tipo_contrato(texto)

    logger.info(
        "OCR | %s | Rep: %s-%s | Protoc: %s | Tipo: %s",
        ruta.name, numero, anho, protocolizado, tipo_contrato,
    )
    return {
        "numero":        numero,
        "anho":          anho,
        "protocolizado": protocolizado,
        "tipo_contrato": tipo_contrato,
    }


# ── Helpers internos ──────────────────────────────────────────────

def _ocr_primera_pagina(ruta_pdf: str) -> str:
    """Renderiza la primera página a 250 DPI y aplica OCR con Tesseract."""
    doc = fitz.open(ruta_pdf)
    if not doc.page_count:
        raise ValueError("El PDF no tiene páginas")
    pixmap = doc[0].get_pixmap(matrix=fitz.Matrix(2.5, 2.5))
    imagen = Image.open(io.BytesIO(pixmap.tobytes("png")))
    doc.close()
    return pytesseract.image_to_string(imagen, lang="eng")


def _extraer_repertorio(texto: str) -> tuple[str | None, str | None]:
    """Extrae número y año del repertorio. Elimina puntos de miles (3.090 → 3090)."""
    m = _PAT_REPERTORIO.search(texto)
    if not m:
        logger.warning("Repertorio no encontrado. Inicio OCR: %s",
                       texto[:200].replace("\n", " | "))
        return None, None
    numero = m.group(1).replace(".", "")
    anho   = m.group(2)
    return numero, anho


def _extraer_protocolizado(texto: str) -> str | None:
    """Extrae el N° protocolizado si existe en el documento."""
    m = _PAT_PROTOCOLIZADO.search(texto)
    if not m:
        return None
    return m.group(1).replace(".", "")


def _extraer_tipo_contrato(texto: str) -> str | None:
    """
    Extrae el título de la escritura.

    En estos PDFs el bloque de título viene así:
      PROTOCOLIZACION              ← línea de arranque (ignorar esta)
      ALZAMIENTO DE PRENDA Y PROHIBICION   ← esta es la que queremos
      AUTOFIN S.A.                 ← fin del bloque
      A
      GUSTAVO ANDRES LEMUI...

    O bien:
      PROTOCOLIZACION DE LEY N° 20.190     ← línea de arranque (ignorar)
      MANDATO Y CONTRATO PRIVADO DE ...    ← título real
      YUDILKA MARTINEZ...                  ← fin

    Estrategia: activar búsqueda al detectar la línea PROTOCOLIZACION*,
    luego tomar la primera línea con ≥2 palabras en mayúsculas que no sea
    un encabezado conocido ni el nombre de una parte.
    """
    lineas    = texto.splitlines()
    tras_inicio = False

    for linea in lineas:
        limpia = linea.strip()
        if not limpia:
            continue

        # Activar al encontrar "PROTOCOLIZACION..."
        if _PAT_INICIO_TITULO.match(limpia):
            tras_inicio = True
            continue

        if not tras_inicio:
            continue

        # Fin del bloque: inicio del cuerpo narrativo o nombre de parte
        if _PAT_FIN_TITULO.match(limpia):
            break

        # Ignorar encabezados conocidos y líneas de una sola letra
        if limpia.upper() in _EXCLUIR_TITULO or len(limpia) <= 2:
            continue

        # Criterio: predominantemente mayúsculas y al menos 2 palabras
        letras = [c for c in limpia if c.isalpha()]
        if len(letras) < 5:
            continue
        ratio    = sum(1 for c in letras if c.isupper()) / len(letras)
        palabras = limpia.split()
        if ratio > 0.75 and len(palabras) >= 2:
            # Limpiar caracteres OCR espurios frecuentes
            titulo = re.sub(r"[°º#@|]", "", limpia).strip()
            return titulo

    logger.warning("Tipo de contrato no encontrado en el PDF")
    return None
