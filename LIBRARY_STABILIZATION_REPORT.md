# LIBRARY_STABILIZATION_REPORT

- **Repositorio:** `speakerbox-acoustics`
- **Rama:** `stabilization/library-audit` (creada desde `main`, commit base `23a0459`)
- **Fecha:** 2026-11-09
- **Versión resultante:** `0.2.0` (pre-1.0)
- **Alcance:** estabilización y cierre de los hallazgos de auditoría; sin
  topologías nuevas, sin cambios de física ni de calibración, sin GUI, sin
  dependencias nuevas.

---

## A. Resumen ejecutivo

Los seis grupos de hallazgos de la auditoría están corregidos y cubiertos por
pruebas: validación completa de resultados en las cuatro topologías,
validación explícita de parámetros sin clamping silencioso, oráculos estrictos
que no aprueban conjuntos vacíos, frecuencias y barridos validados,
importadores que no fabrican parámetros y API/documentación coherentes con una
versión `0.2.0` pre-1.0.

La suite completa pasa: **158 tests** (120 de librería, 32 de integración y 6
doc-tests), más `fmt`, `clippy -D warnings`, `rustdoc -D warnings`, compilación
wasm y MSRV 1.75. Los tests de producción conservan las métricas documentadas
(SPL, `Ze` y `Xd`) sin degradación. El rendimiento se midió con un ejemplo
reproducible: en release, una bocina de 4 tramos tarda ≈ 89 µs/punto y un
barrido de 2.000 puntos ≈ 179 ms.

**Clasificación:** lista para **consumo experimental** como dependencia Rust
`0.2.0`. Los modelos acústicos están validados a producción; la API pública
sigue siendo pre-1.0 y puede cambiar en versiones menores. La limitación de
`ze_phase_deg` está documentada explícitamente.

---

## B. Hallazgos corregidos

### B.1. Validación parcial de resultados (`try_response_at`)

- **Problema:** algunas topologías solo comprobaban `ze_mag`; podían devolver
  `Ok` con `NaN`/infinito en otros campos.
- **Archivos:** `src/error.rs`, `src/system.rs`, `src/tapped.rs`,
  `src/offset.rs`, `src/bandpass.rs`, `src/design.rs`.
- **Solución:**
  - `validate()` en `ResponsePoint`, `TappedPoint`, `OffsetPoint`,
    `BandpassPoint` y `DesignPoint`.
  - `try_response_at` valida la configuración, calcula y valida **todos** los
    campos del resultado.
  - `response_at` delega en `try_response_at` y devuelve el punto neutro
    documentado si falla.
  - Nuevas variantes `SolveError::InvalidParameter` e
    `SolveError::InvalidResponse`, con el campo concreto en el mensaje.
  - Reglas: `ze_mag > 0`; `xd_mm`, potencia y velocidades `>= 0`;
    `spl_db` finito o `-∞` solo por silencio; fases y `za` finitos.
- **Pruebas:** `response_point_validation_rejects_invalid_fields` (System),
  `point_validation_rejects_invalid_fields` (TH/OD/BP),
  `design_point_validation_rejects_invalid_fields`, más los tests de
  `try_response_at` por topología.

### B.2. Validación de parámetros y clamping silencioso

- **Problema:** campos públicos podían contener valores no físicos y varios
  constructores los recortaban con `max()`/`clamp()` sin informar.
- **Archivos:** `src/air.rs`, `src/driver.rs`, `src/segment.rs`,
  `src/element.rs`, `src/system.rs`, `src/tapped.rs`, `src/offset.rs`,
  `src/bandpass.rs`.
