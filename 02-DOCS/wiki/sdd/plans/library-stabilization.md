# Plan — library-stabilization

## Contrato de interfaces

- `SolveError` gana `InvalidParameter(&'static str)` e
  `InvalidResponse(&'static str)`; `NonFiniteResponse` se conserva.
- Cada tipo público de entrada gana `validate(&self) -> Result<(), SolveError>`:
  `Air`, `Driver`, `Segment`, `Horn`, `Chamber`, `Port`, `PassiveRadiator`.
- Cada topología gana `validate(&self)`; `try_response_at` = validar parámetros
  + calcular + validar el punto; `response_at` = `try` o punto neutro.
- `OraclePoint::parse_tsv`/`parse_csv` devuelven
  `Result<Vec<OraclePoint>, ValidationError>`.
- `ValidationReport` gana `invalid_points`; `passes*` exige `n > 0` y
  `invalid_points == 0`.
- `Design::try_sweep_log -> Result<Vec<DesignPoint>, SolveError>`; `sweep_log`
  devuelve un vector vacío si los límites o el número de puntos no son válidos.
- `import_driver_file` se compila solo en nativo; `import_driver(&str)` sigue
  disponible en wasm.

## Tareas

| # | Tarea | Riesgo | Comprobación |
|---|---|---|---|
| T1 | Variantes de error y helper de frecuencia | 1 | tests de `SolveError`/frecuencia |
| T2 | `validate()` en `Air`, `Driver`, `Segment`, `Horn`, `Chamber`, `Port`, `PassiveRadiator`; quitar clamps silenciosos | 1 | tests unitarios por tipo |
| T3 | `System`: validación completa de entrada y respuesta | 1 | tests de `try_response_at` y respuesta inválida |
| T4 | `TappedHorn`: validación de tomas, segmentos y punto | 1 | tests TH |
| T5 | `OffsetHorn`: validación de toma, cámara trasera y punto | 1 | tests OD |
| T6 | `Bandpass`: validación de puertos, QL, aire y punto | 1 | tests BP |
| T7 | Frecuencias y barridos: `is_finite`, `sweep_log`, `try_sweep_log`, filtros | 1 | tests de límites |
| T8 | Oráculos: parser estricto, oráculo vacío, puntos inválidos, `ze_phase` documentado | 1 | tests de `validation` |
| T9 | Importadores: Mms, aire, `numVC`/`VCCon`, `cfg` de filesystem | 1 | tests de importación |
| T10 | API/docs/versión: `DesignPoint`, README, INTEGRATION, VALIDATION, CHANGELOG, `0.2.0` | 0 | `cargo doc`, revisión de textos |
| T11 | Rendimiento: ejemplo `perf_sweep` y medición real | 0 | ejecución release/debug |
| T12 | Verificación completa y `LIBRARY_STABILIZATION_REPORT.md` | 0 | comandos obligatorios |

## Orden

T1 → T2 → T3/T4/T5/T6 (paralelizables por archivo, pero secuenciales para no
romper el workspace) → T7 → T8 → T9 → T10 → T11 → T12.

## Riesgos

- Cambiar firmas de parser rompe tests/ejemplos: se actualizan todos los usos.
- Quitar clamps puede exponer entradas inválidas antes ignoradas: la validación
  en entrada las convierte en `Err`/punto neutro, no en pánico.
- `ze_phase` no se compara: se documenta como limitación, no se declara
  validación completa.
