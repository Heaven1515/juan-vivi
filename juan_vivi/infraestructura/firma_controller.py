"""
Orquestador del módulo Firma Electrónica.

Flujo manual:
  1. listar_pdfs()          → lista PDFs en la carpeta configurada
  2. leer_pdf()             → OCR → extrae repertorio + tipo
  3. buscar_datos()         → lookup en casos hardcodeados (o BD futura)
  4. enviar_pdf()           → envía a SIGN+ y registra resultado

Flujo automático:
  - iniciar_auto()          → activa vigilador
  - callback_auto()         → OCR → lookup → SIGN+ sin intervención humana
  - detener_auto()          → para vigilador
"""

import logging
import os
from pathlib import Path

from . import ocr_lector, signplus, vigilador
from .base_casos import buscar_caso
from .base_repertorios import buscar as _buscar_repertorio

logger = logging.getLogger(__name__)


def _buscar_datos(numero: str) -> dict | None:
    """
    Busca los datos de un repertorio para enviar a SIGN+.
    1° intenta casos.json (entradas manuales con todos los campos).
    2° cae en repertorios.json (planilla histórica) y convierte los campos.
    """
    caso = buscar_caso(numero)
    if caso:
        return caso
    reg = _buscar_repertorio(numero)
    if not reg or not reg.get("fecha"):
        return None
    try:
        anio, mes, dia = reg["fecha"].split("-")
        return {
            "numero":        reg["repertorio"],
            "anho":          anio,
            "tipo_contrato": reg.get("materia", ""),
            "fecha_dia":     int(dia),
            "fecha_mes":     int(mes),
            "fecha_anio":    int(anio),
        }
    except Exception:
        return None


# Ruta de carpeta del escáner (persiste en memoria de sesión)
# Cuando exista BD, se guardará allí.
_carpeta_escaner: str = vigilador.RUTA_SCANNER

# Log en memoria para la sesión actual
_log_sesion: list[dict] = []


# ── Configuración carpeta ─────────────────────────────────────────

def get_carpeta() -> str:
    return _carpeta_escaner


def set_carpeta(ruta: str) -> None:
    global _carpeta_escaner
    _carpeta_escaner = ruta
    logger.info("Carpeta escáner configurada: %s", ruta)


# ── Listado de PDFs ───────────────────────────────────────────────

def listar_pdfs() -> list[dict]:
    """
    Lista los PDFs disponibles en la carpeta del escáner.
    Retorna lista de dicts con {nombre, ruta, ya_enviado}.
    """
    carpeta = Path(_carpeta_escaner)
    if not carpeta.exists():
        logger.warning("Carpeta no encontrada: %s", carpeta)
        return []

    enviados = {e["nombre"] for e in _log_sesion if e.get("estado") == "ok"}

    archivos = sorted(
        f.name for f in carpeta.iterdir()
        if f.suffix.lower() == ".pdf"
    )
    return [
        {
            "nombre":      nombre,
            "ruta":        str(carpeta / nombre),
            "ya_enviado":  nombre in enviados,
        }
        for nombre in archivos
    ]


# ── Lectura OCR ───────────────────────────────────────────────────

def leer_pdf(nombre: str) -> dict:
    """
    Aplica OCR al PDF y retorna los datos extraídos.
    Agrega 'datos_bd' si el repertorio está en los casos hardcodeados.
    """
    ruta = str(Path(_carpeta_escaner) / nombre)
    datos_ocr = ocr_lector.extraer_datos_pdf(ruta)

    # Intentar enriquecer con datos hardcodeados
    numero = datos_ocr.get("numero")
    caso   = _buscar_datos(numero) if numero else None

    return {
        "ocr":     datos_ocr,
        "caso":    caso,
        "en_bd":   caso is not None,
    }


# ── Envío manual ──────────────────────────────────────────────────

