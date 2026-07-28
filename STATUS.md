# JUAN-VIVI — Estado del Proyecto
> Actualizado: 2026-07-27 | Protocolo aplicado: v6.3

---

## Qué es este proyecto

**JUAN-VIVI** es un procesador batch de Excel para **AUTOFIN** (empresa de financiamiento automotriz).
Toma planillas de entrada con carteras de alzamientos, las procesa y genera un archivo resultado consolidado.
Es una app **100 % local, sin estado en nube**, clasificada como **Patrón A** en el protocolo v6.3.

---

## Estado actual

| Ítem | Estado |
|---|---|
| Git inicializado | ✅ (commit inicial) |
| Protocolo v6.3 leído y aplicado | ✅ |
| `.gitignore` configurado | ✅ |
| Dashboard HTML (UI standalone) | ✅ `dashboard.html` |
| Estructura de carpetas | ✅ `juan_vivi/` |
| `dominio/modelos.py` | ✅ FilaEntrada, FilaSalida, ResultadoProcesamiento |
| `dominio/transformador.py` | ✅ Lógica pura — 31 tests, todos pasando |
| `infraestructura/lector.py` | ✅ Lee AUTOFIN SP, multi-archivo |
| `infraestructura/escritor.py` | ✅ Genera planilla masivos con formato |
| `main.py` (CLI) | ✅ Funcional — probado con archivos reales |
| Tests | ✅ 31 tests unitarios del transformador |
| Integración UI ↔ Python | ❌ Pendiente (esperando decisión de UI) |
| Empaquetado (.exe) | ❌ Pendiente |

**Fase actual: Procesador core completo y probado. Pendiente: integración con UI.**

---

## Archivos presentes en el repo (no commiteados — ignorados correctamente)

| Archivo | Rol |
|---|---|
| `3206 AUTOFIN SP 15-07-2026.xlsx` | Planilla de entrada real (84 filas, 7 casos COMPRA PARA) |
| `3217 AUTOFIN SP 22-07-2026.xlsx` | Planilla de entrada real (55 filas, 1 caso COMUNIDAD) |
| `planilla masivos.xlsx` | Plantilla de entrada vacía |
| `planilla_masivos_resultado.xlsx` | Ejemplo de salida — fuente de verdad del formato |
| `Protocolo_v6.3.docx` | Protocolo personal completo |
| `Dashboard Notaría Módulos.zip` | Diseño exportado del dashboard |

---

## Reglas de negocio implementadas

| Col K | Comportamiento |
|---|---|
| vacío (normal) | Compareciente 2 = NOMBRE / RUT (sin \xa0) |
| `COMPRA PARA` | Compareciente 2 = NOMBRE PARA / RUT COMPRA PARA |
| `COMUNIDAD` | Compareciente 2 = primer nombre + " Y OTRO" / primer RUT (antes del /) |
| Fila vacía (sin nombre ni RUT) | Aviso y omisión silenciosa |

Compareciente 1 SIEMPRE: `76139506-8` / `AUTOFIN S.A.`

---

## Prueba real (2026-07-27)

```
python main.py --entrada "3206 AUTOFIN SP..." "3217 AUTOFIN SP..." --salida .
→ 139 filas leídas, 139 filas exportadas, 0 avisos
→ Archivo: planilla_masivos_2026-07-27_16-38.xlsx
```

---

## Estructura de carpetas

```
JUAN-VIVI/
├── juan_vivi/
│   ├── dominio/
│   │   ├── modelos.py          ← FilaEntrada, FilaSalida, ResultadoProcesamiento
│   │   └── transformador.py    ← lógica pura + 31 tests
│   ├── infraestructura/
│   │   ├── lector.py           ← lee Excel AUTOFIN SP
│   │   └── escritor.py         ← genera planilla masivos
│   └── tests/
│       └── test_transformador.py
├── main.py                     ← CLI funcional
├── dashboard.html              ← UI standalone (diseño v6.3)
└── requirements.txt
```

---

## Lo que falta (por orden de prioridad)

### Paso 3 — Integración UI ↔ Python
Javier debe confirmar qué tipo de UI:
- **pywebview** — abre `dashboard.html` en una ventana nativa, Python es el backend
- **Flask local** — servidor HTTP en 127.0.0.1, browser como UI
- **Tauri** — app completa con sidecar Python (más complejo)

La decisión afecta cómo `dashboard.html` llama a `main.py`.
El HTML ya tiene `window.JVAPI` preparado para recibir datos del backend.

### Paso 4 — Empaquetado
Según la decisión anterior:
- pywebview / Flask → PyInstaller `--onefile`
- Tauri → orden de la Regla 66 del protocolo

---

## Reglas críticas recordatorio

1. Todo en español (variables, funciones, comentarios)
2. Máximo 600 líneas por archivo, 40 líneas por función
3. Los `.xlsx` nunca van al repo
4. No inventar lógica de negocio
5. Un módulo completo antes del siguiente (Mandamiento X)
