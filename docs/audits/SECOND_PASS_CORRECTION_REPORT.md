# Informe de segunda pasada correctiva

Base: `5e4388363fa248f49abd2e4a164949dbfdaf7b0b` (rama auditada anterior,
árbol inicial limpio). Rama nueva: `fix/acoustic-energy-consistency`.
Commit de implementación: `11f9d6c`. El commit de cierre posterior contiene
informes, registros y aclaraciones documentales, no cambios de ecuaciones.

## Cambios implementados

| Archivo | Modificación |
|---|---|
| src/segment.rs | Carga terminal única; adaptación geométrica, factores coherentes y cancelación estable; integral de pérdida de pared. |
| src/system.rs | Potencia de la misma carga resuelta; balance por elemento y fuente/bornes; matched sin SPL inventado. |
| src/tapped.rs | Potencia de boca sin factor de observación; balance nodal y pared con estados complejos. |
| src/offset.rs | Potencia de cargas de boca/trasera, sin calibración de campo; balance y pérdida de PR; presión trasera también con tap=0. |
| src/bandpass.rs | Flujo activo de puertos Pipe, sin sumar puerto interior; reutilización de impedancias; pérdidas QL explícitas y Lumped incompleto. |
| src/energy.rs | EnergyReport, cargas con fasores, pérdidas separadas, calidad/ausencia radiativa, dos eficiencias opcionales y residuales diagnósticos. |
| src/design.rs | Fachada científica energética y observación tipada, con fase/retardo opcionales, orden y duplicados. |
| src/phase.rs | Fase de presión compleja con cota de incertidumbre, sin suelo de amplitud. |
| src/lib.rs | Export del módulo de energía. |
| tests/energy_regressions.rs | 12 regresiones/invariantes; 11 fallan sobre la base, 12 pasan ahora. |
| tests/energy_physics.rs | 15 pruebas analíticas, balances, convergencia y contratos de fase/radiación. |
| docs/audits/measure_energy.py | Balance en los 15 fixtures de producción, sin alterar referencias. |
| docs/audits/probe_matched.py | Reproducción compatible antes/después de la discrepancia de boca adaptada. |

No se eliminaron pruebas/correcciones anteriores. Sin nuevas dependencias,
calibraciones, datos de Hornresp, hilos, cachés globales, topologías ni GUI.
Cargo.toml y Cargo.lock se conservan; no se publica una nueva versión.

## Migración mínima de resultados

Firmas y estructuras de puntos existentes se mantienen. Cambia el significado
físico corregido de potencia: absorción por cargas explícitas, no estimación
de campo. Puede ser un subtotal; EnergyReport da calidad y None si no hay
radiación total interpretable. La eficiencia legada es absorción/fuente,
incluyendo Rg, y NaN para 0/0. La eficiencia científica distingue fuente de
bornes y conserva la calidad del numerador.

La fase legada en silencio es NaN, validada únicamente con SPL=-∞; una fase
NaN con SPL finito sigue siendo error. AcousticObservation usa None para fase
indefinida y matched_mouth sin observación. try_response_at de matched ahora
rechaza SPL no definido; no impide consultar energía ni estados de la boca.
Se rechazan factores que desadapten una boca marcada como adaptada.

## Rendimiento real y regresiones

Cinco ejecuciones de `cargo run --release --example perf_sweep` antes y después,
mismo entorno Linux x86_64, Rust 1.85.0, LTO y geometrías. No se usa la medida
de la primera auditoría como línea base actual. Tiempos internos sin compilación.

| Caso | Puntos | Antes mediana [mín,máx] ms | Después mediana [mín,máx] ms | Cambio mediana |
|---|---:|---:|---:|---:|
| System bocina 4 tramos | 500 | 13.738 [13.437,15.231] | 13.763 [13.360,15.202] | +0.18% |
| System bocina 4 tramos | 2000 | 60.479 [56.297,66.635] | 55.151 [53.709,55.996] | -8.81% |
| TappedHorn 4 tramos | 500 | 27.937 [26.879,29.851] | 26.929 [26.083,27.498] | -3.61% |
| TappedHorn 4 tramos | 2000 | 111.624 [107.456,112.846] | 114.464 [109.483,143.929] | +2.54% |
| Bandpass BP6 serie | 500 | 0.279 [0.264,0.294] | 0.285 [0.265,0.321] | +2.15% |
| Bandpass BP6 serie | 2000 | 1.108 [1.091,1.250] | 1.166 [1.116,1.629] | +5.23% |
| 100 cambios × 50 puntos | 5000 | 138.373 [133.444,149.206] | 139.751 [133.578,188.748] | +1.00% |

