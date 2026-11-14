# Plan — integración de Speaker Workshop

**Spec:** [speakerworkshop-integration.md](../specs/speakerworkshop-integration.md)
**Rama:** `feat/speakerworkshop-integration`
**Worktree:** `.worktrees/feat/speakerworkshop-integration`

## Fase A — baseline y contratos

- [x] Registrar HEAD de partida y estado del árbol. Evidencia: `c1e79e1`.
- [x] Ejecutar `cargo test --workspace` antes de tocar código. Evidencia:
      verde, 0 fallos (§5 del informe).
- [x] Publicar RFC: `docs/SPEAKERWORKSHOP_REUSE_RFC.md`.
- [x] Declarar el límite de `DesignPoint.phase_deg` (RFC §7, spec §Contratos).
- [x] Escribir oráculos analíticos antes de la integración:
      `crates/speakerbox-circuits/tests/analytic.rs`.

## Fase B — redes RLC y simulación

- [x] `Cargo Workspace` con el paquete raíz conservado. Evidencia:
      `git diff --name-only -- src tests examples` vacío.
- [x] Modelo tipado y validado (`src/model.rs`).
- [x] Solver MNA con pivoteo y residual (`src/solver.rs`).
- [x] Medidas: tensión, `Zin`, transferencia, barrido (`src/analysis.rs`).
- [x] Parser/serializador `.CIR` (`src/cir.rs`).
- [x] Tests unitarios y de integración: 66.
- [x] Ejemplo ejecutable: `examples/ladder_sweep.rs`.
- [x] Verificación MSRV/WASM/clippy/rustdoc: §5 del informe.
- [x] Informe final: `SPEAKERWORKSHOP_INTEGRATION_REPORT.md`.

## Evidencia

`cargo test --workspace` → 310 tests, 30 suites, 0 fallos.
`cargo test -p speakerbox-circuits` → 66 tests, 0 fallos.
MSRV 1.75 y `wasm32-unknown-unknown` verificados en contenedor (§10 del
informe).