- **Solución:**
  - `validate()` en `Air`, `Driver`, `Segment`, `Horn`, `Chamber`, `Port`,
    `PassiveRadiator`, `Lining` y en las cuatro topologías.
  - Eliminados los recortes silenciosos de longitud, área, volumen, masa,
    `nd` de `Driver`, `loss` y factores de radiación.
  - Reglas estructurales explícitas: puerto/PR/`rear_ql` requieren recinto
    trasero; cámara de garganta y fugas requieren bocina; BP6 exige puerto
    trasero; BP4 lo rechaza; tomas de TH/OD dentro de rango y distintas.
  - `Flare::Custom` documenta que la closure debe devolver áreas finitas y no
    negativas (no se puede validar una función arbitraria).
- **Pruebas:** tests de validación en cada módulo y de configuración
  incoherente en `System`, `TappedHorn`, `OffsetHorn` y `Bandpass`.

### B.3. Oráculos de validación

- **Problema:** `oracle.len().max(1)` permitía RMSE 0 con un conjunto vacío;
  el parser usaba `filter_map` y desplazaba columnas; `ze_phase_deg` se
  importaba pero no se comparaba.
- **Archivos:** `src/validation.rs`, consumidores en `tests/` y `examples/`.
- **Solución:**
  - `OraclePoint::parse_tsv`/`parse_csv` devuelven
    `Result<Vec<OraclePoint>, ValidationError>`.
  - Parseo por índice, sin descartar celdas: columnas faltantes, celdas no
    finitas, frecuencias no positivas y `Ze <= 0`/`Xd < 0` producen error
    descriptivo.
  - `ValidationReport` añade `invalid_points`; `is_valid()` exige `n > 0` e
    `invalid_points == 0`; `passes`/`passes_pointwise` lo aplican.
  - Un oráculo vacío produce `ValidationError::Empty` y un informe inválido.
  - `ze_phase_deg` se parsea y conserva, pero **no se compara**; la limitación
    está documentada en el módulo, README, `docs/VALIDATION.md` y
    `docs/INTEGRATION.md`.
- **Pruebas:** `empty_oracle_never_passes`,
  `invalid_oracle_points_never_pass`, `tsv_rejects_missing_columns_and_invalid_cells`,
  `tsv_preserves_column_positions_with_an_empty_optional_cell`,
  `tsv_rejects_invalid_frequencies_and_values`, `tsv_rejects_empty_or_header_only`,
  `csv_parses_and_rejects_invalid_rows`, `phase_is_parsed_but_not_compared`.

### B.4. Frecuencias y barridos

- **Problema:** algunas rutas comprobaban solo `<= 0` (no rechazaban `NaN`);
  los barridos podían dejar pasar infinito; `sweep_log` recortaba el número de
  puntos y no validaba los límites.
- **Archivos:** `src/error.rs`, `src/system.rs`, `src/design.rs`,
  `src/tapped.rs`, `src/offset.rs`, `src/bandpass.rs`.
- **Solución:**
  - Helper único `check_frequency`: finita y estrictamente positiva.
  - Los barridos filtran `NaN`/infinito/0/negativos.
  - `Design::try_sweep_log` exige límites válidos, `fmax > fmin` y `points >= 2`;
    `sweep_log` devuelve vacío ante cualquier error.
  - `Design::validate()` valida la topología antes del barrido.
- **Pruebas:** `try_response_at_rejects_invalid_frequency` en las cuatro
  topologías, `sweep_log_rejects_invalid_bounds_and_point_counts`,
  `response_filters_non_finite_frequencies`,
  `try_sweep_log_matches_sweep_log`.

### B.5. Importadores de drivers

- **Problema:** `driver_from_ts` inventaba un `Mmd` del 5 % de `Mms` cuando
  `Mms` no superaba la masa de aire; OpenISD no validaba `c`/`rho` como WinISD;
  `numVC` se redondeaba y casteaba sin validar.
