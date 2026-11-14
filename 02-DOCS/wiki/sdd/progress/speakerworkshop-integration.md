# Progreso — integración de Speaker Workshop

**Rama:** `feat/speakerworkshop-integration`
**Spec:** [speakerworkshop-integration.md](../specs/speakerworkshop-integration.md)
**Plan:** [speakerworkshop-integration.md](../plans/speakerworkshop-integration.md)
**Informe:** [`SPEAKERWORKSHOP_INTEGRATION_REPORT.md`](../../../../SPEAKERWORKSHOP_INTEGRATION_REPORT.md)

## Ledger

| Fecha | Hecho | Evidencia |
|---|---|---|
| 2026-11-13 | Baseline en `c1e79e1` | `cargo test --workspace` verde, 0 fallos |
| 2026-11-13 | Workspace + crate eléctrico | `cargo tree -p speakerbox-circuits` solo muestra `num-complex` |
| 2026-11-13 | Solver y medidas | `cargo test -p speakerbox-circuits`: 66 tests verdes |
| 2026-11-13 | Workspace completo | `cargo test --workspace`: 310 tests, 0 fallos |
| 2026-11-13 | Calidad | fmt, clippy `-D warnings` y rustdoc `-D warnings` limpios |
| 2026-11-13 | MSRV y WASM | contenedor `rust:1.75`; ver §10 del informe |

## Decisiones

- Crate hermano en `crates/` en vez de módulos en `src/`: frontera verificada
  por el compilador y cero coste para el consumidor acústico.
- Sin dependencia entre el crate eléctrico y el acústico.
- Extrapolación de curvas prohibida por defecto.
- Cargas pasivas con `Re(Z) > 0`.
- No se copia código legacy; solo fórmulas públicas y el fixture de topología
  descrito en la auditoría.

## Cola

- Fase C: FRD/ZMA, `EquivalentImpedance`, `FitReport`, masa añadida.
- Fase D: HP/LP, L-pad, Zobel, optimizador.
- Fase E: audio I/O y `speakerbox-measure`.
