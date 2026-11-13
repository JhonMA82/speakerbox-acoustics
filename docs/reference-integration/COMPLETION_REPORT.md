# Finalización correctiva de integración acústica

Base íntegra: `3e9e15d867cb1840113c5eee801f57e8a978f843`. Rama: `feature/complete-acoustic-reference-integration`.
Último commit de implementación: `b74fbe8ba380fde98f3d79aa4b3195e038183849` (Rust y herramientas; los commits
posteriores sellan documentación/registros). El SHA de cierre documental está
en el manifiesto FINAL_STATE.txt que acompaña al paquete y se obtiene con
`git rev-parse feature/complete-acoustic-reference-integration`.
No se hizo push ni merge; las dos auditorías previas son ancestros conservados.

## Estado comprobado y clasificación inicial

| Hito | Estado encontrado en 3e9e15d | Resultado implementado y evidencia |
|---|---|---|
| M1 | Pérdidas implementadas parcialmente, sin robustez extrema | Exponencial compleja, criterio de RK4 complejo/convergencia, cota de dominio por tramo/cascada, regresión roja/verde |
| M2 | No implementada | CoilModel opcional, Ke en Ω√s, calor adicional, bancos y balances en todos los solvers |
| M3 | Parcial: factory uniforme experimental basada en OD | TL segmentada nativa de Design, posiciones 0..100%, cierres, zonas, pliegues, estados complejos, pruebas analíticas/energéticas |
| M4 | No implementada | Nelder–Mead y DE acotados/cancelables, candidatos válidos y objetivos acústicos, pruebas matemáticas y ajuste sellado |
| M5 | No implementada | FRD/ZMA con referencias, unidades, errores e interpolación compleja, fixtures propios |
| M6 | No implementada | Harness externo ejecutado, export/comparador genérico con métricas, bandas y tolerancias; sin dependencia de producción |
| M7 | Sellada implementada y validada; ventiladas ausentes | Butterworth/Bessel/Chebyshev, B4/QB3/SC4 en dominios realizables, propuesta EBS optimizada con coste explícito |
| M8 | Pendiente para estos cambios | Suite, compatibilidad, oráculos, energía, benchmarks repetidos y estos informes |

Se implementaron las capacidades; no se declara validación física universal.
TL sigue experimental respecto a oráculos externos; EBS es una propuesta
aproximada cuya calidad se expone, no un alineamiento exacto garantizado.

## Línea base y cronología real

El árbol inicial estaba limpio en la rama de integración anterior. Se creó la
rama solicitada desde ese SHA, sin partir de main. Al comienzo no estaba cargo
en PATH; los registros base-*.log conservan los fallos del entorno. Se restauró
Rust 1.85 y se instalaron rustfmt/Clippy, Rust 1.75 y target WASM.
Antes de modificar el núcleo se ejecutaron lib y suites acústicas/energéticas/
integración, producción mediante los dos scripts de métricas, y tres benchmarks.
La regresión de estrés falló contra el código anterior (baseline-1.log, EXIT=101).
La suite ÍNTEGRA del SHA base se ejecutó retrospectivamente en un worktree
inalterado: 223 pruebas. No se presenta esa ejecución como anterior a los cambios.
La versión final registra 244 pruebas Rust (incluidos 6 doctests), y 2 Python.

## Verificaciones finales realmente ejecutadas

Los logs seal-*.log identifican comando y EXIT; todos los siguientes
terminaron en 0:

| Comando | Resultado |
|---|---|
| cargo fmt --all --check | pasa |
| cargo test --workspace | 244 pasan |
| cargo test --workspace --release | 244 pasan |
| cargo clippy --workspace --all-targets -- -D warnings | pasa |
| RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps | pasa |
| cargo +1.75.0 test --workspace | 244 pasan |
| cargo build --lib --target wasm32-unknown-unknown | pasa |
| python tools/cross-validation/test_compare.py | 2 pasan |
| python docs/audits/measure.py | 15 casos, 6105 puntos Hornresp |
| python docs/audits/measure_energy.py | 15 casos, ningún fallo de energía |