- **Archivos:** `src/driver_import.rs`, `src/lib.rs`.
- **Solución:**
  - `Mms <= masa añadida del aire` devuelve
    `ImportError::Inconsistent`; no se fabrica ningún parámetro.
  - `TsInput::validate` rechaza cualquier parámetro presente no finito o no
    positivo; `Le` admite cero pero no negativos.
  - `c`/`rho` se validan en ambos formatos; ausente usa el valor por defecto,
    presente inválido es error.
  - `integer_field` valida `numVC`/`VCCon`: finito, entero, en rango `u32`.
  - `import_driver_file` se compila solo en nativo; `import_driver(&str)` sigue
    disponible en wasm.
- **Pruebas:** `rejects_mms_not_above_air_mass`,
  `rejects_invalid_air_properties`, `rejects_invalid_numvc_and_vccon`,
  `rejects_negative_le`, `owdr_rejects_invalid_air_numvc_and_mms`, más los
  tests de archivos válidos existentes.

### B.6. API pública, documentación y versionado

- **Problema:** README decía «v1 cerrado» con `Cargo.toml` en `0.1.0`;
  `DesignPoint` no documentaba campos no aplicables; la documentación WASM no
  coincidía con el uso de `std::fs`.
- **Archivos:** `src/design.rs`, `src/lib.rs`, `README.md`, `ROADMAP.md`,
  `docs/INTEGRATION.md`, `docs/VALIDATION.md`, `docs/AGENTS.md`,
  `docs/ALGORITHMS.md`, `Cargo.toml`, `CHANGELOG.md`.
- **Solución:**
  - Versión `0.2.0` pre-1.0; «v1» se define como alcance de validación, no
    SemVer 1.0.
  - README con estado, madurez, partes estables, limitaciones, MSRV, wasm y
    consumo.
  - `DesignPoint`: `za_real`/`za_imag` en `Bandpass` y `port_velocity = None`
    documentados como «no aplicable», no como cero físico.
  - `import_driver(&str)` documentado como ruta wasm;
    `import_driver_file` como nativo.
  - `CHANGELOG.md` con los cambios incompatibles.
- **Pruebas:** `cargo doc -D warnings` y los doc-tests; no hay test de texto,
  la coherencia se verificó por revisión y compilación.

### B.7. `Flare::Parabolic`

- **Problema:** el nombre podía sugerir una parábola matemática.
- **Archivos:** `src/segment.rs`, `docs/ALGORITHMS.md`.
- **Solución:** documentado como **área lineal** entre extremos (perfil `Par`
  del software de referencia); el radio es `sqrt(S(x))`. No se cambió el
  comportamiento físico.
- **Pruebas:** los tests existentes de geometría y producción siguen pasando.

### B.8. Rendimiento

- **Archivos:** `examples/perf_sweep.rs`, `README.md`.
- **Solución:** medición reproducible sin cachés ni paralelismo; resultados en
  la sección F.

### B.9. Correcciones de la revisión adversarial

Una revisión de correctitud con contexto fresco atacó el diff completo. No
encontró blockers; sí un `should-fix` y varias aristas, corregidas en el commit
`b90fad3`:

| Hallazgo | Corrección | Prueba |
|---|---|---|
| OWDR con `numVC`/`VCCon` presente pero no interpretable se degradaba a `1`/`None` | `owdr_integer_field` distingue ausente de malformado | `owdr_rejects_invalid_air_numvc_and_mms` |
| `driver_from_ts` podía devolver un `Driver` derivado con `Mmd`/`Rms`/`Bl` no finitos | `Driver::validate()` sobre el derivado | `rejects_non_finite_derived_driver` |
| `eg` negativo se rechazaba, aunque era la única inversión de polaridad en `System` | `eg` solo debe ser finito; el signo invierte polaridad | `negative_eg_is_valid_and_inverts_phase` |
| `ValidationReport::is_valid` era `true` con métricas infinitas (`eg=0`) | exige métricas finitas | `zero_drive_is_not_a_valid_report` |
| `parse_csv` aceptaba filas con más columnas y podía desplazar una coma decimal | exige exactamente 4 columnas | `csv_parses_and_rejects_invalid_rows` |
| `docs/INTEGRATION.md` decía que `response_at` era más rápido | corregido: delega en `try_response_at` | revisión de texto |