No se observa regresión importante: 2.000 puntos TH +2,54 %, BP6S +5,23 %,
System -8,81 %, dentro de la dispersión de un entorno compartido. No se
atribuyen mejoras al cambio físico sin aislar hardware. Los tiempos siguen
conservando las optimizaciones de la primera pasada. El cálculo BP de flujo
reutiliza Zpf/Zpr: no recalcula Bessel/Struve para obtener la potencia.
Los informes detallados de energía y observación son consultas adicionales;
no se ejecutan dentro del benchmark legado. No se afirma que el diagnóstico
de pérdidas por pared tenga coste cero: integra el estado cuando hay pérdidas.

## Comandos ejecutados y resultados

Sobre la base auditada se ejecutaron suite debug/release, producción,
measure.py y cinco benchmarks (`energy-raw/base-*`). Se creó un worktree
separado en `5e43883`, al que solo se copiaron las regresiones nuevas para
verificar su fallo sin modificar su motor.

Verificación final (`energy-raw/final-[0-9].log`, todos con EXIT=0):

```bash
cargo fmt --all --check
cargo test --workspace
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo +1.75.0 check --workspace --all-targets
cargo check --lib --target wasm32-unknown-unknown
cargo test --test od_production --test th_production --test bp_production --test system_production --test production_accuracy -- --nocapture
python docs/audits/measure.py
python docs/audits/measure_energy.py
```

Además se ejecutaron:

```bash
git status --short --branch
git rev-parse HEAD
rustc -Vv
git switch -c fix/acoustic-energy-consistency
git worktree add --detach ../speakerbox-energy-base 5e43883
cargo test --test energy_regressions -- --nocapture
cargo test --test energy_physics -- --nocapture
cargo test --manifest-path ../speakerbox-energy-base/Cargo.toml --test energy_regressions -- --nocapture
python docs/audits/probe_matched.py ../speakerbox-energy-base
python docs/audits/probe_matched.py
cargo run --release --example perf_sweep
git diff 5e43883 -- tests/data src/calibration.rs Cargo.toml Cargo.lock
```

measure.py también se ejecutó con AUDIT_TRACE_DIR sobre ambas versiones para
comparar todas sus salidas; se conserva output-deltas.csv y se eliminan solo
las trazas intermedias regenerables. Pruebas físicas y métricas completas
están en ENERGY_VALIDATION_REPORT.md.

Fallos reales durante desarrollo: las regresiones previstas fallaron en la
base; clippy inicial detectó condiciones booleanas simplificables en las
excepciones de NaN. Se corrigieron, sin desactivar advertencias, y la ejecución
final pasa. El primer lote de balance y las pruebas analíticas pasaron sin
ajustar calibraciones ni ensanchar tolerancias para esconder errores.
No se omiten los registros iniciales de fallo.

## Limitaciones y estado de cierre

EC-001 queda corregido y probado. EC-002 usa cargas/estados coherentes y
separa fuente/bornes/disipación. EC-003 conserva aproximaciones geométricas
explícitas y marca radiación incompleta. EC-004 conserva fase del mismo SPL
y no devuelve un retardo tipado ficticio ante cancelación. EC-005 aporta
validación analítica y EC-006 contratos científicos aditivos.

No se afirma radiación espacial general ni coincidencia energética con
Hornresp. Persisten fuentes independientes sin impedancia mutua, cargas
calibradas y puertos/PR sin carga radiativa completa. Esas limitaciones ya
no se ocultan tras una potencia de campo o una eficiencia precisa: se
representan como aproximación o ausencia. Ver RADIATION_MODEL_CONTRACT.md.
Los límites numéricos heredados (RK4 con máximo de pasos, pivotado nodal,
closures Custom puras) siguen aplicando fuera de las bandas comprobadas.

Commits locales:

1. `0feee7a`: línea base y confirmación inicial, sin cambios al motor.
2. `11f9d6c`: correcciones de flujo, balances, APIs científicas y pruebas.
3. Commit de consolidación: informes, evidencia final y documentación.

No se hizo push, merge ni se movió main. La rama parte de la primera auditoría
y queda como continuación revisable; las cuatro auditorías nuevas no sustituyen
los archivos históricos ni reescriben los resultados medidos en aquella.

Tras las aclaraciones de documentación se repitieron fmt, rustdoc estricto y
los seis doc-tests (`final-*-refresh.log`), todos correctos. Se verificaron
además `git diff --check`, ausencia de cambios en calibración/dependencias/
oráculos y consistencia de los archivos de entrega. Se normaliza solo el
espaciado final de los logs, sin cambiar sus cifras o resultados.
