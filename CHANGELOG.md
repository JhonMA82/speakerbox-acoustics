# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
El proyecto sigue versionado SemVer, pero permanece en `0.x` mientras la API
pública pueda cambiar.

## [0.2.0] — 2026-11-09

### Añadido

- `validate()` en `Air`, `Driver`, `Segment`, `Horn`, `Chamber`, `Port`,
  `PassiveRadiator`, `System`, `TappedHorn`, `OffsetHorn`, `Bandpass` y
  `Design`.
- `validate()` en los puntos de respuesta (`ResponsePoint`, `TappedPoint`,
  `OffsetPoint`, `BandpassPoint`, `DesignPoint`).
- Variantes `SolveError::InvalidParameter` e `InvalidResponse`.
- `Design::try_sweep_log`; `sweep_log` devuelve vacío ante límites inválidos.
- `ValidationError` y `ValidationReport::invalid_points`/`is_valid`.
- Ejemplo `examples/perf_sweep.rs` para medir barridos de forma reproducible.

### Cambiado

- `try_response_at` valida aire, driver, geometría y todos los campos del
  resultado; `response_at` delega en él y devuelve el punto neutro si falla.
- `OraclePoint::parse_tsv`/`parse_csv` devuelven
  `Result<Vec<OraclePoint>, ValidationError>` y conservan las columnas.
- Los constructores ya no recortan silenciosamente longitudes, áreas,
  volúmenes, masas ni `numVC`.
- La importación de drivers rechaza `Mms <= masa de aire`, propiedades del
  aire no positivas y `numVC`/`VCCon` no enteros.
- `import_driver_file` solo se compila en nativo; `import_driver(&str)` sigue
  disponible en wasm.
- Frecuencias y barridos exigen valores finitos y positivos.
- Versión de la crate a `0.2.0` (pre-1.0).

### Documentado

- `validate` no compara `ze_phase_deg`; la limitación es explícita.
- Campos no aplicables de `DesignPoint` (`za_real`/`za_imag` en `Bandpass`,
  `port_velocity` ausente, `group_delay_ms` en `response_at`).
- `Flare::Parabolic` es un perfil de área lineal, no una parábola matemática.
- Estado de madurez, MSRV y compatibilidad wasm en `README.md`.

### Corregido

- Resultados parcialmente inválidos aceptados como `Ok` en las cuatro
  topologías.
- Clamping silencioso de parámetros en constructores públicos.
- `oracle.len().max(1)` y aprobación de oráculos vacíos.
- Desplazamiento de columnas del parser TSV por `filter_map`.
- `NaN`/infinito aceptados en algunas rutas de frecuencia.
- Fallback del 5 % de `Mms` y conversión sin validar de `numVC`.
- `numVC`/`VCCon` malformados en OpenISD ya no se degradan al valor por
  defecto; devuelven `InvalidParameter`.
- Un `Driver` derivado con `Mmd`/`Rms`/`Bl` no finitos se rechaza.
- `eg` admite signo (inversión de polaridad); `rg` sigue no negativo.
- `ValidationReport::is_valid` exige métricas finitas además de `n > 0` y cero
  puntos inválidos.
- `parse_csv` exige exactamente cuatro columnas para no desplazar datos.

## [0.1.0] — 2026-11-09

- Versión inicial publicada: motor acústico, topologías v1, bancos de drivers,
  importadores y validación de producción.
