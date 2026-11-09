# AGENTS.md

Guía rápida para agentes que trabajen en `speakerbox-acoustics`. La versión
detallada está en [`docs/AGENTS.md`](docs/AGENTS.md); la guía de integración
para aplicaciones está en [`docs/INTEGRATION.md`](docs/INTEGRATION.md).

## Estado

**v1 cerrado.** Topologías validadas a producción contra exports propios de
Hornresp 60.70:

- Bocina normal.
- OD / OD1 / OD2.
- TH (S2/S4) y TH1 (S2/S3).
- BP4, BP6 paralelo y BP6 serie.
- Caja sellada, ventilada y radiador pasivo.

No se aceptan topologías nuevas sin modelo, oráculo propio y test de producción
puntual.

**Fase 3.1 en curso:** `DriverBank` implementado para N drivers, paralelo/serie/
serie-paralelo, isobaric, push-pull y DVC paralelo/serie. Pendiente:
`RadiatorArray` para line arrays y validación de producción con exports propios
multi-driver.

## Comandos obligatorios

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo check --lib --target wasm32-unknown-unknown
cargo +1.75.0 check --workspace --all-targets
```

Métricas por topología:

```bash
cargo test --test od_production -- --nocapture
cargo test --test th_production -- --nocapture
cargo test --test bp_production -- --nocapture
cargo test --test system_production -- --nocapture
cargo test --test production_accuracy -- --nocapture
```

## Invariantes

- API público en **SI**; los `from_ui_units` aceptan cm², cm, L, g, mH, mm.
- `xd_mm` es desplazamiento de **pico** (`√2`·RMS) en todas las topologías.
- `phase_deg` es fase de velocidad volumétrica; la presión añade +90°.
- `solid_angle` en estereorradianes (`4π`, `2π`, `π`, `π/2`).
- `response_at` devuelve un punto neutro; `try_response_at` devuelve
  `SolveError`.
- La calibración es empírica y está ligada a la geometría validada. No cambie
  constantes de calibración sin revalidar los tests de producción.
- No añada una topología sin export propio de Hornresp y test puntual.
- No copie datos ni código de proyectos con licencia incompatible.

## Mapa mínimo

```text
src/system.rs      System: radiador directo, caja, bocina, puerto, PR
src/driver_bank.rs bancos de drivers (N, wiring, isobaric, DVC)
src/driver_import.rs importación .wdr / .owdr
src/tapped.rs      TappedHorn: TH/TH1
src/offset.rs      OffsetHorn: OD/OD1/OD2
src/bandpass.rs    Bandpass: BP4/BP6P/BP6S
src/design.rs      Design/DesignPoint para la UI
src/validation.rs  OraclePoint, validate, tolerancias
tests/data/        registros y oráculos propios
docs/              integración, algoritmos y guía detallada de agentes
```

## Regla de oro

Cada cambio debe dejar `cargo test --workspace` en verde y no debe degradar las
métricas de producción. Si una topología no tiene oráculo propio, queda como
experimental y no entra en v1.
