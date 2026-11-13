# Rendimiento medido

Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b`.
Código final: `b462560`. Mismo Linux x86_64, Rust 1.85.0,
release con LTO y codegen-units=1, mismas geometrías y 20–200 Hz.
Comando: `cargo run --release --example perf_sweep`, cinco ejecuciones antes
y cinco después. Los tiempos son los medidos dentro del ejemplo y excluyen
compilación. Incluyen validación, cálculo, conversión a DesignPoint y fase.
No se comparan con las cifras históricas de otra máquina.

`lscpu` no pudo identificar las CPUs: falta `/sys/devices/system/cpu/possible`.
No se fijó afinidad ni se aisló carga del entorno. Se informa mediana y rango,
no una garantía de latencia. Registros completos: `raw/base-perf-[0-4].log`
y `raw/final-perf-[0-4].log`.

| Caso | Puntos | Antes: mediana [mín, máx] ms | Después: mediana [mín, máx] ms | Cociente antes/después |
|---|---:|---:|---:|---:|
| System bocina 4 tramos | 500 | 82.773 [79.321, 104.315] | 14.023 [13.229, 21.236] | 5.90× |
| System bocina 4 tramos | 2000 | 326.547 [320.547, 335.545] | 57.108 [54.646, 62.161] | 5.72× |
| TappedHorn 4 tramos | 500 | 53.911 [53.071, 71.943] | 27.813 [26.254, 33.756] | 1.94× |
| TappedHorn 4 tramos | 2000 | 217.411 [216.274, 241.209] | 107.282 [105.328, 117.940] | 2.03× |
| Bandpass BP6 serie | 500 | 16.803 [16.479, 19.969] | 0.267 [0.263, 0.280] | 62.93× |
| Bandpass BP6 serie | 2000 | 67.569 [65.379, 69.705] | 1.130 [1.096, 1.185] | 59.80× |
| 100 cambios × 50 puntos | 5000 | 826.275 [795.466, 844.761] | 140.204 [136.253, 200.860] | 5.89× |

## Cambios y alcance

OPT-001: System integra cada tramo una vez por frecuencia, frente a tres
colecciones de matrices en la ruta anterior (loads, input_impedance, transmit).
Offset reutiliza una colección en stub/camino/propagación y Design reutiliza
la impedancia ya resuelta. TH todavía vuelve a calcular la referencia de
cuello al construir DesignPoint; queda como oportunidad futura.

OPT-002: matrices exactas para perfiles analíticos de área constante SIN
pérdidas. Los dos primeros tramos del benchmark System/TH son constantes;
los puertos BP también. Custom y pérdidas mantienen RK4. No se cambió el
paso ni el límite de integración. Equivalencia y precisión en VALIDATION_REPORT.
No se extrapola esta aceleración a una bocina sin tramos constantes.

OPT-003: PreparedDesign presta y valida la configuración una vez por barrido;
la propiedad de Rust impide mutarla hasta soltar el préstamo. Resultado y
frecuencia se siguen comprobando. La positividad de Custom sigue comprobándose
en las etapas RK4. Sin privatizar campos ni debilitar la API ordinaria.

OPT-004: se evaluó caché persistente y no se añadió. Geometría/driver/aire/
calibración/excitación son mutables públicamente, Custom no tiene identidad
estable y los barridos paramétricos cambian configuración con frecuencia.
La reutilización queda limitada a la misma frecuencia/consulta, por lo que
la invalidación es automática y completa. Sin caché global.

OPT-005: medidos 500/2.000 puntos y 100 cambios de volumen × 50 puntos.
No se añadió paralelismo: el trabajo redundante y las integraciones de tubos
eran suficientes para obtener el ahorro medido. Nativo y WASM compilan.

OPT-006: medidas repetidas con el ejemplo original sin cambiar su escenario.
Las pruebas no prueban ausencia de regresiones de velocidad para toda posible
geometría; perfiles Custom o con pérdidas seguirán dependiendo de RK4.
