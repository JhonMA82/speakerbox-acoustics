# Validación energética de segunda pasada

Base medida: `5e4388363fa248f49abd2e4a164949dbfdaf7b0b`.
Implementación: `11f9d6c`. Misma máquina, calibración y oráculos.
Se ejecutaron de nuevo las mediciones, no se tomaron las cifras de informes
anteriores. Los CSV recién generados están en `energy-raw/`.

## Resultados de balance de red

Se calculó EnergyReport en **6105 frecuencias de 15 casos**. Todos produjeron
informe finito sin fallos. Máximo error absoluto de balance: **1.320e-09 W**;
máximo relativo a potencia de fuente: **1.792e-09**. No hay término residual
metido como disipación. La diferencia máxima entre Re(pU*) y |U|²Re(Z) en
las terminaciones es **2.220e-16 W**.

| Caso | N | Máx. residual absoluto (W) | Máx. relativo a fuente | Interpretación radiativa |
|---|---:|---:|---:|---|
| OD-A | 323 | 3.769e-10 | 3.543e-10 | CalibratedLoadApproximation |
| OD1-B | 323 | 2.116e-10 | 2.032e-10 | CalibratedLoadApproximation |
| OD2-C | 323 | 9.663e-11 | 8.098e-11 | CalibratedLoadApproximation |
| TH1 | 533 | 5.239e-16 | 4.193e-15 | CalibratedLoadApproximation |
| TH2 | 533 | 2.089e-15 | 1.397e-14 | CalibratedLoadApproximation |
| TH1-S2S3 | 533 | 1.277e-15 | 1.098e-15 | CalibratedLoadApproximation |
| SEALED-A | 323 | 3.608e-16 | 6.753e-16 | SingleBaffledPiston |
| VENTED-A | 323 | 3.747e-16 | 7.956e-16 | IncompleteRadiationModel |
| PR-A | 323 | 4.857e-16 | 6.311e-16 | IncompleteRadiationModel |
| BP4-A | 323 | 4.894e-16 | 5.528e-16 | SingleBaffledPiston |
| BP6P-A | 323 | 5.872e-16 | 5.308e-16 | IndependentLoadsApproximation |
| BP6S-A | 323 | 4.085e-16 | 4.782e-16 | SingleBaffledPiston |
| HORN-1 | 533 | 1.320e-09 | 1.792e-09 | CalibratedLoadApproximation |
| HORN-2 | 533 | 7.540e-10 | 7.357e-10 | CalibratedLoadApproximation |
| HORN-3 | 533 | 1.017e-09 | 1.083e-09 | CalibratedLoadApproximation |

La suma de cargas independientes conserva la energía del circuito y se
etiqueta aproximada respecto a radiación espacial. No implica que se haya
resuelto la interferencia integrada ni la impedancia mutua de fuentes reales.
En VENTED/PR la radiación total y las eficiencias científicas son None.

### Desglose a 50 Hz (W)

| Caso | Fuente | Rg | Bornes | Bobina | Suspensión | Pérdida acústica | Terminaciones |
|---|---:|---:|---:|---:|---:|---:|---:|
| OD-A | 0.9208051 | 0.0000000 | 0.9208051 | 0.6782105 | 0.0109768 | 0.0000000 | 0.2316178 |
| OD1-B | 0.5434219 | 0.0000000 | 0.5434219 | 0.3351291 | 0.0462853 | 0.0000000 | 0.1620075 |
| OD2-C | 0.2114043 | 0.0000000 | 0.2114043 | 0.0649896 | 0.0707926 | 0.0000000 | 0.0756220 |
| TH1 | 0.8151815 | 0.0000000 | 0.8151815 | 0.5267845 | 0.0157109 | 0.0000000 | 0.2726860 |
| TH2 | 0.5094477 | 0.0000000 | 0.5094477 | 0.3756972 | 0.0551228 | 0.0000000 | 0.0786277 |
| TH1-S2S3 | 0.1130888 | 0.0000000 | 0.1130888 | 0.0100582 | 0.0830642 | 0.0000000 | 0.0199664 |
| SEALED-A | 0.1386071 | 0.0000000 | 0.1386071 | 0.0565482 | 0.0807011 | 0.0000000 | 0.0013578 |
| VENTED-A | 0.2420724 | 0.0000000 | 0.2420724 | 0.0931399 | 0.0679422 | 0.0798471 | 0.0011431 |
| PR-A | 0.1592881 | 0.0000000 | 0.1592881 | 0.0441529 | 0.0771355 | 0.0367019 | 0.0012978 |
| BP4-A | 0.6351379 | 0.0000000 | 0.6351379 | 0.3396615 | 0.0969262 | 0.1939317 | 0.0046185 |
| BP6P-A | 0.7093274 | 0.0000000 | 0.7093274 | 0.3706758 | 0.0664995 | 0.2670794 | 0.0050726 |
| BP6S-A | 0.5311577 | 0.0000000 | 0.5311577 | 0.3363065 | 0.1447077 | 0.0484224 | 0.0017211 |
| HORN-1 | 0.1692052 | 0.0000000 | 0.1692052 | 0.0831135 | 0.0777558 | 0.0000000 | 0.0083359 |
| HORN-2 | 0.2332726 | 0.0000000 | 0.2332726 | 0.1529401 | 0.0726748 | 0.0000000 | 0.0076576 |
| HORN-3 | 0.2291186 | 0.0000000 | 0.2291186 | 0.1466593 | 0.0728892 | 0.0000000 | 0.0095701 |

