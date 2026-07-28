"""
Persistencia de casos de firma en archivo JSON.

Reemplaza casos_prueba.py — los datos sobreviven entre sesiones.
El archivo se guarda en data/casos.json en la raíz del proyecto.
Los 3 casos de prueba originales se insertan si el archivo no existe.
"""

import json
from pathlib import Path

_RAIZ = Path(__file__).parent.parent.parent
_RUTA_JSON = _RAIZ / "data" / "casos.json"

# Casos de prueba iniciales (migrados desde casos_prueba.py)
_INICIALES: dict[str, dict] = {
    "9739": {
        "numero":        "9739",
        "anho":          "2026",
        "tipo_contrato": "ALZAMIENTO DE PRENDA Y PROHIBICION",
        "fecha_dia":     10,
        "fecha_mes":     7,
        "fecha_anio":    2026,
    },
    "9976": {
        "numero":        "9976",
        "anho":          "2026",
        "tipo_contrato": "MANDATO Y CONTRATO PRIVADO DE MODIFICACION DE PRENDA LEY N 20.190",
        "fecha_dia":     13,
        "fecha_mes":     7,
        "fecha_anio":    2026,
    },
    "10178": {
        "numero":        "10178",
        "anho":          "2026",
        "tipo_contrato": "MANDATO Y CONTRATO PRIVADO DE PRENDA LEY N 20.190",
        "fecha_dia":     15,
        "fecha_mes":     7,
        "fecha_anio":    2026,
    },
}


def _cargar() -> dict:
    if _RUTA_JSON.exists():
        return json.loads(_RUTA_JSON.read_text(encoding="utf-8"))
    return dict(_INICIALES)


def _guardar(casos: dict) -> None:
    _RUTA_JSON.parent.mkdir(parents=True, exist_ok=True)
    _RUTA_JSON.write_text(
        json.dumps(casos, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def buscar_caso(numero: str) -> dict | None:
    """Retorna el caso para el número de repertorio, o None si no existe."""
    return _cargar().get(str(numero).strip())


def agregar_caso(numero: str, anho: str, tipo_contrato: str,
                 fecha_dia: int, fecha_mes: int, fecha_anio: int) -> None:
    """Agrega o sobreescribe un caso en la base de datos JSON."""
    casos = _cargar()
    casos[str(numero).strip()] = {
        "numero":        str(numero).strip(),
        "anho":          str(anho).strip(),
        "tipo_contrato": str(tipo_contrato).strip().upper(),
        "fecha_dia":     int(fecha_dia),
        "fecha_mes":     int(fecha_mes),
        "fecha_anio":    int(fecha_anio),
    }
    _guardar(casos)


def eliminar_caso(numero: str) -> bool:
    """Elimina un caso. Retorna True si existía."""
    casos = _cargar()
    if str(numero).strip() in casos:
        del casos[str(numero).strip()]
        _guardar(casos)
        return True
    return False


def listar_casos() -> list[dict]:
    """Retorna todos los casos ordenados por número."""
    casos = _cargar()
    return sorted(casos.values(), key=lambda c: int(c["numero"]) if c["numero"].isdigit() else 0)
