# Progress — library-stabilization

## Ledger

- 2026-11-09: baseline en `main` (commit `23a0459`): `cargo fmt --all --check`
  OK; `cargo test --workspace` OK (todos los tests y doc-tests).
- 2026-11-09: rama `stabilization/library-audit` creada desde `main`.
- 2026-11-09: spec y plan escritos; perfil técnico registrado.
- 2026-11-09: T1–T9 implementadas. `cargo test --workspace`: 117 tests de
  librería + integración + doc-tests, todos verdes.
- 2026-11-09: T10 documentación/versión (`0.2.0`, README, CHANGELOG,
  INTEGRATION, VALIDATION, AGENTS, ALGORITHMS).
- 2026-11-09: T11 rendimiento medido con `examples/perf_sweep` (release:
  bocina 88,5 µs/punto; TH 59,2; BP6S 20,6; 100×50 puntos 443 ms).
- 2026-11-09: T12 verificación completa: fmt, test, clippy, doc, wasm,
  MSRV 1.75, tests de producción con métricas sin degradación.
- 2026-11-09: revisión adversarial de correctitud: sin blockers; 1
  `should-fix` (OWDR `numVC`/`VCCon`) y 5 aristas. Corregidas en `b90fad3`
  con pruebas: 158 tests verdes.
- 2026-11-09: commits `612c5dc` (estabilización) y `b90fad3` (revisión);
  informe `LIBRARY_STABILIZATION_REPORT.md` añadido.
- 2026-11-09: rama publicada y PR #1 abierto:
  <https://github.com/JhonMA82/speakerbox-acoustics/pull/1>.
- 2026-11-09: el CI estable (clippy 1.99) detectó dos `needless_range_loop`
  en `src/tapped.rs`; corregido con `#[allow]` justificado y verificado con
  `cargo +stable clippy`.
- 2026-11-09: CI remoto del PR #1 verde: fmt/check/test/clippy/docs, MSRV,
  wasm y GitGuardian.
- Pendiente: revisión y decisión de merge (mantenedor).

## Suposiciones registradas

- «v1 cerrado» en README significa alcance de topologías validado, no SemVer
  1.0. Se mantiene pre-1.0 y se sube a `0.2.0` por los cambios de firma.
- `ze_phase_deg` queda fuera del contrato de `validate`; se documenta.
- Los constructores dejan de recortar valores; la validación explícita ocurre
  en `try_response_at`/`validate`.
- `Flare::Custom` no se puede validar sin ejecutar la closure; la
  responsabilidad de devolver áreas finitas y no negativas es del consumidor.

## Decision queue

(vacía)
