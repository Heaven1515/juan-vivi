"""
server.py — Sidecar FastAPI de JUAN-VIVI.
Tauri lo lanza como proceso hijo pasando el puerto como argumento.
"""
import sys
import os
import logging
import base64
import re
from pathlib import Path

# ── Rutas y logging ───────────────────────────────────────────────────────────
if getattr(sys, "frozen", False):
    RAIZ = Path(sys.executable).parent
    logging.basicConfig(
        filename=str(RAIZ / "juan_vivi.log"),
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
        encoding="utf-8",
    )
else:
    RAIZ = Path(__file__).parent
    logging.basicConfig(level=logging.WARNING)

sys.path.insert(0, str(RAIZ))

# ── Check de licencia ─────────────────────────────────────────────────────────
# API de GitHub — sin cache, siempre el valor real
_LICENCIA_URL = (
    "https://api.github.com/repos/Heaven1515/juan-vivi/contents/licencia.json"
)

def verificar_licencia() -> tuple[bool, str]:
    """
    Consulta licencia.json via GitHub API (sin cache CDN).
    Retorna (activo, mensaje).
    Si no hay internet o falla la consulta → (True, '') — modo permisivo.
    """
    try:
        import urllib.request
        import json as _json
        import base64 as _b64
        req = urllib.request.Request(
            _LICENCIA_URL,
            headers={"Accept": "application/vnd.github+json", "User-Agent": "juan-vivi"},
        )
        with urllib.request.urlopen(req, timeout=3) as resp:
            meta = _json.loads(resp.read().decode())
        # El contenido viene en base64
        data = _json.loads(_b64.b64decode(meta["content"]).decode())
        return bool(data.get("activo", True)), str(data.get("mensaje", ""))
    except Exception:
        return True, ""   # permisivo si no hay internet

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

from juan_vivi.infraestructura.lector import leer_multiples
from juan_vivi.infraestructura.escritor import escribir_planilla, nombre_archivo_salida
from juan_vivi.dominio.transformador import transformar_lote
from juan_vivi.infraestructura import firma_controller
from juan_vivi.infraestructura.base_casos import buscar_caso, agregar_caso, listar_casos

app = FastAPI(title="JUAN-VIVI API")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Estado en memoria ────────────────────────────────────────────────────────
_ultimo_resultado = None
_ultimo_prefijo_sp = ""


# ── Helper nombre SP ────────────────────────────────────────────────────────
def _extraer_numero_sp(rutas: list[str]) -> str:
    numeros = []
    for ruta in rutas:
        nombre = os.path.splitext(os.path.basename(ruta))[0]
        sin_fecha = re.sub(r"\d{1,2}[-/]\d{1,2}[-/]\d{4}", "", nombre)
        sin_fecha = re.sub(r"\d{4}[-/]\d{1,2}[-/]\d{1,2}", "", sin_fecha)
        m = re.search(r"\d+", sin_fecha)
        if m:
            numeros.append(m.group(0))
    return "_".join(numeros) if numeros else ""


# ── Modelos ──────────────────────────────────────────────────────────────────
class CargarExcelBody(BaseModel):
    rutas: list[str]

class GenerarPlanillaBody(BaseModel):
    ruta_destino: str

class SetCarpetaBody(BaseModel):
    ruta: str

class EnviarPDFBody(BaseModel):
    nombre: str
    numero: str
    anho: str
    tipo_contrato: str
    fecha_dia: int
    fecha_mes: int
    fecha_anio: int

class AgregarCasoBody(BaseModel):
    numero: str
    anho: str
    tipo_contrato: str
    fecha_dia: int
    fecha_mes: int
    fecha_anio: int


# ── Repertorios ──────────────────────────────────────────────────────────────
@app.post("/cargar-excel")
def cargar_excel(body: CargarExcelBody):
    global _ultimo_resultado, _ultimo_prefijo_sp
    filas_entrada, errores = leer_multiples(body.rutas)
    if not filas_entrada:
        return {"ok": False, "error": "No se pudieron leer los archivos", "errores": errores}
    resultado = transformar_lote(filas_entrada)
    _ultimo_resultado = resultado
    _ultimo_prefijo_sp = _extraer_numero_sp(body.rutas)
    nombres = [os.path.basename(r) for r in body.rutas]
    filas_json = [
        {"n": str(i + 1), "nombre": f.c2_apellido_paterno, "rut": f.c2_rut}
        for i, f in enumerate(resultado.filas)
    ]
    return {
        "ok": True,
        "nombre": " + ".join(nombres),
        "filas": filas_json,
        "avisos": resultado.avisos,
    }