Además: cargo test --test coil_model --test energy_physics --test reference_integration;
cargo test --test transmission_line; cargo test --test optimization;
cargo test --test measured_data; cargo test --test alignments -- --nocapture;
cargo test --test completion_numerics --lib; cargo run --release --example perf_sweep;
cargo run --release --example completion_perf; cargo run --release --example uniform_loss_perf;
python tools/cross-validation/run.py ../loudspeaker-sim-reference;
python tools/cross-validation/compare.py con/sin tolerancias explícitas.
Los registros conservan rondas intermedias y fallos corregidos, no sólo ejecuciones verdes.
El test actual contra el comparador anterior falla (comparator-before-fix.log),
y contra la versión corregida pasa; no se aceptan comparaciones vacías ni NaN en tolerancias.

## Precisión antes/después

Todos los valores impresos por measure.py coinciden antes/después a sus 9 decimales;
no es una afirmación de igualdad bit a bit por debajo de esa resolución. No se
cambiaron factores ni datos para conservarlos. Los siguientes valores RMS son
los mismos en ambos extremos (Ze/Xd relativos; no equivalen a error complejo).

| Caso | n | SPL RMS dB | Ze RMS % | Xd RMS % | Potencia RMS % diagnóstico | Eficiencia RMS pp diagnóstico |
|---|---:|---:|---:|---:|---:|---:|
| OD-A | 323 | 0.845938538 | 4.702637965 | 4.033760633 | 33.499474648 | 5.367614647 |
| OD1-B | 323 | 0.845017949 | 4.682380070 | 4.098343417 | 35.085031204 | 5.393609854 |
| OD2-C | 323 | 0.853211309 | 4.786857662 | 4.310838815 | 40.005402875 | 5.480656893 |
| TH1 | 533 | 0.526771140 | 7.029530274 | 5.678048014 | 23.996965983 | 2.111660163 |
| TH2 | 533 | 0.515683669 | 7.059529428 | 5.930873246 | 24.098646348 | 2.071232304 |
| TH1-S2S3 | 533 | 0.547724620 | 6.727089435 | 4.591618006 | 24.565586170 | 1.647051325 |
| SEALED-A | 323 | 0.000000330 | 0.000002816 | 0.000396220 | 0.002725662 | 0.000000296 |
| VENTED-A | 323 | 0.061028033 | 0.738564011 | 0.362479337 | 45.224126149 | 0.745230186 |
| PR-A | 323 | 0.375291768 | 2.491700380 | 1.006330254 | 39.167596028 | 0.171057025 |
| BP4-A | 323 | 0.214413299 | 0.236877437 | 0.460645638 | — | — |
| BP6P-A | 323 | 0.217976469 | 0.213849923 | 0.569147518 | — | — |
| BP6S-A | 323 | 0.256475188 | 1.439469704 | 3.959352013 | — | — |
| HORN-1 | 533 | 1.914094277 | 6.417324470 | 9.337784340 | 32.787189431 | 3.777127389 |
| HORN-2 | 533 | 1.942630236 | 5.977588800 | 7.142782703 | 32.191456853 | 3.722226837 |
| HORN-3 | 533 | 1.926989899 | 6.171664089 | 7.760350303 | 32.082569915 | 3.811918971 |

Potencia/eficiencia Hornresp son diagnósticos de magnitudes con definiciones y
calibraciones diferentes, no nuevos criterios de aprobación. Los errores históricos
no desaparecen con estas capacidades: no se ajustaron para forzar coincidencia.
Fase de impedancia/acústica y retardo se conservan en el CSV diagnóstico; BP
no dispone de todas esas columnas de referencia. El contrato de validación
histórico no certifica la fase de impedancia por haberla parseado.

## Balance energético antes/después

Los CSV de energía coinciden a la resolución impresa. Ningún residual se añadió
como pérdida. Tabla: máximo absoluto sobre cada barrido, calidad radiativa tal
como la reporta el modelo (no todas las topologías tienen potencia exterior completa).