Los registros de producción tienen Rg=0; no se deduce de ello que potencia de
fuente y de bornes sean equivalentes en general. Las pruebas analíticas
repiten el balance con Rg=1,2,10 Ω y comparan las dos eficiencias. Las pruebas
con revestimiento, resistencia de garganta, fuga y paredes con pérdidas
calculan cada disipación desde sus estados.

## Comparación antes/después contra Hornresp

Potencia de referencia: P_ref=Efficiency(%)·Pin(W)/100, solo cuando ambas
columnas reales están presentes. No se ajustó ninguna tolerancia energética.
Las comparaciones TH/bocina abarcan todo el export; OD/cajas, 10–1.000 Hz.
Las columnas de eficiencia expresan error RMS en puntos porcentuales.

| Caso | Error potencia RMS %, antes → después | Error eficiencia RMS pp, antes → después |
|---|---:|---:|
| OD-A | 323.3464 → 33.4995 | 75.6581 → 5.3676 |
| OD1-B | 323.0747 → 35.0850 | 65.1691 → 5.3936 |
| OD2-C | 322.9137 → 40.0054 | 61.3882 → 5.4807 |
| TH1 | 215.9690 → 23.9970 | 41.2834 → 2.1117 |
| TH2 | 214.9431 → 24.0986 | 40.5475 → 2.0712 |
| TH1-S2S3 | 217.0279 → 24.5656 | 21.8815 → 1.6471 |
| SEALED-A | 0.0027 → 0.0027 | 0.0000 → 0.0000 |
| VENTED-A | 57.5560 → 45.2241 | 1.0383 → 0.7452 |
| PR-A | 52.9542 → 39.1676 | 0.6130 → 0.1711 |
| HORN-1 | 133.8349 → 32.7872 | 41.3273 → 3.7771 |
| HORN-2 | 141.2681 → 32.1915 | 30.8423 → 3.7222 |
| HORN-3 | 140.9748 → 32.0826 | 37.8689 → 3.8119 |

**VENTED/PR: las cifras después de esta tabla comparan un subtotal conocido de
terminaciones con la potencia total de referencia, no una validación de
radiación total.** EnergyReport devuelve None para esa radiación/eficiencia.
Se conserva esta comparación del campo legado solo para mostrar el cambio de
semántica y evitar una mejora engañosa. Los CSV etiquetan esa magnitud con
el nombre histórico; la calidad científica está en energy-balance.csv.

En TH se quitó el factor 2π/Ω del flujo real de boca: para Ω=π/2 ese factor
introducía una ganancia energética 4 sin cambiar la carga. En OD también se
eliminó la conversión de su presión directa calibrada a potencia de carga.
En System se usa ahora Zterm con sus área/factor efectivos, en lugar de una
resistencia de pistón incompatible. Así se explican las mejoras sin retocar
calibración. El error restante depende de los modelos y cargas empíricas;
no se asume que Hornresp y la red tengan pérdidas y fronteras idénticas.

Para BP solo hay referencias de SPL, módulo Ze y Xd; no se inventan fases,
retardo, potencia o eficiencia de referencia. El cierre de su balance se
prueba analíticamente y con estados complejos, no por comparación inexistente.

### Conservación de resultados válidos

Las trazas antes/después confirman **delta máximo exactamente cero** en SPL,
Ze, excursión, fase eléctrica, fase acústica y retardo de los 15 oráculos.
Solo cambian sus magnitudes energéticas. Resumen por caso en output-deltas.csv;
las trazas intermedias se regeneran con AUDIT_TRACE_DIR en measure.py.
Todos los tests de producción originales pasan, sin tocar oráculos,
calibración ni tolerancias. La configuración OD tap=0 sin cámara no está en
esos oráculos: su presión trasera omitida se cubre con regresión independiente.

