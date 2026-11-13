# Informe de correcciones y compatibilidad

Repositorio: speakerbox-acoustics. Rama local:
`fix/acoustic-physical-consistency`.
Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b`.
Último commit de implementación: `b462560` (el commit posterior solo consolida
informes y registros). Para el hash final del paquete de revisión consultar
`git log -1` en la rama; el mensaje de entrega lo identifica explícitamente.

## Implementación

| Archivos | Cambio |
|---|---|
| src/system.rs | Rserie dentro de carga frontal, fuga de resistencia cero, matrices locales reutilizadas, salidas traseras con bocina y derivada común. |
| src/offset.rs | Caudales y presión de puerto/PR, fase total, corriente desde impedancia, una colección de matrices, OffsetSolution con fasores y potencia de red. |
| src/bandpass.rs | polarity aplicada, B=-Z en puerto concentrado, longitud coherente con Pipe, corriente desde Ze, BandpassSolution con puertos separados y potencia de red. |
| src/segment.rs | Dominio Hypex y primera raíz, rechazo de cilindros incompatibles, Custom sin clamp, exponentes más estables, matriz exacta de área constante, límite de terminación abierta. |
| src/element.rs | Paralelo escalado, polos abiertos, propagación de NaN y try_parallel. |
| src/radiation.rs | Series de R1/X1 para x pequeño, paridad de Struve, documentación correcta de Ω. |
| src/driver.rs | Qts con Rms=0 sin ∞/∞. |
| src/driver_bank.rs | Validación base/derivado y count u32 comprobado. |
| src/driver_import.rs | Rechazo de campos OWDR presentes malformados, sin inventar Le=0. |
| src/tapped.rs | Carga independiente de Eg, corriente/polaridad coherentes, cuello con stub completo, errores diagnósticos explícitos, clippy preexistente corregido. |
| src/design.rs | PreparedDesign prestado, respuestas nativas, impedancia opcional, mismo postprocesado de fase. |
| src/phase.rs | Derivada cuadrática en rejilla irregular, fase/retardo opcionales, duplicados y orden gestionados en adaptadores. |
| src/validation.rs | Diagnóstico de fase eléctrica independiente del criterio histórico. |

## Regresiones y comprobaciones

`tests/acoustic_consistency.rs`: 23 regresiones, copiadas sin cambios a un
worktree del commit base. Todas fallan allí (`raw/regressions-base-final.log`)
y pasan en la rama (`raw/final-1.log`, también release). Las pruebas añadidas
tras el primer lote tienen además registros de fallo y corrección específicos:
`expanded-before`, `expanded2-before`, `expanded3-before`, `import-before/after`
y `system-rear-before/after`. Ningún fallo inicial se cuenta como aprobación.

`tests/prepared_engine.rs`: ocho pruebas de equivalencia y contratos de
integración; `phase.rs`: tres pruebas analíticas. Total final: 192 pruebas,
frente a 158 en la base. Los oráculos se ejecutaron otra vez con sus mismos
constructores y tolerancias. Véanse VALIDATION_REPORT y PERFORMANCE_REPORT.

## Cambios de API y migración

Se conservan las firmas/estructuras de los puntos existentes y todos los
constructores. APIs aditivas en sus módulos: PreparedDesign, TopologyPoint,
OffsetSolution, BandpassSolution, phase_trace, validate_impedance_phase y
try_parallel. Sin dependencias nuevas, GUI, CAD, threads ni filesystem nuevo
en el núcleo. Cargo.toml, Cargo.lock y calibración no cambian.

Cambios conductuales necesarios:

- Cilindros con extremos diferentes, Hypex imposible y Custom inválido ahora
  se rechazan. No convertirlos silenciosamente en geometrías constantes.
- Some(0) de horn_leak es un corto; para desactivar usar None.
- Fase OD ahora corresponde a presión combinada reducida por j; añadir +90°
  para presión física. No comparar la fase histórica de boca como equivalente.
- Nuevas variantes BankError exigen actualizar matches exhaustivos.
- Helpers diagnósticos de impedancia TH sin Result devuelven NaN ante
  singularidad/incoherencia en vez de fabricar un cero.
- Cancelaciones en barridos legados dan NaN explícito. phase_trace devuelve
  Option y try_sweep_log rechaza resultados finales no interpretables.
- Los valores legados de potencia se conservan pero su documentación deja
  claro que son estimaciones, no garantías energéticas generales.

DesignPoint sigue siendo un resumen con pérdida de información por
compatibilidad; `try_topology_response_at` conserva puertos separados,
`try_reference_impedance_at` distingue None de cero físico en BP y las
soluciones complejas OD/BP permiten análisis de contribuciones. Las APIs de
System/TH aún no tienen un contenedor complejo uniforme; TH conserva debug_at
con fasores. Se documenta como evolución pendiente, sin inventar magnitudes.

## Comandos realmente ejecutados

Se ejecutaron sobre la base y de nuevo sobre la implementación final:

```bash
cargo fmt --all --check
cargo test --workspace
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo check --lib --target wasm32-unknown-unknown
cargo +1.75.0 check --workspace --all-targets
cargo test --test od_production --test th_production --test bp_production --test system_production --test production_accuracy -- --nocapture
cargo run --release --example perf_sweep
```

El último comando se repitió cinco veces en cada versión. Registros con
comando, salida y código de retorno en `raw/base-*` y `raw/final-*`.
Los comandos iniciales sin toolchain fallaron con 127, registrados en
`raw/baseline-*`. La primera ejecución real de clippy encontró dos avisos
preexistentes; la final pasa con -D warnings.

Preparación y evidencia adicional ejecutadas:

```bash
git clone https://github.com/JhonMA82/speakerbox-acoustics.git
git status --short --branch
git rev-parse HEAD
git switch -c fix/acoustic-physical-consistency
rustc -Vv
rustup toolchain list
rustup target list --installed
git worktree add --detach ../speakerbox-base 558e2fa
cargo test --manifest-path ../speakerbox-base/Cargo.toml --test acoustic_consistency -- --nocapture
cargo test --test acoustic_consistency -- --nocapture
cargo test --test prepared_engine -- --nocapture
cargo test --lib phase -- --nocapture
python docs/audits/measure.py ../speakerbox-base
python docs/audits/measure.py
git diff 558e2fa -- tests/data src/calibration.rs Cargo.toml Cargo.lock
```

Se copiaron las pruebas de regresión al worktree base antes de ejecutarlas;
no se modificó su motor. El script de métricas se ejecutó también con
`AUDIT_TRACE_DIR` para obtener deltas de salida; el resumen está en
`raw/output-deltas.csv`. Las trazas intermedias se pueden regenerar con ese
parámetro y no se necesitan como referencias nuevas. `lscpu` falló por ausencia
de `/sys/devices/system/cpu/possible`, sin impedir las medidas de tiempo.

Toolchains instalados para desbloquear el entorno: Rust 1.85.0, rustfmt y
clippy; Rust 1.75.0; rust-std wasm32-unknown-unknown. MSRV y WASM se verificaron
por compilación, no se asumen a partir de CI histórico.

## Historia y estado

1. `fc79164`: línea base y logs, motor intacto.
2. `af694cf`: correcciones físicas/numéricas y regresiones.
3. `b462560`: equivalencia, preparación de barridos, soluciones complejas,
   diagnósticos y correcciones ampliadas.
4. Commit de consolidación documental: informes y evidencia final.

No se hizo push, merge, tag ni se movió main. El resultado es una rama local
revisable. Los errores reproducidos están corregidos; el cierre energético
integral queda bloqueado por los límites descritos en REMAINING_LIMITATIONS,
no por controles técnicos fallidos ni por falta de ejecución de las pruebas.

El control final `git diff --check 558e2fa..HEAD` detectó espacios/saltos
finales de los logs y CRLF del CSV de deltas. Se normalizó únicamente la
presentación de esos archivos (sin cambiar cifras) y se repitió el control.
