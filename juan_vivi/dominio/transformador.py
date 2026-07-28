"""
Transformador — lógica de negocio PURA de JUAN-VIVI.
Sin acceso a disco, sin HTTP, sin frameworks.
Todo el conocimiento del dominio AUTOFIN vive aquí (Regla 6 / Regla 58).

Reglas de transformación:
  - Compareciente 1 siempre es AUTOFIN S.A. / 76139506-8
  - Compareciente 2 depende de la col K (tipo):
      · None / vacío   → usar NOMBRE y RUT de la fila
      · 'COMPRA PARA'  → usar NOMBRE PARA y RUT COMPRA PARA
      · 'COMUNIDAD'    → primer nombre del campo + " Y OTRO" / primer RUT (antes del /)
  - Los RUT pueden traer \xa0 u otros espacios — se limpian antes de usar
  - Si NOMBRE y RUT principales están vacíos → aviso y se omite la fila
"""
from .modelos import FilaEntrada, FilaSalida, ResultadoProcesamiento


# ── Constantes de dominio ─────────────────────────────────────────
AUTOFIN_RUT = "76139506-8"
AUTOFIN_NOMBRE = "AUTOFIN S.A."

TIPO_COMPRA_PARA = "COMPRA PARA"
TIPO_COMUNIDAD   = "COMUNIDAD"


# ── Utilidades internas ───────────────────────────────────────────

def _limpiar_rut(rut: str | None) -> str:
    """Elimina espacios (\xa0, normales) y retorna el RUT limpio en mayúsculas."""
    if not rut:
        return ""
    return str(rut).replace("\xa0", "").replace(" ", "").strip().upper()


def _limpiar_nombre(nombre: str | None) -> str:
    """Elimina espacios extra y \xa0; retorna en mayúsculas."""
    if not nombre:
        return ""
    limpio = str(nombre).replace("\xa0", "").strip()
    # Colapsar espacios internos múltiples
    import re
    limpio = re.sub(r" {2,}", " ", limpio)
    return limpio.upper()


def _primer_rut(rut_raw: str) -> str:
    """
    Para caso COMUNIDAD el RUT viene como '18878668-5/18446556-6'.
    Retorna solo el primero.
    """
    return rut_raw.split("/")[0].strip()


def _primer_nombre(nombre_raw: str) -> str:
    """
    Para caso COMUNIDAD el nombre viene como 'BÁRBARA FRANCISCA CABRERA HERRERA Y MICHEL GONZALO RAMÍREZ LARA'.
    Busca la primera ocurrencia de ' Y ' (con espacios) para separar.
    Retorna 'BÁRBARA FRANCISCA CABRERA HERRERA Y OTRO'.
    Si no hay ' Y ' en el nombre, retorna el nombre completo + ' Y OTRO'.
    """
    separador = " Y "
    idx = nombre_raw.upper().find(separador)
    if idx != -1:
        primer = nombre_raw[:idx].strip()
    else:
        primer = nombre_raw.strip()
    return primer.upper() + " Y OTRO"


def _es_vacia(fila: FilaEntrada) -> bool:
    """True si la fila no tiene nombre ni RUT principal."""
    nombre_limpio = _limpiar_nombre(fila.nombre)
    rut_limpio    = _limpiar_rut(fila.rut)
    return not nombre_limpio and not rut_limpio


# ── Transformación principal ──────────────────────────────────────

def transformar_fila(fila: FilaEntrada) -> tuple[FilaSalida | None, str | None]:
    """
    Transforma una FilaEntrada en FilaSalida según las reglas de dominio.

    Retorna (FilaSalida, None) si fue exitoso,
            (None, aviso_str) si la fila se debe omitir con aviso.
    """
    # Validación: fila completamente vacía
    if _es_vacia(fila):
        aviso = f"Fila {fila.num_fila_excel}: sin nombre ni RUT — omitida"
        return None, aviso

    tipo = (fila.tipo or "").strip().upper()

    # ── Caso COMPRA PARA ─────────────────────────────────────────
    if tipo == TIPO_COMPRA_PARA:
        rut_c2    = _limpiar_rut(fila.rut_para)
        nombre_c2 = _limpiar_nombre(fila.nombre_para)
        if not rut_c2 or not nombre_c2:
            aviso = (f"Fila {fila.num_fila_excel}: tipo 'COMPRA PARA' pero "
                     f"NOMBRE PARA o RUT PARA están vacíos — omitida")
            return None, aviso

    # ── Caso COMUNIDAD ───────────────────────────────────────────
    elif tipo == TIPO_COMUNIDAD:
        rut_raw    = _limpiar_rut(fila.rut)
        nombre_raw = _limpiar_nombre(fila.nombre)
        rut_c2    = _primer_rut(rut_raw)
        nombre_c2 = _primer_nombre(nombre_raw)

    # ── Caso normal ──────────────────────────────────────────────
    else:
        rut_c2    = _limpiar_rut(fila.rut)
        nombre_c2 = _limpiar_nombre(fila.nombre)
        if not rut_c2 or not nombre_c2:
            aviso = f"Fila {fila.num_fila_excel}: RUT o nombre vacío — omitida"
            return None, aviso

    salida = FilaSalida(
        c1_rut=AUTOFIN_RUT,
        c1_apellido_paterno=AUTOFIN_NOMBRE,
        c2_rut=rut_c2,
        c2_apellido_paterno=nombre_c2,
    )
    return salida, None


def transformar_lote(filas_entrada: list) -> ResultadoProcesamiento:
    """
    Procesa una lista de FilaEntrada y retorna un ResultadoProcesamiento
    con todas las FilaSalida y los avisos de filas omitidas.
    """
    resultado = ResultadoProcesamiento()

    for fila in filas_entrada:
        salida, aviso = transformar_fila(fila)
        if aviso:
            resultado.avisos.append(aviso)
        if salida:
            resultado.filas.append(salida)

    return resultado