def enviar_pdf(nombre: str, numero: str, anho: str, tipo_contrato: str,
               fecha_dia: int, fecha_mes: int, fecha_anio: int) -> dict:
    """
    Envía un PDF a SIGN+ con los datos confirmados por el usuario.
    Registra el resultado en el log de sesión.
    """
    ruta = str(Path(_carpeta_escaner) / nombre)
    entrada_log = {
        "nombre":    nombre,
        "repertorio": f"{numero}-{anho}",
        "tipo":      tipo_contrato,
        "estado":    "procesando",
        "mensaje":   "",
    }
    _log_sesion.append(entrada_log)

    try:
        signplus.enviar_formulario(
            ruta_pdf      = ruta,
            numero        = numero,
            anho          = anho,
            tipo_contrato = tipo_contrato,
            fecha_dia     = fecha_dia,
            fecha_mes     = fecha_mes,
            fecha_anio    = fecha_anio,
        )
        entrada_log["estado"] = "ok"
        logger.info("Enviado OK: %s (rep %s-%s)", nombre, numero, anho)

    except Exception as exc:
        entrada_log["estado"]  = "error"
        entrada_log["mensaje"] = str(exc)
        logger.error("Error enviando %s: %s", nombre, exc)

    return dict(entrada_log)


# ── Log de sesión ─────────────────────────────────────────────────

def get_log() -> list[dict]:
    """Retorna el log de envíos de la sesión actual (más reciente primero)."""
    return list(reversed(_log_sesion))


# ── Modo automático ───────────────────────────────────────────────

def _callback_auto(ruta_pdf: str) -> None:
    """
    Procesamiento automático de un PDF nuevo detectado por el vigilador:
      1. OCR
      2. Lookup en casos hardcodeados
      3. Si encontrado → enviar a SIGN+
      4. Si no encontrado → log 'sin_datos', dejar para proceso manual
    """
    nombre = Path(ruta_pdf).name
    logger.info("AUTO | Procesando: %s", nombre)

    try:
        datos_ocr = ocr_lector.extraer_datos_pdf(ruta_pdf)
        numero    = datos_ocr.get("numero")
        anho      = datos_ocr.get("anho")

        if not numero or not anho:
            _log_sesion.append({
                "nombre":    nombre,
                "repertorio": "—",
                "tipo":      "—",
                "estado":    "error",
                "mensaje":   "OCR: repertorio no detectado",
            })
            logger.warning("AUTO | OCR sin repertorio: %s", nombre)
            return

        caso = _buscar_datos(numero)
        if caso is None:
            _log_sesion.append({
                "nombre":    nombre,
                "repertorio": f"{numero}-{anho}",
                "tipo":      datos_ocr.get("tipo_contrato", "—"),
                "estado":    "sin_datos",
                "mensaje":   f"Repertorio {numero}-{anho} no está en la base de datos — procesar manualmente",
            })
            logger.warning("AUTO | Sin datos para rep %s-%s (%s)", numero, anho, nombre)
            return

        # Enviar
        enviar_pdf(
            nombre        = nombre,
            numero        = caso["numero"],
            anho          = caso["anho"],
            tipo_contrato = caso["tipo_contrato"],
            fecha_dia     = caso["fecha_dia"],
            fecha_mes     = caso["fecha_mes"],
            fecha_anio    = caso["fecha_anio"],
        )

    except Exception as exc:
        _log_sesion.append({
            "nombre":    nombre,
            "repertorio": "—",
            "tipo":      "—",
            "estado":    "error",
            "mensaje":   str(exc),
        })
        logger.error("AUTO | Excepción procesando %s: %s", nombre, exc)


def iniciar_auto() -> None:
    vigilador.iniciar_vigilancia(_callback_auto, ruta=_carpeta_escaner)
    logger.info("Modo automático ACTIVADO — carpeta: %s", _carpeta_escaner)


def detener_auto() -> None:
    vigilador.detener_vigilancia()
    logger.info("Modo automático DESACTIVADO")


def estado_auto() -> dict:
    activo      = vigilador.esta_activo()
    total_ok    = sum(1 for e in _log_sesion if e["estado"] == "ok")
    total_error = sum(1 for e in _log_sesion if e["estado"] == "error")
    return {"activo": activo, "subidos": total_ok, "errores": total_error}