Se mantiene el rechazo de `rear_ql = Some(0.0)`: el valor documentado para
desactivar el `QL` es `None`, no `0`; el campo lo dice explícitamente. Es un
cambio de comportamiento intencional, no un clamp silencioso.

---

## C. Hallazgos no corregidos

### C.1. Comparación de `ze_phase_deg` en `validate`

- **Motivo:** no forma parte del contrato de validación actual ni existe una
  tolerancia validada para la fase de impedancia. Implementarla sin oráculo y
  tolerancia habría declarado una validación no demostrada.
- **Riesgo:** los errores de fase de impedancia no se cuantifican; un
  consumidor podría asumir una validación completa de `Ze`.
- **Mitigación:** la limitación es explícita en `src/validation.rs`, README,
  `docs/VALIDATION.md`, `docs/INTEGRATION.md` y `docs/AGENTS.md`; el campo se
  conserva para trazabilidad.
- **¿Bloquea el cierre?** No, porque el brief permite documentar la limitación
  si la fase no está en el alcance. No se declara validación completa de la
  impedancia.

### C.2. `Flare::Custom` no validable

- **Motivo:** una closure arbitraria no se puede validar sin muestrear todo el
  perfil, lo que cambiaría el contrato y el coste.
- **Riesgo:** una función que devuelva `NaN`/áreas negativas produce resultados
  inválidos.
- **Mitigación:** documentado en `Flare::Custom`; `try_response_at` rechaza los
  resultados no finitos.
- **¿Bloquea el cierre?** No.

### C.3. Limitaciones de alcance ya existentes

- `RadiatorArray` para line arrays: no implementado (fuera de alcance).
- `DriverBank`: sin export multi-driver permanente para test de producción
  (pre-existente; el brief no lo incluye).
- `record.rs` conserva la conversión `nd.round().max(1.0) as u32` del lector
  binario legado; no se tocó porque el hallazgo de conversiones se refería a
  `driver_import.rs` y `nd` es informativo. Riesgo bajo; no bloquea.
- Metadatos opcionales del importador (`Znom`, `Xmax`, `Pe`) no participan en
  el cálculo acústico; un valor inválido se omite. No es un parámetro del
  modelo; no bloquea.

### C.4. `rear_ql = Some(0.0)` ahora es error

- **Motivo:** `None` es el valor documentado para desactivar el `QL`; `Some(0.0)`
  se ignoraba en silencio en `loads`.
- **Riesgo:** una UI que use `0` como sentinela debe cambiar a `None`.
- **¿Bloquea el cierre?** No; es una decisión de compatibilidad explícita.

---

## D. Verificación

### Ejecutado y aprobado

| Comando | Resultado |
|---|---|
| `cargo fmt --all --check` | OK |
| `cargo test --workspace` | 158 tests: 120 librería, 32 integración, 6 doc-tests; 0 fallos |
| `cargo clippy --workspace --all-targets -- -D warnings` | OK |
| `cargo +stable clippy --workspace --all-targets -- -D warnings` | OK (clippy 1.99, el del CI) |
| Revisión adversarial de correctitud (contexto fresco) | 1 `should-fix` + 5 aristas; corregidas en `b90fad3`; sin blockers |
| GitHub Actions del PR #1 | `fmt, check, test, clippy, docs`, MSRV 1.75, wasm32 y GitGuardian: **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | OK |
| `cargo check --lib --target wasm32-unknown-unknown` | OK |
| `cargo +1.75.0 check --workspace --all-targets` | OK |
| `cargo test --test production_accuracy -- --nocapture` | 4 tests OK |
| `cargo test --test system_production -- --nocapture` | 3 tests OK |
| `cargo test --test od_production -- --nocapture` | 3 tests OK |
| `cargo test --test th_production -- --nocapture` | 3 tests OK |
| `cargo test --test bp_production -- --nocapture` | 3 tests OK |
| `cargo run --release --example perf_sweep` | Medición real (sección F) |