| Caso | Máximo balance W | Máximo relativo a fuente | Calidad |
|---|---:|---:|---|
| OD-A | 3.768512724101e-10 | 3.543365396395e-10 | CalibratedLoadApproximation |
| OD1-B | 2.115721486895e-10 | 2.031626342289e-10 | CalibratedLoadApproximation |
| OD2-C | 9.662862177073e-11 | 8.098010793589e-11 | CalibratedLoadApproximation |
| TH1 | 5.238864897450e-16 | 4.193129295414e-15 | CalibratedLoadApproximation |
| TH2 | 2.088607065076e-15 | 1.396619721298e-14 | CalibratedLoadApproximation |
| TH1-S2S3 | 1.276756478319e-15 | 1.098426584139e-15 | CalibratedLoadApproximation |
| SEALED-A | 3.608224830032e-16 | 6.752503627653e-16 | SingleBaffledPiston |
| VENTED-A | 3.747002708110e-16 | 7.956007615575e-16 | IncompleteRadiationModel |
| PR-A | 4.857225732735e-16 | 6.311131787877e-16 | IncompleteRadiationModel |
| BP4-A | 4.894495182414e-16 | 5.527780434229e-16 | SingleBaffledPiston |
| BP6P-A | 5.872038966181e-16 | 5.307700253278e-16 | IndependentLoadsApproximation |
| BP6S-A | 4.084689333192e-16 | 4.781660694612e-16 | SingleBaffledPiston |
| HORN-1 | 1.320302089880e-9 | 1.791621789176e-9 | CalibratedLoadApproximation |
| HORN-2 | 7.540250923643e-10 | 7.357162775090e-10 | CalibratedLoadApproximation |
| HORN-3 | 1.017406381365e-9 | 1.083248739815e-9 | CalibratedLoadApproximation |

Nuevas pruebas independientes: matrices analíticas y convergencia, balance
Darcy por integral local, circuitos de bobina y calor, nodos de TL y fronteras,
reciprocidad/subdivisión/polaridad, filtros ideales y problemas de optimización
con mínimos conocidos. El comparador externo y sus números están separados en
CROSS_VALIDATION_REPORT.md: SPL RMS 0.0244794 dB, Ze RMS 0.339605 Ω;
no convierten loudspeaker-sim en oráculo físico. El harness identifica Mms/Mmd,
RMS/pico, aire, tensión, banda y las diferencias residuales de radiación.

## Rendimiento y compatibilidad

PERFORMANCE_COMPARISON.md contiene medianas/rangos y precisión conjunta de
benchmarks. Matriz Darcy uniforme: 57.28× más rápida, error B de ~1e-10 a ~6e-16.
No se afirma mejora global; se investigó y corrigió el coste BP6 repetitivo.
TL uniforme/amortiguada usa matrices exactas; geometría variable mantiene RK4.
Se midieron 100/1000/5000 evaluaciones acústicas del optimizador, no se confundió
presupuesto solicitado con evaluaciones realmente realizadas.

Sin dependencias nuevas. Constructores anteriores conservan Le y Rf=0.
Migración indispensable: literales Driver añaden coil_model; matches exhaustivos
Design/TopologyPoint añaden TransmissionLine; TerminationKind añade
PrescribedResistance para no confundir cargas arbitrarias con bocas adaptadas. La representación única de Driver
permite que todos los solvers/bancos contabilicen el mismo modelo eléctrico.
Los importadores anteriores permanecen clásicos: no infieren Ke ni T/S ausentes.

## Limitaciones residuales y alcance de la evidencia

- Dominio numérico de matrices de transferencia documentado: ≤8 nepers acumulados;
  también límite de resolución complejo compartido con la cuadratura. Fuera hay
  diagnóstico, no una respuesta ficticia. No se implementó matriz de dispersión.
