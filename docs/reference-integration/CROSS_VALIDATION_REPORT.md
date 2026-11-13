# M6 — comparación automatizada ejecutada

`tools/cross-validation/run.py ../loudspeaker-sim-reference` crea un proyecto
Cargo temporal externo, enlaza ambos motores y exporta CSV normalizados.
No cambia dependencias de producción. Fuente ejecutada:
a0e128dfb6c21f68599d74584d4d6af2ec418820. El harness es original; no se copia
implementación ni fixtures externos. La incertidumbre de licencia de la fuente
sigue documentada; se ejecuta sólo para comparación externa local.

Caso propio: sellada 40 L, Fs=30 Hz, Re=6 Ω, Le=1 mH, Qes=0.4, Qms=5,
Vas=80 L, Sd=0.035 m², ρ=1.2041 kg/m³, c=343.21 m/s, 2.83 V RMS,
Rg=0, 201 puntos logarítmicos de 5 a 100 Hz, sin pérdidas de caja.
Mmd principal=Mms referencia−masa de aire de baja frecuencia. La referencia
retorna desplazamiento RMS aunque su tipo dice pico: se convierte ×√2.
Fase principal reducida +90° = presión física respecto a tensión.

| Magnitud | RMS | Máximo absoluto | Sesgo |
|---|---:|---:|---:|
| SPL dB | 0.0244794 | 0.0693185 | -0.0142337 |
| Ze Ω | 0.339605 | 2.26518 | -0.0717502 |
| Fase Ze grados | 0.314601 | 1.03413 | 0.0698912 |
| Excursión pico mm | 0.00116420 | 0.00231895 | -0.000758633 |
| Fase acústica grados | 0.0653583 | 0.269427 | 0.0177352 |
| Retardo ms | 0.00779848 | 0.0380162 | -0.00263688 |

Discrepancias: nuestro pistón aporta resistencia y masa dependientes de frecuencia;
la referencia usa Mms constante y no incluye resistencia radiativa en la ecuación
mecánica sellada. Por ello las condiciones de carga no son exactamente iguales;
no se exige coincidencia arbitraria ni se elimina la carga para igualarlas.
Retardos de extremos usan distintos esquemas de derivación.
Caudal de boca y potencia no están disponibles en esta salida de referencia:
no se fabrican ni se comparan. Los CSV y métricas por banda se conservan en
completion-raw/cross. Sin tolerancias solicitadas, out_of_tolerance es null.

`compare.py` también consume exportaciones externas, calcula RMS, máximo,
sesgo, errores circulares de fase, bandas y recuentos por tolerancia explícita;
rechaza rejillas distintas y datos no finitos. Pruebas Python: errores analíticos
sintéticos conocidos y rechazo de rejillas distintas (2/2 pasan).
Categoría de evidencia: comparación de implementaciones, no validación física
independiente. Hornresp histórico y pruebas analíticas conservan sus categorías.
