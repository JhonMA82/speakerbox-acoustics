# REVIEW.md — cómo se juzga este repositorio

## Criterios de corrección

- Cada hallazgo del brief de estabilización tiene una prueba que falla sin el
  cambio y pasa con él.
- `try_response_at` no devuelve `Ok` con ningún campo inválido.
- Los parámetros se validan en los puntos de entrada; no hay clamping silencioso
  que convierta una entrada incorrecta en un resultado aparente.
- Un oráculo vacío o con puntos inválidos no puede aprobar una validación.
- El parser TSV/CSV conserva la correspondencia de columnas y falla con error
  descriptivo.
- Las frecuencias se validan con `is_finite() && > 0` en todas las rutas.
- Los importadores no fabrican parámetros físicos.
- La API pública, README y `Cargo.toml` declaran el mismo estado de madurez.

## Criterios de seguridad y compatibilidad

- Sin dependencias nuevas ni capas arquitectónicas.
- Sin cambios de calibración, física ni tolerancias de producción.
- Nativo, `wasm32-unknown-unknown` y MSRV 1.75 compilan.
- `import_driver(&str)` funciona sin filesystem; el acceso a disco es nativo.

## Evidencia exigida

- Comandos ejecutados y su resultado real en el informe.
- Métricas de producción antes/después.
- Mediciones de rendimiento reales o limitación declarada.
