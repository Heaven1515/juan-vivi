"""
JUAN-VIVI — Punto de entrada CLI.
Uso:
  python main.py --entrada "3206 AUTOFIN SP.xlsx" "3217 AUTOFIN SP.xlsx"
  python main.py --entrada *.xlsx --salida ./resultado

Cuando la UI esté lista, este módulo será invocado desde ella.
"""
import sys
import os
import argparse

# Forzar UTF-8 en la consola de Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    sys.stderr.reconfigure(encoding='utf-8', errors='replace')

# Permite ejecutar desde la raíz del proyecto
sys.path.insert(0, os.path.dirname(__file__))

from juan_vivi.infraestructura.lector import leer_multiples
from juan_vivi.infraestructura.escritor import escribir_planilla, nombre_archivo_salida
from juan_vivi.dominio.transformador import transformar_lote


def main():
    parser = argparse.ArgumentParser(
        description="JUAN-VIVI — Procesador de planillas AUTOFIN"
    )
    parser.add_argument(
        "--entrada", "-e",
        nargs="+",
        required=True,
        metavar="ARCHIVO.xlsx",
        help="Uno o más archivos Excel AUTOFIN SP de entrada",
    )
    parser.add_argument(
        "--salida", "-s",
        default=".",
        metavar="CARPETA",
        help="Carpeta de destino para la planilla generada (default: .)",
    )

    args = parser.parse_args()

    sep = "-" * 50
    print(f"\n{sep}")
    print("  JUAN-VIVI - Procesador AUTOFIN")
    print(f"{sep}\n")

    # ── 1. Lectura ────────────────────────────────────────────────
    print(f"Leyendo {len(args.entrada)} archivo(s)…")
    filas_entrada, errores_lectura = leer_multiples(args.entrada)

    for err in errores_lectura:
        print(f"  [!] {err}")

    if not filas_entrada and errores_lectura:
        print("\nNo se pudieron leer los archivos. Revisa los errores anteriores.")
        sys.exit(1)

    print(f"  → {len(filas_entrada)} filas leídas en total\n")

    # ── 2. Transformación ─────────────────────────────────────────
    print("Procesando filas…")
    resultado = transformar_lote(filas_entrada)

    if resultado.avisos:
        print(f"\n  Avisos ({len(resultado.avisos)} filas ignoradas):")
        for aviso in resultado.avisos:
            print(f"    · {aviso}")

    print(f"\n  → {len(resultado.filas)} filas válidas para exportar\n")

    if not resultado.filas:
        print("Sin filas válidas para generar la planilla. Fin.")
        sys.exit(0)

    # ── 3. Escritura ──────────────────────────────────────────────
    ruta_salida = nombre_archivo_salida(args.salida)
    print(f"Generando planilla…")
    escribir_planilla(resultado.filas, ruta_salida)
    print(f"  → Archivo generado: {ruta_salida}\n")

    print(f"{sep}")
    print(f"  Listo. {len(resultado.filas)} filas exportadas.")
    if resultado.avisos:
        print(f"  {len(resultado.avisos)} fila(s) ignorada(s) - ver avisos arriba.")
    print(f"{sep}\n")


if __name__ == "__main__":
    main()
