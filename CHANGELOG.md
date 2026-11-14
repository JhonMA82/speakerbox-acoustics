# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
El proyecto sigue versionado SemVer, pero permanece en `0.x` mientras la API
pública pueda cambiar.

## [Unreleased] — finalización de integración acústica

- Matriz analítica compleja para tramos uniformes y refinamiento RK4 en perfiles
  con pérdidas; errores explícitos por condicionamiento de tramo/cascada.
- Semiinductancia opcional con pérdidas eléctricas y escalado de bancos coherentes.
- TL segmentada integrada en Design, con cierres, zonas, estados complejos,
  pliegues lineales y balance RMS; validación externa todavía experimental.
- Optimizadores acotados locales/globales, importación FRD/ZMA independiente,
  adaptador de comparación externa y alineamientos físicamente realizables.
- Migración de literales Driver y matches exhaustivos documentada en README.
- Sin cambios de calibración ni de los oráculos originales.

### Multivía

- `sum_at`/`sum_sweep`: suma coherente de presiones complejas con referencia
  de 2.83 V RMS, polaridad, ganancia y retardo explícitos; total `None` si
  falta una vía. Contrato en `docs/audits/MULTIWAY_PRESSURE_CONTRACT.md`.
- Comprobación aritmética con dos oráculos reales de un solo driver
  (`tests/multiway_oracles.rs`), etiquetada como tal; no sustituye a una
  medición multivía.
- Validador de dataset multivía (`speakerbox-measure`): carga FRD y `meta.txt`,
  aplica las tolerancias del contrato y sale con código 1 si falla. Listo para
  ejecutarse cuando existan mediciones.

### Workspace y circuitos (Fase A + B)

- El repositorio pasa a `Cargo Workspace`; el paquete `speakerbox-acoustics`
  conserva su API y sus oráculos.
- Nuevo crate `crates/speakerbox-circuits`: modelo nodal (R, L con DCR, C,
  fuentes ideales, cargas constantes o medidas), solver MNA complejo con
  pivoteo parcial, detección de singularidad y verificación de residual,
  `Zin`, transferencia entre nodos, barridos y parser/serializador `.CIR`.
- 66 pruebas nuevas entre unitarias, integración, fixture RLC y contratos de
  error. Sin cambios en los solvers acústicos.
- Endurecimiento de entrada tras revisión: cota de 512 incógnitas comprobada
  antes de asignar, cargas medidas pasivas en todos sus puntos, división
  compleja estable, round-trip `.CIR` con valores negativos, mensajes de error
  saneados y acumulación de todos los errores de parseo.

### Identificación de impedancia (Fase C)

- Nuevo crate `crates/speakerbox-measure`: lectura y escritura FRD/ZMA con
  fase en grados, circuito equivalente de siete parámetros en SI, ajuste
  Levenberg–Marquardt restringido con residual complejo, `FitReport` con
  avisos, T/S parciales (`Fs/Qms/Qes/Qts`) y experimentos de masa añadida y
  caja sellada.
- 31 pruebas nuevas, incluido un fixture ZMA generado con una implementación
  independiente. Sin acoplamiento con `Driver` ni con los solvers acústicos.
- Serializadores FRD/ZMA con `Result` y rechazo de valores no releíbles; el
  ajuste exige `Re(Z) > 0` en todos los puntos.
- Errores de ZMA por línea (todas las filas malas, no solo la primera),
  convergencia declarada cuando la semilla ya es óptima, semilla fuera de
  límites rechazada y guarda de finitud en `Fs/Qms/Qes/Qts`.

### Crossover y optimización (Fase D)

- `speakerbox-circuits` incorpora prototipos Butterworth y Linkwitz-Riley
  (respuesta compleja), síntesis pasiva Butterworth LP/HP de orden 1–4,
  L-pad con bypass explícito, red Zobel y un optimizador Levenberg–Marquardt
  restringido contra magnitud en dB.
- 21 pruebas nuevas; la escalera sintetizada se contrasta con la función de
  transferencia analítica, no con tablas heredadas.
- RMSE ponderado por `Σw`, rechazo de objetivos inválidos construidos a mano,
  Jacobiano acotado, arranque de DCR desde el mínimo y límites asintóticos de
  los prototipos para relaciones de frecuencia enormes.

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

## Auditoría correctiva (rama fix/acoustic-physical-consistency)

- Corrige carga Rserie, radiación trasera OD y System+bocina, polaridad BP,
  fase total OD, Hypex/cilindros, paralelo y matriz BP6 concentrada.
- Corrige Qts sin pérdidas, impedancia TH con Eg=0, eficiencia al invertir
  polaridad, cuello TH con toma avanzada y OWDR malformado.
- Cilindro exacto, matrices reutilizadas y barridos preparados por préstamo.
- APIs aditivas: soluciones complejas OD/BP, TopologyPoint, PreparedDesign,
  phase_trace y diagnóstico de fase eléctrica.
- Mayor rigor en geometrías; nuevas variantes BankError. Puntos públicos,
  dependencias, calibración y oráculos conservados.
- Potencia/rendimiento legados siguen siendo estimaciones de campo; límites
  y desviaciones medidos en docs/audits.

## Segunda pasada energética (sin publicación de versión)

- Potencia de las mismas terminaciones usadas por el solver, sin convertir
  SPL en potencia ni aplicar 2π/Ω al balance; calibración y oráculos intactos.
- Carga de boca centralizada. Matched exige factores que se cancelen, usa
  ρc/Sgeom sin overflow intermedio y no fabrica SPL de un absorbente ideal.
- APIs aditivas `try_energy_at` y EnergyReport: fuente, bornes, pérdidas,
  cargas, residuales y calidad radiativa; eficiencia científica opcional.
- Observación tipada con fase/retardo opcionales y fase compleja con cota de
  redondeo. Fase legada NaN en silencio y eficiencia legada NaN para 0/0.
- OD con toma en garganta conserva su contribución trasera directa.
- Migración: los campos de potencia legados son absorción de cargas explícitas,
  no potencia integral deducida del campo; consultar la calidad para interpretar
  radiación. `try_response_at` de matched devuelve error de SPL no definido;
  usar `try_energy_at` o la observación opcional. Ver contrato de radiación.