Métricas de producción observadas (sin cambios de tolerancias ni calibración):

| Caso | SPL RMS / máx (dB) | Ze RMS / máx (%) | Xd RMS / máx (%) |
|---|---:|---:|---:|
| Default 44 puntos | 1,86 / 3,15 | 7,2 / 23,7 | 9,9 / 28,6 |
| Diseño 2 (533) | 1,94 / 3,23 | 6,0 / 23,9 | 7,1 / 23,6 |
| Diseño 3 (533) | 1,93 / 3,17 | 6,2 / 23,2 | 7,8 / 25,1 |
| Sellada-A | 0,00 / 0,00 | 0,0 / 0,0 | 0,0 / 0,0 |
| Ventilada-A | 0,06 / 0,20 | 0,7 / 3,1 | 0,4 / 2,6 |
| Radiador pasivo-A | 0,38 / 0,97 | 2,5 / 9,3 | 1,0 / 3,6 |
| OD-A | 0,85 / 1,59 | 4,7 / 11,4 | 4,0 / 16,4 |
| OD1-B | 0,85 / 1,77 | 4,7 / 12,0 | 4,1 / 17,2 |
| OD2-C | 0,85 / 1,61 | 4,8 / 11,7 | 4,3 / 18,9 |
| TH1 (S2/S4) | 0,53 / 1,59 | 7,0 / 29,9 | 4,3 / 18,6 |
| TH2 (S2/S4) | 0,52 / 1,63 | 7,1 / 30,9 | 4,5 / 19,0 |
| TH1 (S2/S3) | 0,55 / 2,69 | 6,7 / 15,9 | 3,5 / 15,8 |
| BP4-A | 0,21 / 0,86 | 0,2 / 0,8 | 0,4 / 2,0 |
| BP6P-A | 0,22 / 0,82 | 0,2 / 0,7 | 0,5 / 2,5 |
| BP6S-A | 0,26 / 1,50 | 1,4 / 6,0 | 3,7 / 33,0 |

### Ejecutado y fallido (durante el desarrollo)

- `cargo clippy ... -D warnings` falló dos veces por
  `clippy::neg_cmp_op_on_partial_ord` en `driver_import.rs` y `segment.rs`.
  Se sustituyeron las comparaciones negadas por `<=`/`>=`; la ejecución final
  pasa.
- La revisión adversarial encontró un `should-fix` (OWDR degradaba
  `numVC`/`VCCon` malformados) y aristas de compatibilidad; todas se
  corrigieron y se cubrieron con pruebas en `b90fad3`.
- El CI estable (clippy 1.99) detectó dos `needless_range_loop` en
  `src/tapped.rs` que el clippy nightly local no marcaba. Se justificó el
  índice con `#[allow(clippy::needless_range_loop)]` (el bucle recorre dos
  filas de `y` a la vez) y se verificó con `cargo +stable clippy`.

### No ejecutado

- Windows/macOS nativos: no disponibles en esta máquina.
- Ejecución en runtime wasm: solo se verificó la compilación del target.
- Pruebas de rendimiento en otra máquina: solo x86_64 local.
- Merge del PR: no realizado; queda a revisión del mantenedor.

### No aplicable

- FFI/JNI/Swift/Kotlin: fuera de alcance.
- `RadiatorArray`, topologías nuevas, cachés y paralelismo: fuera de alcance.

---

## E. API y compatibilidad

- **Versión:** `0.2.0`, pre-1.0. La API pública puede cambiar en versiones
  menores; no se declara estabilidad SemVer 1.0.
