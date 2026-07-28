"""
Tests unitarios del transformador — lógica de negocio crítica de JUAN-VIVI.
Cubre todos los casos del dominio: normal, COMPRA PARA, COMUNIDAD, filas vacías.
Ejecutar: pytest juan_vivi/tests/test_transformador.py -v
"""
import pytest
from ..dominio.modelos import FilaEntrada
from ..dominio.transformador import (
    transformar_fila,
    transformar_lote,
    _limpiar_rut,
    _limpiar_nombre,
    _primer_rut,
    _primer_nombre,
    AUTOFIN_RUT,
    AUTOFIN_NOMBRE,
)


# ── Fixture: fila mínima válida ───────────────────────────────────

def fila(numero=1, nombre="JUAN PÉREZ", rut="12345678-9",
         nombre_para=None, rut_para=None, tipo=None, num_fila_excel=2):
    return FilaEntrada(
        numero=numero,
        nombre=nombre,
        rut=rut,
        nombre_para=nombre_para,
        rut_para=rut_para,
        operacion="6000001",
        patente="ABCD12-3",
        tipo=tipo,
        num_fila_excel=num_fila_excel,
    )


# ══════════════════════════════════════════════════════════════════
# Utilidades de limpieza
# ══════════════════════════════════════════════════════════════════

class TestLimpiarRut:
    def test_quita_nbsp(self):
        assert _limpiar_rut("12345678-9\xa0") == "12345678-9"

    def test_quita_espacios(self):
        assert _limpiar_rut("  12345678-9  ") == "12345678-9"

    def test_none_retorna_vacio(self):
        assert _limpiar_rut(None) == ""

    def test_preserva_k(self):
        # Los RUT con dígito verificador K deben quedar en mayúsculas
        assert _limpiar_rut("16993259-k\xa0") == "16993259-K"

    def test_rut_con_barra_comunidad(self):
        # No se parte aquí; _primer_rut lo hace
        resultado = _limpiar_rut("18878668-5/18446556-6")
        assert resultado == "18878668-5/18446556-6"


class TestLimpiarNombre:
    def test_quita_nbsp(self):
        assert _limpiarNombre_wrapper("COMERCIALIZADORA SYS SPA\xa0\xa0") == "COMERCIALIZADORA SYS SPA"

    def test_colapsa_espacios_internos(self):
        assert _limpiarNombre_wrapper("ANA  MARÍA   PÉREZ") == "ANA MARÍA PÉREZ"

    def test_mayusculas(self):
        assert _limpiarNombre_wrapper("juan pérez") == "JUAN PÉREZ"

    def test_none_retorna_vacio(self):
        assert _limpiar_nombre(None) == ""

def _limpiarNombre_wrapper(s):
    return _limpiar_nombre(s)


class TestPrimerRut:
    def test_separa_barra(self):
        assert _primer_rut("18878668-5/18446556-6") == "18878668-5"

    def test_sin_barra_retorna_entero(self):
        assert _primer_rut("12345678-9") == "12345678-9"


class TestPrimerNombre:
    def test_caso_real(self):
        nombre = "BÁRBARA FRANCISCA CABRERA HERRERA Y MICHEL GONZALO RAMÍREZ LARA"
        resultado = _primer_nombre(nombre)
        assert resultado == "BÁRBARA FRANCISCA CABRERA HERRERA Y OTRO"

    def test_sin_separador(self):
        # Si no hay " Y " en el nombre, retorna el nombre + " Y OTRO"
        resultado = _primer_nombre("PATRICIO SOTO VEGA")
        assert resultado == "PATRICIO SOTO VEGA Y OTRO"


# ══════════════════════════════════════════════════════════════════
# Transformación de filas
# ══════════════════════════════════════════════════════════════════

class TestTransformarFilaNormal:
    def test_c1_siempre_autofin(self):
        salida, aviso = transformar_fila(fila())
        assert aviso is None
        assert salida.c1_rut == AUTOFIN_RUT
        assert salida.c1_apellido_paterno == AUTOFIN_NOMBRE

    def test_c2_rut_limpio(self):
        salida, _ = transformar_fila(fila(rut="17674276-3\xa0"))
        assert salida.c2_rut == "17674276-3"

    def test_c2_nombre_en_apellido_paterno(self):
        salida, _ = transformar_fila(fila(nombre="CAMILA ANDREA VARGAS RAMÍREZ"))
        assert salida.c2_apellido_paterno == "CAMILA ANDREA VARGAS RAMÍREZ"

    def test_c2_campos_opcionales_son_none(self):
        salida, _ = transformar_fila(fila())
        assert salida.c2_apellido_materno is None
        assert salida.c2_nombres is None

    def test_columnas_vehiculo_vacias(self):
        salida, _ = transformar_fila(fila())
        assert salida.tasacion_fiscal is None
        assert salida.patente is None
        assert salida.codigo_operacion is None


