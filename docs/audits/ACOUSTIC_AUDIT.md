# Auditoría de consistencia física

Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b` (main). Rama local:
`fix/acoustic-physical-consistency`. Árbol inicial limpio. Fecha local: 2026-11-12.
Sin push, merge ni modificaciones de calibración u oráculos.

## Línea base real

Se leyeron AGENTS.md, REVIEW.md, README.md, docs/AGENTS.md,
docs/ALGORITHMS.md, docs/VALIDATION.md, LIBRARY_STABILIZATION_REPORT.md y
los tres informes SDD de estabilización antes de cambiar el motor.

El entorno inicialmente carecía de Rust: todos los comandos solicitados
fallaron con código 127 (`raw/baseline-*.log`). Se instaló Rust 1.85.0,
rustfmt, clippy, MSRV 1.75.0 y el target wasm32-unknown-unknown.
La repetición sobre el motor intacto está en `raw/base-*.log`:
fmt, tests debug/release, rustdoc, MSRV y WASM pasan. Clippy falla por dos
`needless_range_loop` preexistentes en la sustitución hacia atrás de tapped.rs.
Métricas Hornresp: `raw/base-8.log`. Cinco mediciones release:
`raw/base-perf-*.log`; no se usa la medición histórica de otra máquina.

## Confirmación por regresión

`cargo test --test acoustic_consistency -- --nocapture`: diez pruebas fallan
sobre la base, antes de modificar el motor (`raw/regressions-before.log`).
La suite cubre AC-001/002/003/004/005/006/007 y Qts con Rms=0.
AC-008 requiere distinguir la carga de pistón en pantalla de la aproximación
empírica de campo/potencia; no se cambiará la calibración para igualar curvas.

Este registro se ampliará con implementación y verificación final.
