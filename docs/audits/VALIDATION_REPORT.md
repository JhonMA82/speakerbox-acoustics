# Validación antes/después

Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b`.
Implementación final: `b462560`. Misma calibración y datos Hornresp 60.70.

Reproducción: `. "$HOME/.cargo/env"` y
`python docs/audits/measure.py [ruta-repo]`. El script reutiliza los
constructores de los tests de producción, genera un ejemplo temporal y lo
elimina. Se ejecutó contra un worktree de la base y contra la rama corregida.
CSV con nueve decimales en `raw/metrics-before.csv` y `raw/metrics-after.csv`.
No se generan nuevos oráculos. Bandas: 10 Hz–1 kHz para OD/cajas/BP;
todo el export para TH y las tres bocinas normales. Esta tabla de bocina 1
usa el TSV completo (533 puntos); el test histórico usa también CSV de 44.

## SPL, impedancia y excursión

Cada celda es RMS antes → después. Los CSV incluyen los máximos puntuales.
Los 15 casos conservan sus tolerancias de producción; los máximos de Ze/Xd
siguen por debajo de 35 %/40 % y el SPL máximo por debajo de 3,5 dB.
Xd excluye valores de referencia <=0.02 mm y divide por los puntos restantes;
esto explica la diferencia con las cifras TH/BP de reportes históricos.

| Caso | N | SPL RMS (dB) | Ze RMS (%) | Xd RMS (%) |
|---|---:|---:|---:|---:|
| OD-A | 323 | 0.845939 → 0.845939 | 4.702638 → 4.702638 | 4.033761 → 4.033761 |
| OD1-B | 323 | 0.845018 → 0.845018 | 4.682380 → 4.682380 | 4.098343 → 4.098343 |
| OD2-C | 323 | 0.853211 → 0.853211 | 4.786858 → 4.786858 | 4.310839 → 4.310839 |
| TH1 | 533 | 0.526769 → 0.526771 | 7.029530 → 7.029530 | 5.678048 → 5.678048 |
| TH2 | 533 | 0.515683 → 0.515684 | 7.059529 → 7.059529 | 5.930873 → 5.930873 |
| TH1-S2S3 | 533 | 0.547726 → 0.547725 | 6.727089 → 6.727089 | 4.591618 → 4.591618 |
| SEALED-A | 323 | 0.000000 → 0.000000 | 0.000003 → 0.000003 | 0.000396 → 0.000396 |
| VENTED-A | 323 | 0.061028 → 0.061028 | 0.738564 → 0.738564 | 0.362479 → 0.362479 |
| PR-A | 323 | 0.375292 → 0.375292 | 2.491700 → 2.491700 | 1.006330 → 1.006330 |
| BP4-A | 323 | 0.214413 → 0.214413 | 0.236877 → 0.236877 | 0.460646 → 0.460646 |
| BP6P-A | 323 | 0.217976 → 0.217976 | 0.213850 → 0.213850 | 0.569148 → 0.569148 |
| BP6S-A | 323 | 0.256475 → 0.256475 | 1.439470 → 1.439470 | 3.959352 → 3.959352 |
| HORN-1 | 533 | 1.914094 → 1.914094 | 6.417324 → 6.417324 | 9.337784 → 9.337784 |
| HORN-2 | 533 | 1.942630 → 1.942630 | 5.977589 → 5.977589 | 7.142783 → 7.142783 |
| HORN-3 | 533 | 1.926990 → 1.926990 | 6.171664 → 6.171664 | 7.760350 → 7.760350 |

Las diferencias absolutas máximas entre salidas de base y rama (estos casos)
son 0.000722 dB de SPL, 6e-08 Ω de Ze y 3.34e-10 mm de
excursión. Son cambios numéricos de matrices exactas/normalización; las
correcciones físicas no se ejercitan por completo en los oráculos existentes
(p.ej. no hay Rserie ni puerto trasero OD en sus fixtures). Por eso las
regresiones analíticas son necesarias. Los deltas están en `raw/output-deltas.csv`.

## Magnitudes ampliadas: diagnóstico, sin umbral de aprobación inventado

Fase eléctrica: diferencia circular contra ZePhase. Fase acústica: se suma
+90° a la referencia reducida del motor antes de comparar con UPhase del
export. Retardo en ms, tal como se presenta en ambas salidas, sin ajustar un
retardo común para forzar coincidencia. Potencia de referencia calculada
únicamente donde hay datos: `Efficiency(%) * Pin(W) / 100`.

| Caso | Fase Ze RMS (°), después | Fase acústica RMS (°), antes → después | Retardo RMS (ms), antes → después | Potencia RMS relativa (%), después | Eficiencia RMS (puntos porcentuales), después |
|---|---:|---:|---:|---:|---:|
| OD-A | 2.526 | 53.896 → 19.715 | 10.48346 → 3.75476 | 323.346 | 75.658 |
| OD1-B | 2.504 | 30.051 → 19.739 | 13.62848 → 3.25035 | 323.075 | 65.169 |
| OD2-C | 2.552 | 60.603 → 20.515 | 31.77169 → 4.10555 | 322.914 | 61.388 |
| TH1 | 3.991 | 4.450 → 4.450 | 13.07135 → 9.47423 | 215.969 | 41.283 |
| TH2 | 4.073 | 4.492 → 4.492 | 11.35363 → 8.33203 | 214.943 | 40.547 |
| TH1-S2S3 | 3.551 | 4.527 → 4.527 | 7.33934 → 5.25032 | 217.028 | 21.882 |
| SEALED-A | 0.000 | 0.000 → 0.000 | 0.01654 → 0.00011 | 0.003 | 0.000 |
| VENTED-A | 0.426 | 0.359 → 0.359 | 0.17501 → 0.11934 | 57.556 | 1.038 |
| PR-A | 1.388 | 2.101 → 2.101 | 0.67777 → 0.61028 | 52.954 | 0.613 |
| HORN-1 | 3.514 | 5.964 → 5.964 | 1.84415 → 1.85928 | 133.835 | 41.327 |
| HORN-2 | 3.378 | 6.129 → 6.129 | 1.79487 → 1.80601 | 141.268 | 30.842 |
| HORN-3 | 3.458 | 6.028 → 6.028 | 1.68221 → 1.66590 | 140.975 | 37.869 |

**Estas cifras no certifican potencia, eficiencia, fase acústica ni retardo.**
La gran desviación de potencia/rendimiento ya existía: la calibración de SPL
no es un modelo energético general. La corrección de fase combinada OD mejora
sus tres casos, pero deja ~20° RMS. La derivada centrada reduce varios errores
de retardo y aumenta ligeramente otros de bocina normal: una derivada más
coherente no implica automáticamente menor diferencia contra otra aplicación.
No se han reajustado calibraciones ni tolerancias.

Para BP faltan fases, retardo, potencia y eficiencia en los exports disponibles:
NaN en el CSV significa **sin referencia**, nunca error cero ni aprobación.
No existen referencias permanentes de velocidad de puerto.

## Invariantes y controles ejecutados

- 23 regresiones: fallan todas sobre la base y pasan en la rama.
- Matrices exactas frente a RK4 independiente (Custom constante), 36 casos
  con áreas 1e-4–1 m², longitudes 0.001–2 m y frecuencias 1 Hz–10 kHz;
  error normalizado <3e-6, determinante unitario <1e-12.
- Balance de red y pasividad: Rserie de 0 a 1e10 Pa·s/m³; potencia eléctrica
  menos pérdidas coincide con radiación para una boca a 2π sin calibración.
  Soluciones complejas BP comprueban además balance absorbido por la red.
- Polos del paralelo, abiertos/cortos, escalas 1e-200–1e200 y amortiguamiento
  tendiendo a cero; sin sustitución por un corto ficticio.
- Inversión de polaridad: SPL/Ze/excursión conservados y fase invertida;
  potencia eléctrica y eficiencia invariantes en los casos comprobados.
- Rejilla no uniforme, fase cuadrática de derivada exacta, saltos ±180°, orden
  de entrada, duplicados y cancelación exacta explícita.
- Fasores de puerto/PR, ausencia de salida en cámara sellada, equivalencia
  preparado/no preparado y recálculo tras cambio paramétrico.
- Suite final: 192 pruebas (123 unitarias, 63 de integración, 6 doc-tests),
  tanto debug como release. Fmt, clippy estricto, rustdoc estricto, MSRV 1.75
  y compilación WASM pasan. Ver `raw/final-0.log` a `final-7.log`.

No se ejecutó Hornresp de nuevo: se volvieron a ejecutar todos sus **oráculos
existentes**, sin modificar ningún archivo de referencia.
