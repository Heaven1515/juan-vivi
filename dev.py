"""
dev.py — Lanzador de desarrollo de JUAN-VIVI.
Abre el dashboard en una ventana nativa con DevTools activadas.
Python expone la API al JS via pywebview.

Uso:
  python dev.py
"""
import sys
import os
import json
import threading

sys.stdout.reconfigure(encoding='utf-8', errors='replace')
sys.stderr.reconfigure(encoding='utf-8', errors='replace')

# Ruta raíz del proyecto
RAIZ = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, RAIZ)

import re
import webview
from juan_vivi.infraestructura.lector import leer_multiples
from juan_vivi.infraestructura.escritor import escribir_planilla, nombre_archivo_salida
from juan_vivi.dominio.transformador import transformar_lote
from juan_vivi.infraestructura import firma_controller


def _extraer_numero_sp(rutas: list) -> str:
    """
    Extrae el número SP del nombre de archivo sin importar su posición.
    Estrategia: eliminar patrones de fecha (15-07-2026, 2026-07-15)
    y quedarse con el primer número restante.

    Ej: "3206 AUTOFIN SP 15-07-2026.xlsx"  → "3206"
    Ej: "AUTOFIN 3206 SP 15-07-2026.xlsx"  → "3206"
    Ej: "AUTOFIN SP 15-07-2026 3206.xlsx"  → "3206"
    Ej: dos archivos 3206 + 3217           → "3206_3217"
    """
    numeros = []
    for ruta in rutas:
        nombre = os.path.splitext(os.path.basename(ruta))[0]
        # Eliminar fechas en formato dd-mm-aaaa, dd/mm/aaaa, aaaa-mm-dd, etc.
        sin_fecha = re.sub(r'\d{1,2}[-/]\d{1,2}[-/]\d{4}', '', nombre)
        sin_fecha = re.sub(r'\d{4}[-/]\d{1,2}[-/]\d{1,2}', '', sin_fecha)
        m = re.search(r'\d+', sin_fecha)
        if m:
            numeros.append(m.group(0))
    return "_".join(numeros) if numeros else ""


# ══════════════════════════════════════════════════════════════════
# API Python expuesta al JavaScript del dashboard
# ══════════════════════════════════════════════════════════════════