- **MSRV:** 1.75, verificado con `cargo +1.75.0 check --workspace --all-targets`.
- **Nativo:** compilación y tests verdes.
- **WASM:** `cargo check --lib --target wasm32-unknown-unknown` verde.
  `import_driver(&str)` no usa filesystem; `import_driver_file` solo se compila
  en nativo.
- **Dependencias:** `num-complex`, `libm` y `serde_json`; sin dependencias de
  GUI, sistema, hilos ni tiempo.
- **Cambios incompatibles de 0.2.0:**
  - `OraclePoint::parse_tsv`/`parse_csv` devuelven `Result`.
  - `parse_csv` exige exactamente cuatro columnas.
  - Nuevas variantes en `SolveError` e `ImportError`; nuevo `ValidationError`.
  - `import_driver_file` no existe en wasm.
  - Los constructores ya no recortan valores; hay que usar `validate()` o
    `try_response_at`.
  - `ValidationReport::is_valid` exige métricas finitas.
  - `System::validate` rechaza `rear_ql = Some(0.0)`; use `None`.
- **`eg`:** admite signo; un valor negativo invierte la polaridad. `rg` debe
  ser finito y no negativo.
- **Partes estables en la práctica:** modelos acústicos, unidades SI,
  `response_at`/`try_response_at`, `Design`, calibración validada y datos de
  `tests/data`.
- **Partes que pueden cambiar:** firmas de parser, variantes de error y campos
  de `DesignPoint`.
- **Errores:** `SolveError` distingue frecuencia, parámetro, topología,
  respuesta y sistema singular; `ImportError` distingue formato, campo
  faltante, número inválido, parámetro inválido e inconsistencia;
  `ValidationError` distingue vacío, columnas, celda, frecuencia y valor.

---

## F. Rendimiento

Medición con `examples/perf_sweep.rs`, 20 Hz–200 Hz, barrido logarítmico,
validación incluida. Máquina x86_64 de escritorio; los números son de una
ejecución local, no un benchmark estadístico.

### Release

| Caso | Puntos | Total (ms) | µs/punto |
|---|---:|---:|---:|
| System, bocina 4 tramos | 500 | 44,598 | 89,20 |
| System, bocina 4 tramos | 2.000 | 179,024 | 89,51 |
| TappedHorn 4 tramos | 500 | 29,664 | 59,33 |
| TappedHorn 4 tramos | 2.000 | 118,703 | 59,35 |
| Bandpass BP6 serie | 500 | 10,289 | 20,58 |
| Bandpass BP6 serie | 2.000 | 41,145 | 20,57 |
| 100 cambios × 50 puntos | 5.000 | 443,245 | 88,65 |

### Debug

| Caso | Puntos | Total (ms) | µs/punto |
|---|---:|---:|---:|
| System, bocina 4 tramos | 500 | 443,677 | 887,35 |
| System, bocina 4 tramos | 2.000 | 1.774,052 | 887,03 |
| TappedHorn 4 tramos | 500 | 294,550 | 589,10 |
| TappedHorn 4 tramos | 2.000 | 1.178,541 | 589,27 |
| Bandpass BP6 serie | 500 | 141,326 | 282,65 |
| Bandpass BP6 serie | 2.000 | 565,042 | 282,52 |
| 100 cambios × 50 puntos | 5.000 | 4.399,563 | 879,91 |

**Configuración:** driver `Sd=350 cm²`, `Cms=4e-4`, `Mmd=20 g`, `Re=6 Ω`,
`Bl=18`, `Rms=4`, `Le=1 mH`; bocina de 4 tramos Con/Exp/Con/Con; TH con la
misma geometría; BP6 serie con cámaras de 20/10 L y puertos de 50/40 cm².
Los cambios de parámetro simulan interacción de UI: 100 volúmenes de cámara
distintos, 50 puntos por barrido.

