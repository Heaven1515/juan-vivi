"""
Casos de prueba hardcodeados para la fase de validación OCR + SIGN+.

Estos 3 repertorios se usan para probar el flujo completo sin BD.
Cuando el OCR detecte uno de estos números, se toman estos datos.

BORRAR este archivo cuando se implemente la BD real.
"""

# Clave: número de repertorio (str sin puntos)
# Valor: datos para el formulario SIGN+
CASOS_PRUEBA: dict[str, dict] = {
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


def buscar_caso(numero_repertorio: str) -> dict | None:
    """
    Retorna los datos hardcodeados para el repertorio indicado,
    o None si no está en los casos de prueba.
    """
    return CASOS_PRUEBA.get(str(numero_repertorio).strip())
