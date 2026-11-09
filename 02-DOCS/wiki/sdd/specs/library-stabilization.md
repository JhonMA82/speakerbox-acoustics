# Spec — Estabilización y cierre de `speakerbox-acoustics`

- Fecha: 2026-11-09
- Origen: brief de auditoría técnica del usuario (aprobado; ejecución autopilot).
- Riesgo previsto: tier 1 (código ordinario; sin auth, CI, migraciones ni secretos).
- Rama: `stabilization/library-audit`.

## 1. Qué puede hacer el usuario

- Consumir la librería como dependencia Rust nativa o `wasm32-unknown-unknown`.
- Consultar cualquier topología con `try_response_at` y recibir `Err` cuando una
  entrada o un campo de la respuesta sea inválido.
- Consultar con `response_at` sin manejar errores, recibiendo el punto neutro
  documentado (`spl_db = -∞`) en vez de datos parcialmente inválidos.
- Barrer con `Design::sweep_log`/`Design::try_sweep_log` y `Design::response`
  sin que `NaN` o infinito generen puntos falsos.
- Validar contra un export TSV/CSV y obtener error de parseo o un informe
  explícitamente inválido si no hay datos comparables.
- Importar `.wdr`/`.owdr` desde texto (válido en wasm) o desde disco (nativo) y
  recibir error si los datos son físicamente inconsistentes.

## 2. Qué resultado obtiene (criterios de aceptación)

1. Las cuatro topologías validan todos los campos de respuesta: finitos donde
   corresponde; `ze_mag > 0`; `xd_mm`, potencia y velocidades `>= 0`;
   `spl_db` finito o `-∞` solo por salida nula.
2. `try_response_at` valida parámetros (driver, aire, segmentos, cámaras,
   puertos, radiador pasivo, escalas de calibración, polaridad) antes de calcular.
3. `validate` no aprueba un oráculo vacío; `passes*` devuelve `false` con
   `n == 0` o con puntos inválidos contados.
4. `parse_tsv`/`parse_csv` conservan las columnas y devuelven `Result` con error
   descriptivo: celda inválida, columna faltante, frecuencia no finita o no
   positiva, valor de referencia no finito.
5. Frecuencias: `is_finite() && > 0` en todas las rutas; los barridos validan
   ambos límites; `sweep_log` no inventa puntos con límites inválidos.
6. Importadores: sin fallback del 5 % de Mms; `Mms <= masa de aire` es error;
   `c`/`rho` inválidos son error; `numVC`/`VCCon` se validan antes del cast.
7. API pública documentada: SI, campos no aplicables de `DesignPoint`, errores,
   `import_driver_file` nativo e `import_driver(&str)` wasm.
8. README y `Cargo.toml` coherentes: versión `0.2.0` pre-1.0; «v1» significa
   alcance de validación, no estabilidad SemVer 1.0.
9. Tests nuevos proporcionales a los cambios; producción sin degradación;
   fmt, test, clippy, doc, wasm y MSRV verdes.

## 3. Qué debe respetar

- `AGENTS.md`: SI, `xd_mm` pico, `phase_deg` velocidad, `solid_angle` en sr,
  calibración intacta, sin topologías nuevas, sin copiar código incompatible.
- No cambiar constantes de calibración ni las métricas de producción.
- No añadir dependencias, cachés, paralelismo ni capas.
- No privatizar campos públicos; validar en los puntos de entrada.
- Cambios incompatibles solo los imprescindibles: parsers a `Result`, variantes
  nuevas de error, bump a `0.2.0`.

## 4. Cómo lo prueba

- Tests unitarios por módulo para cada hallazgo: entradas `NaN`/infinito/0/
  negativas, resultados inválidos, oráculos vacíos, celdas TSV, `Mms`
  inconsistente, `numVC`.
- `cargo test --test *_production -- --nocapture` sin tocar tolerancias ni
  calibración; comparación de métricas antes/después.
- Comandos obligatorios de `AGENTS.md`, incluidos wasm y MSRV reales.
- Ejemplo `perf_sweep` reproducible; números reales en el informe.

## Fuera de alcance

- Comparar `ze_phase_deg` en `validate`: no forma parte del contrato actual ni
  tiene tolerancia validada; se documenta la limitación explícitamente.
- `RadiatorArray`, topologías nuevas, FFI, GUI, cachés, paralelismo,
  optimización especulativa.
- Reescritura de módulos y cambios de física o calibración.