## Reproducción numérica específica de matched_mouth

Cilindro de área 0,02 m² y longitud 0,7 m, Ω=2π, mismo altavoz y excitación.
Los estados resueltos no cambian: solo se corrige la interpretación de salida.

| Frecuencia | Potencia acústica reportada antes (W) | Flujo real absorbido por la terminación (W), antes y después |
|---|---:|---:|
| 10 Hz | 0.0000229102 | 0.2157501724 |
| 20 Hz | 0.0001065346 | 0.2508414165 |
| 50 Hz | 0.0006965952 | 0.2626224437 |
| 100 Hz | 0.0027901027 | 0.2636722935 |
| 500 Hz | 0.0517938152 | 0.2131434714 |

Después, la API de energía devuelve ese flujo con tipo MatchedAbsorber y
la API de SPL rechaza el dato no definido; no sustituye la absorción por cero.
Re(pU*) y |U|²Re(Z) difieren como máximo 2.78e-17 W en esta prueba.
Registros y reproducción: matched-before.csv, matched-after.csv y
`python docs/audits/probe_matched.py [ruta-repo]`.

## Evidencia analítica y numérica independiente

- Cilindro adaptado: 1,20,100,1.000 y 5.000 Hz. Zin=ρc/S y onda reflejada
  menor que 1e-11 de la incidente. Energía Re(pU*) y |U|²Re(Z) coinciden.
- Cilindros/tramos sin pérdidas: conservación de flujo y determinante unitario
  (reciprocidad) hasta 2 kHz. Se conservan las pruebas exactas/RK4 anteriores.
- Caja sellada ideal y circuito electromecánico: pérdidas de bobina y
  suspensión más pistón explican la potencia de fuente; Rg se separa.
- Oscilador masa-resorte: a ω0=1/sqrt(MCms) la reactancia se anula, sin
  añadir una resistencia ficticia; la potencia disipada sigue siendo Rms|v|².
- Helmholtz concentrado: a f=c/(2π)sqrt(S/(Leff·V)) las admitancias reactivas
  de cámara y masa de puerto se cancelan; no se extrapola a puertos largos.
- Resistencia pura: Re(pU*)=|p|²/R exactamente para el caso sintético.
- Pared con pérdidas: integral independiente ∫g|p|²dx frente a diferencia de
  flujos y subdivisión de un tramo en cuatro, sin definir pérdida como residual.
- BP4/BP6 paralelo/serie y OD/TH: balance con y sin QL, paredes y PR; puerto
  interior no contado como terminación exterior.
- Fuente nula: potencias cero, eficiencia científica None, fase indefinida.
- Rejillas lineales/logarítmicas, fase de oscilador en resonancia, orden
  arbitrario, duplicados, inversión de polaridad y cancelaciones exactas.
- Presión 1e-200 Pa conserva su fase con cota de error cero. Cancelación
  1e-17 Pa entre componentes de orden uno, con cota de redondeo 2ε Pa,
  devuelve fase/retardo None. No se impone un suelo universal de SPL.

Tolerancias de balance de las pruebas: |error|<=1e-10 W + 2e-7·Psource,
conservadoras respecto al error RK4 observado y a la comparación previa
exacta/RK4 (3e-6 normalizado); las identidades puramente eléctricas usan
1e-10 relativo o mejor. Las igualdades escalaras independientes usan
1e-11 W absoluto y tolerancia relativa específica. La convergencia de pared
usa 2e-6 entre mallas y 2e-7 frente a flujo. No se ampliaron para ocultar fallos.
Las cifras medidas se muestran arriba, muy por debajo de los límites.

## Suite y compatibilidad

**219 pruebas pasan** en debug y release: 123 unitarias, 90 de integración y
6 doc-tests. Se conservan las 192 pruebas de la primera auditoría. Las nuevas
son 12 regresiones y 15 pruebas físicas/contractuales. En la base, 11 de las
12 regresiones fallan; la de potencia de fuente/bornes ya pasaba y se conserva
como invariante, no se declara un defecto que no existía.

Fmt, clippy -D warnings, rustdoc -D warnings, MSRV 1.75 y compilación
wasm32-unknown-unknown pasan. No se ejecutó un runtime WASM, Windows/macOS,
Hornresp interactivo ni acústica espacial. Se reejecutaron los oráculos
originales; no se crearon datos de referencia ficticios.