**Evaluación:** un barrido de 2.000 puntos tarda 41–179 ms según topología;
un barrido de 50 puntos para interacción tarda ≈ 4,4 ms. Es razonable para uso
interactivo. El cuello de botella es la integración RK4 por segmento y
frecuencia (hasta 60.000 pasos por matriz), no la validación añadida. No se
introdujeron cachés ni paralelismo.

---

## G. Estado de release

**Lista para consumo experimental como dependencia Rust `0.2.0`.**

Justificación:

- Los modelos acústicos están validados a producción y las métricas no se
  degradaron.
- Los riesgos de la auditoría (resultados parcialmente inválidos, clamping,
  oráculos vacíos, columnas TSV, frecuencias, importadores) están cerrados y
  probados.
- Nativo, wasm y MSRV compilan; fmt, clippy y docs pasan.
- **No** se declara estable porque la versión es pre-1.0, `0.2.0` contiene
  cambios incompatibles y quedan limitaciones explícitas (`ze_phase_deg`,
  `Flare::Custom`, `DriverBank`/`RadiatorArray`).
- **No** se clasifica como «no lista» porque no queda ningún hallazgo
  bloqueante sin corregir.

No se creó tag ni release.

---

## H. Cambios Git

- **Rama:** `stabilization/library-audit`.
- **Base:** `main` en `23a0459`.
- **Commits:** `612c5dc` (estabilización), `b90fad3` (correcciones de la
  revisión adversarial), `0a4fd8f` (informe), `f348d86` (registro del PR) y
  un commit de corrección del CI (`needless_range_loop`).
- **Commit del informe:** `0a4fd8f`.
- **Tag:** ninguno.
- **Push/PR:** rama publicada en `origin`; PR #1:
  <https://github.com/JhonMA82/speakerbox-acoustics/pull/1>. CI remoto verde
  (fmt/check/test/clippy/docs, MSRV, wasm, GitGuardian). `main` intacto.
- **Working tree:** limpio tras los commits, salvo `.rsc/` (estado local del
  harness, añadido a `.gitignore`).

### Archivos modificados

`Cargo.toml`, `Cargo.lock`, `.gitignore`, `README.md`, `ROADMAP.md`,
`docs/AGENTS.md`, `docs/ALGORITHMS.md`, `docs/INTEGRATION.md`,
`docs/VALIDATION.md`, `examples/compare_oracle.rs`, `examples/compare_th.rs`,
`src/air.rs`, `src/bandpass.rs`, `src/design.rs`, `src/driver.rs`,
`src/driver_import.rs`, `src/element.rs`, `src/error.rs`, `src/lib.rs`,
`src/offset.rs`, `src/segment.rs`, `src/system.rs`, `src/tapped.rs`,
`src/validation.rs`, `tests/od_production.rs`, `tests/production_accuracy.rs`,
`tests/system_production.rs`, `tests/th_production.rs`.

### Archivos nuevos

`CHANGELOG.md`, `REVIEW.md`, `examples/perf_sweep.rs`,
`02-DOCS/wiki/**` (spec, plan, progreso, decisiones y perfil de la ejecución
ASDD), `LIBRARY_STABILIZATION_REPORT.md`.

### Resumen de commits

1. `612c5dc` — Estabilización: validación de entrada/resultado, oráculos
   estrictos, frecuencias/barridos, importadores, API/docs/versión `0.2.0`,
   ejemplo de rendimiento.
2. `b90fad3` — Correcciones de la revisión adversarial: OWDR `numVC`/`VCCon`,
   validación del driver derivado, `eg` con signo, `is_valid` con métricas
   finitas, `parse_csv` exacto y nota de rendimiento.
3. `0a4fd8f` — Informe de estabilización.
4. `f348d86` — Registra la publicación del PR #1.
5. Commit de corrección del CI — `needless_range_loop` en `src/tapped.rs`
   (clippy estable 1.99).

El PR queda abierto para revisión y CI. No se hizo merge ni se creó tag.
