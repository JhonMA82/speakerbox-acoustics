# Informe — Fase C: curvas e identificación

**Fecha:** 2026-11-13
**Rama:** `feat/speakerworkshop-identification` (apilada sobre
`feat/speakerworkshop-integration`)
**Worktree:** `.worktrees/feat/speakerworkshop-identification`
**Base:** `2626cd8`

## 1. Qué se entregó

| Área | Contenido |
|---|---|
| Curvas | Parser y serializador FRD/ZMA con fase en grados, errores por línea y sin calibración asumida |
| Modelo | `EquivalentImpedance` de siete parámetros, evaluación en SI y estimaciones `Fs/Qms/Qes/Qts` |
| Ajuste | Levenberg–Marquardt en log-parámetros, residual complejo, límites, `FitReport` y avisos |
| Experimentos | Masa añadida (`Mms`, `Cms`) y caja sellada (`Vas`) con validación física |
| Pruebas | 24 nuevas (18 unitarias, 5 de integración, 1 doc-test) |
| Fixture | `tests/data/driver.zma`, generado con una implementación independiente en Python |

## 2. Contratos que no se rompen

- No se toca `src/driver.rs` ni se convierte el ajuste en un `Driver`: faltan
  `Bl`, `Sd` y una base física para repartir los elementos del circuito.
- `DesignPoint.phase_deg` sigue siendo fase de velocidad volumétrica.
- El crate acústico no depende de la medida ni de los circuitos.

## 3. Resultados reales

| Verificación | Resultado |
|---|---|
| `cargo test -p speakerbox-measure` | 24 pasan, 0 fallan |
| `cargo test --workspace` | 334 pasan, 0 fallan |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpio |
| `cargo fmt --all --check` | limpio |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | limpio |
| MSRV 1.75 y `wasm32-unknown-unknown` | contenedor `rust:1.75` (ver §6) |

Recuperación con el fixture independiente (24 puntos, 5 Hz–20 kHz):

- Error relativo máximo del ajuste: `< 1e-5`.
- Los siete parámetros quedan a menos del 5 % del valor conocido.
- `Fs` ajustada: 40 Hz dentro del 1 %.
- Coste final menor que el inicial en todas las pruebas de ajuste.
- Curva plana: aviso `NoResonance`; datos con 2 % de ruido: error relativo
  máximo `< 10 %`.
- Determinismo: dos ejecuciones idénticas devuelven exactamente el mismo
  `FitReport`.

## 4. Diferencias con el legacy

| Legacy | Aquí |
|---|---|
| Ajustes heurísticos con `fright − fcurerror` en el denominador | LM con criterio de convergencia y límites |
| Residual amplitud + fase ponderada a mano (`phase/20`) | residual complejo, sin salto de ±180° |
| Estado del objeto modificado y `AfxMessageBox` | `FitReport` puro, sin UI |
| T/S completos a partir de la curva | solo `Fs/Qms/Qes/Qts` derivados del circuito, declarados como estimación |
| Experimentos con constantes y gramos fijos | SI, validación de que la masa baja `Fs` y la caja la sube |

## 5. Límites conocidos

- El ajuste de siete parámetros puede ser débilmente identificable en bandas
  estrechas; el informe lo señala con `ParameterAtBound` y `PoorFit`, no lo
  esconde.
- No se estima `Vas` por masa añadida porque exige `Sd`; solo por caja
  sellada.
- Las curvas FRD no se interpretan como presión compleja calibrada.
- Sin optimizador de crossover ni topologías de filtro: Fase D.
- Sin audio I/O ni FFT: Fase E.

## 6. MSRV y WASM

Contenedor `rust:1.75`:

```text
$ cargo check --workspace --all-targets
    Checking speakerbox-measure v0.1.0 (/w/crates/speakerbox-measure)
    Checking speakerbox-circuits v0.1.0 (/w/crates/speakerbox-circuits)
    Checking speakerbox-acoustics v0.2.0 (/w)
    Finished dev profile
```

```text
$ rustup target add wasm32-unknown-unknown && \
  cargo check --workspace --lib --target wasm32-unknown-unknown
    Finished dev profile
```

## 7. Uso

```rust
use speakerbox_measure::{fit_equivalent, parse_zma, FitOptions};

let parsed = parse_zma(&std::fs::read_to_string("driver.zma")?)?;
let report = fit_equivalent(&parsed.curve, &FitOptions::default())?;
println!("Fs = {} Hz", report.parameters.ts_estimate().fs_hz);
```