- Rf es Darcy lineal, no identifica cualquier material poroso ni turbulencia.
- TL: no oráculo Hornresp TL ni medición externa documentada; prueba analítica/
  energética sí. SPL compacto y cargas independientes omiten geometría espacial,
  orientación y acoplamiento mutuo. Los pliegues requieren masa/resistencia
  lineales suministradas; el recorrido está en las longitudes, sin inventar pérdidas.
- Ke está validado analíticamente/energéticamente; falta medición ZMA con
  procedencia y referencias suficientes para afirmar ajuste experimental.
- B4/Bessel/C4 no son realizables con cualquier Qts sin asistencia externa;
  SC4 tiene dominio explícito. EBS no es único y el ejemplo conserva ~3.947 dB
  RMS de desajuste: usar el coste para rechazar propuestas insuficientes.
- Nelder–Mead/DE no garantizan mínimo global; el caché local tiene coste de
  búsqueda lineal y la función objetivo debe permanecer fija durante una ejecución.
- FRD/ZMA no infiere distancia, referencia temporal ni fase ausente; no extrapola.
  Decimal con punto, escalas explícitas; no parser universal de CSV entrecomillado.
- Comparación externa ejecutada sólo para sellada normalizada; el adaptador consume
  otras exportaciones equivalentes, sin afirmar que esas comparaciones se hayan hecho.
- Incertidumbre de licencia/procedencia de archivos del proyecto de referencia:
  no se incorporó implementación ni datos de terceros; harness original externo.

## Archivos y entregables

Módulos añadidos: transmission_line.rs, optimization.rs, measured_data.rs,
alignments.rs; núcleo modificado: segment/driver/driver_bank/energy y
call sites de System/TH/OD/Bandpass/Design/lib. Pruebas nuevas en tests/
completion_numerics, coil_model, transmission_line, optimization, measured_data,
alignments. Herramienta independiente en tools/cross-validation; dos ejemplos
adicionales de rendimiento. README, CHANGELOG y algoritmos actualizados.
Los siete informes solicitados y ALIGNMENT_VALIDATION están en este directorio;
las auditorías originales se preservan. oracle-integrity.log confirma tests/data
y src/calibration.rs intactos. No se declara validación universal ni 100% de
exactitud por tener suite verde; las capacidades se entregan con sus dominios.

## Commits de trabajo

```text
f1796b4 fix: resolve complex distributed propagation and reject ill-conditioned segments
9eca123 feat: add passive fractional coil model with explicit excess heat accounting
f765cf6 feat: integrate segmented transmission lines with boundary states folds and energy
3d64459 feat: implement bounded deterministic local and global acoustic optimization
5b01579 feat: import FRD and ZMA with explicit references and complex interpolation
e46dcc8 test: automate external solver comparison with normalized physical references
3cce234 feat: synthesize classical alignments and fit explicit extended bass shelf proposals
3a22dd0 fix: harden cascade conditioning nodal flow and measurement edge cases
f2c8962 perf: skip RK4 resolution work for exact lossless uniform propagation
6614cd4 bench: measure new solvers optimizers and analytic lossy propagation
d0193fd perf: reuse validated port geometry in bandpass transfer matrices
ded6c19 fix: reject empty external comparisons and invalid diagnostic tolerances
c21dbe7 fix: reject nonfinite lossless matrices from invalid custom profiles
b74fbe8 fix: distinguish prescribed resistive boundaries from matched absorbers
```


Seguimiento de cierre: la comprobación de finitud también cubre RK4 sin pérdidas
con Custom inválido entre muestras preliminares. Su prueba falló antes de la
línea corregida y pasó después (custom-area-before/after.log). Tras un cambio
de sesión el entorno retiró los toolchains; se restauraron las mismas versiones
y se repitieron los controles completos (seal-*.log). Los fallos de disponibilidad
se conservan por separado, sin confundirlos con regresiones científicas.

La clasificación de una frontera resistiva prescrita se corrigió mediante prueba
roja/verde, sin cambiar sus estados ni potencia (boundary-kind-before/after.log).