class TestTransformarFilaCompraPara:
    def _f(self):
        return fila(
            nombre="MARÍA JUANA VIVEROS SANHUEZA",
            rut="15248224-8\xa0",
            nombre_para="LESLIE IVONNE RUIZ VIVEROS",
            rut_para="20708725-4",
            tipo="COMPRA PARA",
        )

    def test_usa_nombre_para(self):
        salida, aviso = transformar_fila(self._f())
        assert aviso is None
        assert salida.c2_apellido_paterno == "LESLIE IVONNE RUIZ VIVEROS"

    def test_usa_rut_para(self):
        salida, _ = transformar_fila(self._f())
        assert salida.c2_rut == "20708725-4"

    def test_ignora_nombre_original(self):
        salida, _ = transformar_fila(self._f())
        assert "VIVEROS SANHUEZA" not in salida.c2_apellido_paterno

    def test_error_si_falta_rut_para(self):
        f = self._f()
        f.rut_para = None
        salida, aviso = transformar_fila(f)
        assert salida is None
        assert aviso is not None
        assert "omitida" in aviso.lower()

    def test_error_si_falta_nombre_para(self):
        f = self._f()
        f.nombre_para = None
        salida, aviso = transformar_fila(f)
        assert salida is None
        assert aviso is not None


class TestTransformarFilaComunidad:
    def _f(self):
        return fila(
            nombre="BÁRBARA FRANCISCA CABRERA HERRERA Y MICHEL GONZALO RAMÍREZ LARA",
            rut="18878668-5/18446556-6",
            tipo="COMUNIDAD",
            num_fila_excel=10,
        )

    def test_nombre_con_y_otro(self):
        salida, aviso = transformar_fila(self._f())
        assert aviso is None
        assert salida.c2_apellido_paterno == "BÁRBARA FRANCISCA CABRERA HERRERA Y OTRO"

    def test_primer_rut(self):
        salida, _ = transformar_fila(self._f())
        assert salida.c2_rut == "18878668-5"


class TestFilasInvalidas:
    def test_fila_nombre_vacio(self):
        salida, aviso = transformar_fila(fila(nombre="", rut=""))
        assert salida is None
        assert aviso is not None

    def test_fila_solo_nbsp(self):
        salida, aviso = transformar_fila(fila(nombre="\xa0 \xa0", rut="\xa0"))
        assert salida is None
        assert aviso is not None

    def test_aviso_incluye_numero_de_fila(self):
        _, aviso = transformar_fila(fila(nombre="", rut="", num_fila_excel=42))
        assert "42" in aviso


# ══════════════════════════════════════════════════════════════════
# Procesamiento en lote
# ══════════════════════════════════════════════════════════════════

class TestTransformarLote:
    def test_consolida_validas_e_ignora_vacias(self):
        filas = [
            fila(numero=1, nombre="ANA PÉREZ",   rut="11111111-1"),
            fila(numero=2, nombre="",             rut=""),          # debe omitirse
            fila(numero=3, nombre="LUIS GARCÍA",  rut="22222222-2"),
        ]
        resultado = transformar_lote(filas)
        assert len(resultado.filas) == 2
        assert len(resultado.avisos) == 1

    def test_lote_vacio(self):
        resultado = transformar_lote([])
        assert resultado.filas == []
        assert resultado.avisos == []

    def test_todos_los_tipos(self):
        filas = [
            fila(numero=1, nombre="NORMAL", rut="1-1"),
            fila(numero=2, nombre="COMPRADOR", rut="2-2",
                 nombre_para="TERCERO", rut_para="3-3", tipo="COMPRA PARA"),
            fila(numero=3, nombre="A Y B", rut="4-4/5-5", tipo="COMUNIDAD"),
        ]
        resultado = transformar_lote(filas)
        assert len(resultado.filas) == 3
        assert resultado.filas[1].c2_rut == "3-3"
        assert resultado.filas[2].c2_apellido_paterno == "A Y OTRO"
