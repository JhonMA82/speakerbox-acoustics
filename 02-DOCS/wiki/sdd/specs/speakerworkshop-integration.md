# Spec — integración de Speaker Workshop

**Estado:** aprobada por instrucción del mantenedor (2026-11-13)
**Rama:** `feat/speakerworkshop-integration`
**Referencia técnica:** [`docs/SPEAKERWORKSHOP_REUSE_RFC.md`](../../../../docs/SPEAKERWORKSHOP_REUSE_RFC.md)

## Intención

Extender `speakerbox-acoustics` con análisis de redes eléctricas pasivas
—filtros de crossover— sin tocar los solvers acústicos ni sus oráculos, y sin
copiar código del legacy.

## Alcance de esta PR (Fase A + Fase B)

1. `Cargo Workspace` con el paquete raíz intacto y `crates/speakerbox-circuits`
   como miembro.
2. Modelo: nodos, R, L con DCR, C, fuentes de tensión ideales y cargas de
   impedancia constante o medida.
3. Solver MNA complejo con pivoteo parcial, detección de singularidad,
   verificación de residual y diagnóstico (`relative_residual`, `pivot_ratio`).
4. Medidas: tensión por nodo, `Zin`, transferencia compleja entre nodos y
   barrido.
5. Parser y serializador `.CIR` con sufijos SI y errores por línea.
6. Tests unitarios, de integración, de entradas inválidas, de determinismo y
   fixture RLC analítico.
7. Documentación: RFC, informe, rustdoc y ejemplo ejecutable.

## Fuera de alcance (declarado, no olvidado)

- Fase C: ajuste de impedancia, estimación T/S, FRD/ZMA.
- Fase D: crossover, L-pad, Zobel, optimizador.
- Fase E: audio I/O, MLS, FFT.
- Referencia de presión compleja para combinar drivers.
- Cualquier modificación de `src/system.rs`, `src/tapped.rs`, `src/offset.rs`,
  `src/bandpass.rs`, `src/calibration.rs`, `src/validation.rs` o
  `tests/data/`.

## Contratos que no se rompen

- API pública del paquete raíz: sin cambios.
- `DesignPoint.phase_deg` sigue documentado como fase de velocidad
  volumétrica; no se reinterpreta ni se usa para acústica.
- Calibración y tolerancias de producción: sin cambios.

## Criterios de aceptación

| Criterio | Prueba |
|---|---|
| El paquete raíz conserva su API | `cargo test --workspace` incluye los tests preexistentes sin editar |
| El solver reproduce oráculos analíticos | `tests/analytic.rs`, `tests/cir_fixture.rs` |
| Fallo explícito ante entradas inválidas | `tests/invalid.rs`, tests de módulo |
| Resultado independiente del orden | `tests/determinism.rs` |
| MSRV 1.75 y WASM | `docker run rust:1.75` con `cargo check` y target `wasm32-unknown-unknown` |
| Sin código legacy copiado | RFC §6 y revisión de fuentes (todas las fórmulas son teoría de circuitos) |