class ApiJuanVivi:
    """
    Cada método público aquí es invocable desde JS como:
      await window.pywebview.api.nombre_metodo(args)
    """

    def __init__(self):
        self._ventana = None   # se asigna después de crear la ventana

    def _js(self, codigo: str):
        """Ejecuta código JS en la ventana."""
        if self._ventana:
            self._ventana.evaluate_js(codigo)

    def _notificar(self, mensaje: str, tipo: str = "none"):
        """Llama al toast del dashboard."""
        msg_safe = mensaje.replace("'", "\\'")
        self._js(f"mostrarToast('{msg_safe}', '{tipo}')")

    # ── Selector de archivos Excel ────────────────────────────────

    def abrir_excel(self) -> dict:
        """
        Abre el diálogo de selección de archivos Excel.
        Retorna { ok: bool, rutas: list[str] }
        """
        rutas = self._ventana.create_file_dialog(
            dialog_type=webview.OPEN_DIALOG,
            allow_multiple=True,
            file_types=('Archivos Excel (*.xlsx)',),
        )
        if not rutas:
            return {"ok": False, "rutas": []}
        return {"ok": True, "rutas": list(rutas)}

    # ── Lectura + transformación + carga en tabla ─────────────────

    def cargar_excel(self, rutas: list) -> dict:
        """
        Lee los Excel indicados, transforma las filas y actualiza el dashboard.
        Retorna { ok: bool, filas: int, avisos: list[str] }
        """
        if not rutas:
            self._notificar("No se seleccionaron archivos", "err")
            return {"ok": False}

        # Leer
        filas_entrada, errores_lectura = leer_multiples(rutas)
        for err in errores_lectura:
            self._notificar(err, "err")

        if not filas_entrada:
            self._notificar("No se pudieron leer los archivos", "err")
            return {"ok": False}

        # Transformar
        resultado = transformar_lote(filas_entrada)

        # Preparar filas para la tabla del dashboard
        filas_json = [
            {
                "n":      str(idx + 1),
                "nombre": f.c2_apellido_paterno,
                "rut":    f.c2_rut,
            }
            for idx, f in enumerate(resultado.filas)
        ]

        # Nombres de archivos para mostrar en el dashboard
        nombres = [os.path.basename(r) for r in rutas]
        nombre_display = " + ".join(nombres)

        # Llamar a la API del dashboard para actualizar la UI
        filas_str = json.dumps(filas_json, ensure_ascii=False).replace("'", "\\'")
        avisos_str = json.dumps(resultado.avisos, ensure_ascii=False).replace("'", "\\'")
        nombre_safe = nombre_display.replace("'", "\\'")

        self._js(f"window.JVAPI.cargarDatos('{nombre_safe}', JSON.parse('{filas_str}'), JSON.parse('{avisos_str}'))")

        # Guardar resultado en estado para generar después
        self._ultimo_resultado = resultado
        self._ultimo_prefijo_sp = _extraer_numero_sp(rutas)

        return {
            "ok":     True,
            "filas":  len(resultado.filas),
            "avisos": resultado.avisos,
        }

    # ── Generación de la planilla de salida ───────────────────────

    def generar_planilla(self) -> dict:
        """
        Genera el Excel de salida con los datos ya cargados.
        Abre el diálogo de guardar para elegir destino.
        """
        if not hasattr(self, '_ultimo_resultado') or not self._ultimo_resultado.filas:
            self._notificar("Primero carga un Excel", "err")
            return {"ok": False}

        # Diálogo de guardar
        prefijo = getattr(self, '_ultimo_prefijo_sp', '')
        destino = self._ventana.create_file_dialog(
            dialog_type=webview.SAVE_DIALOG,
            save_filename=os.path.basename(nombre_archivo_salida(prefijo_sp=prefijo)),
            file_types=('Archivos Excel (*.xlsx)',),
        )
        if not destino:
            return {"ok": False}

        # pywebview a veces retorna lista, a veces str
        ruta_final = destino[0] if isinstance(destino, (list, tuple)) else destino
        if not ruta_final.endswith('.xlsx'):
            ruta_final += '.xlsx'

        try:
            escribir_planilla(self._ultimo_resultado.filas, ruta_final)
            nombre_safe = os.path.basename(ruta_final).replace("'", "\\'")
            self._js(f"window.JVAPI.planillaGenerada('{nombre_safe}')")
            return {"ok": True, "ruta": ruta_final}
        except Exception as e:
            self._notificar(f"Error al guardar: {e}", "err")
            return {"ok": False}

    # ── Abrir carpeta en el explorador ────────────────────────────

    def abrir_carpeta(self, ruta: str = "") -> None:
        """Abre el explorador de Windows en la ruta indicada."""
        import subprocess
        destino = ruta if ruta and os.path.exists(ruta) else os.path.expanduser("~\\Desktop")
        subprocess.Popen(f'explorer "{destino}"')

    # ══════════════════════════════════════════════════════════════
    # API Firma Electrónica
    # ══════════════════════════════════════════════════════════════

    def firma_seleccionar_carpeta(self) -> dict:
        """Abre diálogo para elegir la carpeta del escáner."""
        ruta = self._ventana.create_file_dialog(
            dialog_type=webview.FOLDER_DIALOG,
        )
        if not ruta:
            return {"ok": False, "ruta": firma_controller.get_carpeta()}
        carpeta = ruta[0] if isinstance(ruta, (list, tuple)) else ruta
        firma_controller.set_carpeta(carpeta)
        return {"ok": True, "ruta": carpeta}

    def firma_listar_pdfs(self) -> dict:
        """Lista los PDFs disponibles en la carpeta del escáner."""
        archivos = firma_controller.listar_pdfs()
        carpeta  = firma_controller.get_carpeta()
        return {"ok": True, "archivos": archivos, "carpeta": carpeta}

    def firma_leer_pdf(self, nombre: str) -> dict:
        """
        Aplica OCR al PDF y retorna datos extraídos + datos hardcodeados si existen.
        """
        try:
            resultado = firma_controller.leer_pdf(nombre)
            return {"ok": True, **resultado}
        except Exception as e:
            self._notificar(f"Error OCR: {e}", "err")
            return {"ok": False, "error": str(e)}

    def firma_enviar_pdf(self, nombre: str, numero: str, anho: str,
                         tipo_contrato: str, fecha_dia: int,
                         fecha_mes: int, fecha_anio: int) -> dict:
        """Envía el PDF a SIGN+ con los datos confirmados."""
        self._notificar(f"Enviando {nombre} a SIGN+…", "none")
        resultado = firma_controller.enviar_pdf(
            nombre=nombre, numero=numero, anho=anho,
            tipo_contrato=tipo_contrato, fecha_dia=fecha_dia,
            fecha_mes=fecha_mes, fecha_anio=fecha_anio,
        )
        if resultado["estado"] == "ok":
            self._notificar(f"Enviado correctamente: {nombre}", "ok")
        else:
            self._notificar(f"Error: {resultado['mensaje']}", "err")
        return resultado

    def firma_toggle_auto(self) -> dict:
        """Activa o desactiva el modo automático del vigilador."""
        estado = firma_controller.estado_auto()
        try:
            if estado["activo"]:
                firma_controller.detener_auto()
                self._notificar("Modo automático detenido", "none")
            else:
                firma_controller.iniciar_auto()
                self._notificar("Modo automático activado", "ok")
        except Exception as e:
            self._notificar(str(e), "err")
        return firma_controller.estado_auto()

    def firma_estado_auto(self) -> dict:
        """Retorna estado del modo automático + contadores."""
        return firma_controller.estado_auto()

    def firma_get_log(self) -> dict:
        """Retorna el log de envíos de la sesión."""
        return {"ok": True, "log": firma_controller.get_log()}

    def firma_cargar_visor_pdf(self, nombre: str) -> dict:
        """
        Renderiza todas las páginas del PDF y las envía al visor una a una
        via window.JVAPI.agregarPaginaVisor() para no saturar el bridge.
        """
        import fitz
        import base64
        from pathlib import Path
        try:
            ruta = Path(firma_controller.get_carpeta()) / nombre
            doc  = fitz.open(str(ruta))
            total = len(doc)
            mat   = fitz.Matrix(1.5, 1.5)   # ~108 DPI

            self._js(f"window.JVAPI.iniciarVisorPDF({total})")

            for i in range(total):
                pix  = doc[i].get_pixmap(matrix=mat)
                data = base64.b64encode(pix.tobytes("png")).decode()
                self._js(f"window.JVAPI.agregarPaginaVisor({i + 1},{total},'{data}')")

            doc.close()
            return {"ok": True, "total": total}
        except Exception as e:
            self._js("window.JVAPI.errorVisorPDF()")
            return {"ok": False, "error": str(e)}

    def firma_buscar_caso(self, numero: str) -> dict:
        """Busca un caso en la base de datos por número de repertorio."""
        from juan_vivi.infraestructura.base_casos import buscar_caso
        caso = buscar_caso(numero)
        return {"ok": caso is not None, "caso": caso}

    def firma_agregar_caso(self, numero: str, anho: str, tipo_contrato: str,
                           fecha_dia: int, fecha_mes: int, fecha_anio: int) -> dict:
        """Agrega o actualiza un caso en la base de datos local (JSON)."""
        try:
            from juan_vivi.infraestructura.base_casos import agregar_caso
            agregar_caso(numero, anho, tipo_contrato,
                         int(fecha_dia), int(fecha_mes), int(fecha_anio))
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    def firma_listar_casos(self) -> dict:
        """Lista todos los casos de la base de datos local."""
        from juan_vivi.infraestructura.base_casos import listar_casos
        return {"ok": True, "casos": listar_casos()}

    def firma_abrir_carpeta_escaner(self) -> None:
        """Abre el explorador en la carpeta del escáner."""
        import subprocess
        carpeta = firma_controller.get_carpeta()
        if os.path.exists(carpeta):
            subprocess.Popen(f'explorer "{carpeta}"')
        else:
            self._notificar(f"Carpeta no accesible: {carpeta}", "err")


# ══════════════════════════════════════════════════════════════════
# Entrada principal
# ══════════════════════════════════════════════════════════════════

def main():
    api = ApiJuanVivi()
    ruta_html = os.path.join(RAIZ, "dashboard.html")

    print(f"\n[JUAN-VIVI dev] Abriendo {ruta_html}")
    print("[JUAN-VIVI dev] DevTools: F12 o clic derecho → Inspect\n")

    ventana = webview.create_window(
        title="JUAN-VIVI · 33ª Notaría · Dev",
        url=f"file:///{ruta_html.replace(os.sep, '/')}",
        js_api=api,
        width=1440,
        height=900,
        min_size=(1024, 600),
    )
    api._ventana = ventana

    webview.start(debug=True)


if __name__ == "__main__":
    main()
