# Rendimiento medido al finalizar

Rust 1.85, release/LTO del Cargo.toml, misma máquina/entorno. Comparación final
con 7 pares intercalados base→HEAD, ejecutables compilados con el mismo toolchain.
Se informan medianas y rangos, sin concluir causalidad a partir de una muestra.
Antes de editar también se guardaron 3 ejecuciones baseline-4/5/6.
Las cifras de la primera integración eran de otra ejecución y no se reutilizan
como línea base actual. El coste de compilar no se incluye en tiempos de solver.

| Caso | Puntos | Base mediana ms (mín–máx) | Final mediana ms (mín–máx) | Cambio mediana |
|---|---:|---:|---:|---:|
| System bocina 4 tramos | 500 | 14.137 (13.851–16.387) | 14.949 (14.078–15.984) | +5.74% |
| System bocina 4 tramos | 2000 | 56.456 (54.548–61.161) | 57.350 (56.368–62.144) | +1.58% |
| TappedHorn 4 tramos | 500 | 27.494 (27.077–31.710) | 28.095 (26.967–28.644) | +2.19% |
| TappedHorn 4 tramos | 2000 | 112.429 (110.694–143.601) | 112.615 (110.255–117.857) | +0.17% |
| Bandpass BP6 serie | 500 | 0.283 (0.265–0.735) | 0.293 (0.277–0.342) | +3.53% |
| Bandpass BP6 serie | 2000 | 1.124 (1.095–1.791) | 1.219 (1.139–2.383) | +8.45% |
| 100 cambios × 50 puntos | 5000 | 140.987 (135.597–170.293) | 140.779 (133.973–146.382) | -0.15% |

La dispersión del host es visible; no se afirma una aceleración global. Los
barridos de 2000 puntos conservan tiempos cercanos a la base. Durante el trabajo
se detectó una regresión BP6 de ~25–30%: dos experimentos la investigaron.
Aislar el stack TL no ayudó (cambio descartado); eliminar criterio RK4 en tubo
exacto y reutilizar geometría de puertos validada sí redujo coste. En el ensayo
pareado de esta última modificación, BP6S 2000 pasó de 1.361 a 1.169 ms (-14.1%).
No se debilitó validación pública ni se alteraron respuestas para ganar tiempo.

## Matriz uniforme con pérdidas: comparación directa

El mismo examples/uniform_loss_perf.rs se compiló en base y final: 10000 matrices,
Rf=1000 Pa·s/m², L=0.8 m, S=0.02 m², 100 Hz; 3 repeticiones por versión.

| Versión | Mediana ms | Error relativo B frente exponencial exacta |
|---|---:|---:|
| Base RK4 | 119.505199 | 9.918991524362e-11 |
| Final analítica | 2.254692 | 5.644168026485e-16 |

Reducción de tiempo medida: 53.00×. Se mide este núcleo, no toda la aplicación.

## Nuevas capacidades

Tres repeticiones, `cargo run --release --example completion_perf`. La energía
se comprueba aparte del cronómetro en 20/50/100/200 Hz; su residual no es una
pérdida añadida. No hay comparación antes/después de topologías antes inexistentes.

| Caso | Puntos | Mediana ms | Máximo residual relativo de energía |
|---|---:|---:|---:|
| TL uniform | 500 | 0.240216 | 3.029137846e-16 |
| TL uniform | 2000 | 0.922848 | 3.029137846e-16 |
| TL variable | 500 | 17.931137 | 4.080354843e-13 |
| TL variable | 2000 | 73.140729 | 4.080354843e-13 |
| TL damped | 500 | 0.443267 | 1.637343502e-12 |
| TL damped | 2000 | 1.849592 | 1.637343502e-12 |
| TL variable damped | 500 | 53.119313 | 5.952949444e-12 |
| TL variable damped | 2000 | 210.807167 | 5.952949444e-12 |

| Optimización | Presupuesto | Evaluaciones reales | Mediana ms | Mejor coste |
|---|---:|---:|---:|---:|
| optimizer NelderMead | 100 | 100 | 0.108631 | 2.882974089e-07 |
| optimizer NelderMead | 1000 | 363 | 0.327746 | 3.153835686e-23 |
| optimizer NelderMead | 5000 | 363 | 0.073859 | 3.153835686e-23 |
| optimizer DifferentialEvolution | 100 | 100 | 0.018287 | 8.220898564e-01 |
| optimizer DifferentialEvolution | 1000 | 1000 | 0.885813 | 2.483165223e-04 |
| optimizer DifferentialEvolution | 5000 | 5000 | 10.618138 | 5.866621290e-24 |
| acoustic DE | 100 | 100 | 0.126307 | 2.078343106e-01 |
| acoustic DE | 1000 | 1000 | 1.088774 | 9.070628755e-05 |
| acoustic DE | 5000 | 5000 | 12.243528 | 7.889641008e-20 |

Los dos primeros algoritmos minimizan una esfera tridimensional; el caso
acústico ajusta volumen/Eg/Rg a tres respuestas selladas sintéticas (20/50/100 Hz),
coste suma de cuadrados en dB. Se reportan evaluaciones reales si la convergencia
termina antes del presupuesto. DE acústico sí ejecutó 100/1000/5000 evaluaciones.
La memoria/caché del optimizador es local; la búsqueda lineal de entradas
cacheadas tiene coste O(N²) acumulado y es una limitación de escalabilidad.
No se introdujo paralelismo ni caché global. Registros y experimentos intermedios
se conservan en completion-raw; los prefijos seal-perf-/seal-new-/seal-uniform-
corresponden al código final b74fbe8. Se reconstruyeron base y final con el
mismo toolchain tras restaurar el entorno de sesión; cada par se ejecutó en
el mismo host y sin compilaciones simultáneas. Los prefijos anteriores son
experimentos históricos, no la tabla final.