@app.get("/nombre-planilla")
def get_nombre_planilla():
    return {"nombre": os.path.basename(nombre_archivo_salida(prefijo_sp=_ultimo_prefijo_sp))}


@app.post("/generar-planilla")
def generar_planilla(body: GenerarPlanillaBody):
    if not _ultimo_resultado or not _ultimo_resultado.filas:
        return {"ok": False, "error": "No hay datos cargados"}
    ruta = body.ruta_destino
    if not ruta.endswith(".xlsx"):
        ruta += ".xlsx"
    try:
        escribir_planilla(_ultimo_resultado.filas, ruta)
        return {"ok": True, "nombre": os.path.basename(ruta)}
    except Exception as e:
        return {"ok": False, "error": str(e)}


# ── Firma Electrónica ────────────────────────────────────────────────────────
@app.post("/firma/set-carpeta")
def set_carpeta(body: SetCarpetaBody):
    firma_controller.set_carpeta(body.ruta)
    return {"ok": True, "ruta": body.ruta}


@app.get("/firma/listar-pdfs")
def listar_pdfs():
    return {
        "ok": True,
        "archivos": firma_controller.listar_pdfs(),
        "carpeta": firma_controller.get_carpeta(),
    }


@app.get("/firma/pdf-page/{nombre}/{pagina}")
def pdf_page(nombre: str, pagina: int):
    import fitz
    try:
        ruta = Path(firma_controller.get_carpeta()) / nombre
        doc = fitz.open(str(ruta))
        total = len(doc)
        if pagina < 0 or pagina >= total:
            doc.close()
            return {"ok": False, "error": "Página fuera de rango"}
        mat = fitz.Matrix(1.5, 1.5)
        pix = doc[pagina].get_pixmap(matrix=mat)
        data = base64.b64encode(pix.tobytes("png")).decode()
        doc.close()
        return {"ok": True, "data": data, "pagina": pagina, "total": total}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.post("/firma/enviar")
def enviar_pdf(body: EnviarPDFBody):
    return firma_controller.enviar_pdf(
        nombre=body.nombre, numero=body.numero, anho=body.anho,
        tipo_contrato=body.tipo_contrato,
        fecha_dia=body.fecha_dia, fecha_mes=body.fecha_mes, fecha_anio=body.fecha_anio,
    )


@app.post("/firma/toggle-auto")
def toggle_auto():
    estado = firma_controller.estado_auto()
    if estado["activo"]:
        firma_controller.detener_auto()
    else:
        firma_controller.iniciar_auto()
    return firma_controller.estado_auto()


@app.get("/firma/estado-auto")
def estado_auto():
    return firma_controller.estado_auto()


@app.get("/firma/log")
def get_log():
    return {"ok": True, "log": firma_controller.get_log()}


@app.get("/firma/buscar-caso/{numero}")
def buscar_caso_ep(numero: str):
    caso = buscar_caso(numero)
    return {"ok": caso is not None, "caso": caso}


@app.post("/firma/agregar-caso")
def agregar_caso_ep(body: AgregarCasoBody):
    try:
        agregar_caso(body.numero, body.anho, body.tipo_contrato,
                     body.fecha_dia, body.fecha_mes, body.fecha_anio)
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.get("/firma/listar-casos")
def listar_casos_ep():
    return {"ok": True, "casos": listar_casos()}


# ── Licencia ─────────────────────────────────────────────────────────────────
@app.get("/licencia")
def get_licencia():
    """El frontend consulta esto al arrancar para saber si puede operar."""
    activo, mensaje = verificar_licencia()
    return {"activo": activo, "mensaje": mensaje}


# ── Main ─────────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 18765
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")
