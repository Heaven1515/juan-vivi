"""
Vigilador de carpeta del escáner — polling cada 8 segundos.

Idéntico al de SAGE: más robusto que watchdog en rutas UNC de Windows.
Detecta PDFs nuevos y espera 12s antes de procesarlos (tiempo para que
el escáner termine de copiar el archivo completo).
"""

import logging
import os
import threading

logger = logging.getLogger(__name__)

RUTA_SCANNER   = r"\\Desktop-14rq1mp\escaner riho 550"
INTERVALO_S    = 8    # segundos entre revisiones
ESPERA_COPIA_S = 12   # segundos de espera antes de procesar un PDF nuevo


class _HiloVigilador(threading.Thread):
    """Hilo daemon que vigila la carpeta y encola PDFs nuevos."""

    def __init__(self, ruta: str, callback):
        super().__init__(daemon=True)
        self._ruta     = ruta
        self._callback = callback
        self._detener  = threading.Event()
        self._vistos   = set()

    def run(self) -> None:
        logger.info("Vigilador iniciado en: %s", self._ruta)
        while not self._detener.is_set():
            try:
                self._revisar_carpeta()
            except Exception as exc:
                logger.error("Error revisando carpeta: %s", exc)
            self._detener.wait(timeout=INTERVALO_S)
        logger.info("Vigilador detenido")

    def detener(self) -> None:
        self._detener.set()

    def _revisar_carpeta(self) -> None:
        try:
            archivos = {
                f for f in os.listdir(self._ruta)
                if f.lower().endswith(".pdf")
            }
        except OSError as exc:
            logger.warning("No se pudo leer la carpeta del escáner: %s", exc)
            return

        nuevos = archivos - self._vistos
        for nombre in sorted(nuevos):
            self._vistos.add(nombre)
            ruta_completa = os.path.join(self._ruta, nombre)
            logger.info(
                "PDF nuevo: %s — esperando %ds antes de procesar...",
                nombre, ESPERA_COPIA_S,
            )
            timer = threading.Timer(
                ESPERA_COPIA_S, self._callback, args=[ruta_completa]
            )
            timer.daemon = True
            timer.start()


# ── Instancia global ──────────────────────────────────────────────

_hilo: _HiloVigilador | None = None


def iniciar_vigilancia(callback, ruta: str | None = None) -> None:
    """Arranca el hilo. Lanza ValueError si ya está activo."""
    global _hilo
    if _hilo is not None and _hilo.is_alive():
        raise ValueError("El vigilador ya está activo")
    _hilo = _HiloVigilador(ruta or RUTA_SCANNER, callback)
    _hilo.start()


def detener_vigilancia() -> None:
    """Detiene el hilo limpiamente."""
    global _hilo
    if _hilo is not None:
        _hilo.detener()
        _hilo.join(timeout=ESPERA_COPIA_S + 2)
        _hilo = None


def esta_activo() -> bool:
    return _hilo is not None and _hilo.is_alive()
