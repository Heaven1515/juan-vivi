"""
Envía el PDF y sus datos al servidor SIGN+ de la notaría.

Flujo idéntico a SAGE/formulario_controller.py:
  1. POST /login.php  → extrae Bearer JWT del HTML de respuesta
  2. POST /escrituras_publicas/api/ con multipart/form-data + Bearer
     accion = guardar_copia_compulsa
     datos  = JSON con campos del formulario
     file_principal = PDF

Accesible solo desde la red interna de la notaría.
"""

import json
import logging
import re
from datetime import date
from pathlib import Path

import requests

logger = logging.getLogger(__name__)

# ── Configuración SIGN+ ───────────────────────────────────────────
URL_BASE         = "http://192.168.1.177"
URL_API          = "http://192.168.1.177/app/escrituras_publicas/api/"
USUARIO          = "JESPINA"
PASSWORD         = "Cpina2026"
TIMEOUT_SEGUNDOS = 30

_MESES = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio",
    "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
]


def enviar_formulario(
    ruta_pdf:      str,
    numero:        str,
    anho:          str,
    tipo_contrato: str,
    fecha_dia:     int,
    fecha_mes:     int,
    fecha_anio:    int,
) -> None:
    """
    Envía el PDF al formulario SIGN+.
    Lanza excepción si falla el login, la conexión o el servidor rechaza.
    El archivo NO se elimina aquí — lo hace el controller si todo va bien.
    """
    ruta = Path(ruta_pdf)
    if not ruta.exists():
        raise FileNotFoundError(f"PDF no encontrado al enviar: {ruta_pdf}")

    sesion, token = _login()

    fecha_escritura = f"{fecha_anio:04d}-{fecha_mes:02d}-{fecha_dia:02d}"
    datos = _construir_datos(numero, anho, tipo_contrato, fecha_escritura)

    with open(ruta, "rb") as f:
        respuesta = sesion.post(
            URL_API,
            data={
                "accion": "guardar_copia_compulsa",
                "datos":  json.dumps(datos),
            },
            files={"file_principal": (ruta.name, f, "application/pdf")},
            headers={"Authorization": f"Bearer {token}"},
            timeout=TIMEOUT_SEGUNDOS,
        )

    if respuesta.status_code != 200:
        raise RuntimeError(
            f"SIGN+ retornó HTTP {respuesta.status_code} al subir {ruta.name}"
        )

    resultado = respuesta.json()
    if not resultado.get("estado"):
        raise RuntimeError(
            f"SIGN+ rechazó la copia: {resultado.get('mensaje', 'sin mensaje')}"
        )

    logger.info(
        "SIGN+ OK | Rep: %s-%s | Archivo: %s | id_firma: %s",
        numero, anho, ruta.name, resultado.get("id_firma", "-"),
    )


# ── Helpers internos ──────────────────────────────────────────────

def _login() -> tuple[requests.Session, str]:
    """Abre sesión en SIGN+ y retorna (sesion, token JWT)."""
    sesion = requests.Session()
    sesion.headers.update({"User-Agent": "Mozilla/5.0"})

    respuesta = sesion.post(
        f"{URL_BASE}/login.php",
        data={
            "username":    USUARIO,
            "password":    PASSWORD,
            "secondLogin": "s",
        },
        timeout=TIMEOUT_SEGUNDOS,
    )

    match = re.search(
        r"localStorage\.setItem\('token',\s*'([^']+)'\)",
        respuesta.text,
    )
    if not match:
        raise RuntimeError(
            "Login a SIGN+ fallido: no se encontró el token JWT. "
            "Verifica usuario/contraseña en signplus.py"
        )

    token = match.group(1)
    logger.debug("Login SIGN+ OK — token %d chars", len(token))
    return sesion, token


def _construir_datos(
    numero:          str,
    anho:            str,
    tipo_contrato:   str,
    fecha_escritura: str,   # YYYY-MM-DD
) -> dict:
    """Construye el payload JSON para el campo 'datos' del POST."""
    hoy          = date.today().isoformat()
    cuerpo_cert  = _generar_cuerpo_cert(tipo_contrato, fecha_escritura)

    return {
        "ot":                      "",
        "repertorio":              numero,
        "anho":                    int(anho),
        "tipo_contrato":           tipo_contrato,
        "email":                   "",
        "fecha_escritura":         fecha_escritura,
        "monto":                   "",
        "moneda":                  "PESOS",
        "voltear":                 False,
        "fecha_emision":           hoy,
        "matricera":               "",
        "banco":                   "",
        "modificar_certificacion": False,
        "cuerpo_cert":             cuerpo_cert,
        "existe_pdf":              "nop",
        "url_pdf_existente":       "",
        "lista_comparecientes":    [],
    }


def _generar_cuerpo_cert(tipo_contrato: str, fecha_escritura: str) -> str:
    """Genera el texto de certificación (mismo algoritmo que SAGE)."""
    tipo = tipo_contrato.upper() if tipo_contrato else "LA ESCRITURA PUBLICA"
    try:
        year, month, day = fecha_escritura.split("-")
        nombre_mes = _MESES[int(month) - 1]
        return (
            f"Certifico que el presente documento electrónico es copia fiel e "
            f"íntegra de {tipo} otorgado el {day} de {nombre_mes} de {year} "
            f"reproducido en las siguientes páginas."
        )
    except Exception:
        return (
            f"Certifico que el presente documento electrónico es copia fiel e "
            f"íntegra de {tipo} reproducido en las siguientes páginas."
        )
