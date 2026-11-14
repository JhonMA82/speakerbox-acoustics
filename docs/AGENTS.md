# Guía para agentes

Este documento describe el repositorio, los comandos, las invariantes y el
flujo para extender la librería sin romper la validación de producción.

## Estado actual

**v1 cerrado.** Las topologías del alcance v1 están validadas contra exports
propios de Hornresp 60.70:

- Bocina normal.
- OD / OD1 / OD2.
- TH (S2/S4) y TH1 (S2/S3).
- BP4, BP6 paralelo y BP6 serie.
- Caja sellada, ventilada y radiador pasivo.

No se aceptan topologías nuevas sin modelo, export propio y test de producción
puntual. Ver `ROADMAP.md` para lo diferido.

## Mapa del repositorio

| Ruta | Responsabilidad |
|---|---|
| `src/air.rs` | propiedades del aire |
| `src/driver.rs` | modelo electromecánico del altavoz |
| `src/driver_bank.rs` | bancos de drivers: N, wiring, isobaric, DVC |
| `src/driver_import.rs` | importación WinISD `.wdr` / OpenISD `.owdr` |
| `src/segment.rs` | Webster, matrices 2×2, perfiles y `Horn` |
| `src/element.rs` | `Chamber`, `Port`, `PassiveRadiator`, `Lining`, `EndCorrection` |
| `src/system.rs` | red completa, radiador directo, caja, bocina, puerto, PR |
| `src/tapped.rs` | solver nodal de TH/TH1 |
| `src/offset.rs` | stub/path de OD/OD1/OD2 y radiador trasero directo |
| `src/bandpass.rs` | BP4/BP6P/BP6S |
| `src/design.rs` | fachada `Design`/`DesignPoint` para la UI |
| `src/calibration.rs` | calibración empírica `validated_60_70` |
| `src/record.rs` | lector del registro binario de 734 bytes |
| `src/validation.rs` | `OraclePoint`, `validate`, tolerancias |
| `tests/*_production.rs` | validación puntual por topología |
| `tests/data/` | registros y oráculos propios |
| `docs/INTEGRATION.md` | guía para integrar la librería |
| `docs/ALGORITHMS.md` | ecuaciones y calibración |
| `docs/VALIDATION.md` | tablas detalladas de producción |
| `docs/RECORD_FORMAT.md` | formato del registro de 734 bytes |
| `crates/speakerbox-circuits/` | análisis nodal de redes eléctricas pasivas (MNA, `.CIR`) |
| `ROADMAP.md` | alcance v1 y topologías diferidas |

El repositorio es un `Cargo Workspace`: el paquete raíz sigue siendo
`speakerbox-acoustics` y los subsistemas nuevos viven en `crates/`. El crate
eléctrico no depende del acústico ni al revés, así que una dependencia circular
es imposible por construcción.

## Comandos obligatorios

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo check --lib --target wasm32-unknown-unknown
cargo +1.75.0 check --workspace --all-targets
```

Para ver las métricas de una topología:

```bash
cargo test --test od_production -- --nocapture
cargo test --test th_production -- --nocapture
cargo test --test bp_production -- --nocapture
cargo test --test system_production -- --nocapture
cargo test --test production_accuracy -- --nocapture
```

## Flujo de datos

```text
Driver + geometría
      │
      ▼
System / TappedHorn / OffsetHorn / Bandpass
      │  response_at(f)
      ▼
ResponsePoint / TappedPoint / OffsetPoint / BandpassPoint
      │  Design
      ▼
DesignPoint (SPL, Ze, Xd, fase, retardo, potencia, puerto)
```

## Convenciones que no se deben romper

- **SI** en el API público. Los `from_ui_units` aceptan cm², cm, L, g, mH, mm.
- `spl_db` es presión de potencia a 1 m, dB re 20 µPa.
- `xd_mm` es desplazamiento de **pico** (`√2`·RMS) en todas las topologías.
- `phase_deg` es fase de presión reducida por j (en OD, suma total);
  la presión física añade +90°.
- `eg` puede ser negativo (invierte la polaridad); `rg` debe ser finito y no
  negativo.
- `solid_angle` en estereorradianes (`4π`, `2π`, `π`, `π/2`).
- `response_at` devuelve un punto neutro si no es resoluble;
  `try_response_at` devuelve `SolveError` y valida entrada, configuración y
  todos los campos del resultado.
- Los constructores no recortan valores; `validate()` rechaza parámetros no
  finitos o no físicos.
- `Design::response` desenrolla la fase y calcula el retardo de grupo; no
  duplique esa lógica en la UI.
- `Design::sweep_log` devuelve vacío ante límites o número de puntos inválidos;
  `try_sweep_log` devuelve el `SolveError`.
- La calibración es empírica y está ligada a la geometría validada. No cambie
  `spl_offset_db`, `mouth_radiation_scale`, `mouth_radiation_area_factor` ni
  `rear_radiation_scale` sin revalidar los tests de producción.

## Cómo añadir una topología

1. Implemente el modelo en `src/` con constructores claros y sin `unsafe`.
2. Genere un registro y un `All Chart Data` propios de Hornresp 60.70.
3. Guarde los datos en `tests/data` con nombres `topologia_a_record.txt` y
   `topologia_a_oracle.txt` (o `_spl`, `_ze`, `_xd`).
4. Añada un test `tests/<topologia>_production.rs` con las tolerancias
   puntuales:
   - SPL RMS ≤ 2 dB, máximo ≤ 3,5 dB.
   - `Ze` RMS ≤ 15 %, máximo ≤ 35 %.
   - `Xd` RMS ≤ 15 %, máximo ≤ 40 % (ignorar `Xd ≤ 0.02 mm`).
5. Documente ecuaciones, calibración y métricas en `README.md`,
   `docs/ALGORITHMS.md` y `tests/data/README.md`.
6. Actualice `ROADMAP.md` si cambia el alcance.
7. Ejecute todos los comandos obligatorios.

Sin oráculo propio, la topología queda como **experimental** y no entra en v1.

## Validación de producción

- Los oráculos son exports propios, nunca datos de proyectos con licencia
  incompatible.
- `OraclePoint::parse_tsv` acepta el `All Chart Data` de Hornresp con coma o
  punto decimal y devuelve `Result`; conserva la posición de las columnas y
  rechaza celdas inválidas o columnas faltantes.
- `validate` compara SPL, `Ze` y `Xd`; `Xd` ya es pico en todos los puntos.
- `validate` **no** compara `ze_phase_deg`; no declare una validación completa
  de la impedancia.
- Un oráculo vacío o con puntos inválidos no aprueba: `passes`/`passes_pointwise`
  exigen `n > 0`, `invalid_points == 0` y métricas finitas.
- `parse_csv` exige exactamente 4 columnas; `parse_tsv` usa índices fijos y
  admite columnas extra.
- `passes_pointwise` aplica las tolerancias puntuales, no solo RMS.
- Los tests de producción deben usar la misma calibración global para todos
  los diseños de la topología.

## Trampas conocidas

- **No** añada líneas crudas como `TH1 = 1` a un registro de texto para
  importarlo: el software de referencia puede crashear. El flag TH1 se activa en la UI con
  doble clic en `TH`; el registro exportado ya trae `TH1 = 1`.
- Los registros exportados usan **CRLF** y a veces coma decimal. Use
  `OraclePoint::parse_tsv` o un parser que tolere ambos.
- En OD, `Vrc` no actúa como cámara sellada en los registros validados: la
  parte trasera es un radiador directo (`rear_chamber_m3 = None`).
- En BP6S, el `QL` se aplica solo a la cámara delantera y el puerto trasero
  lleva corrección de salida embridada (`0.8216·a`).
- En cajas ventiladas, el puerto usa `EndCorrection::Inner(1.5)` en la
  validación; no confunda la corrección del puerto con la del radiador pasivo.
- El registro del radiador pasivo puede guardar `Cmp` como `4.00E-01`; use el
  valor del registro, no el que se haya pedido en la especificación.
- `System` reporta `Xd` de pico; `validation` ya no multiplica por `√2`.
- No cambie la firma pública de `DesignPoint` sin actualizar `design.rs`,
  `docs/INTEGRATION.md` y los ejemplos.
- En `.wdr` (WinISD) los valores son SI y el separador decimal puede ser coma;
  `Mms` es masa total, así que el importador resta la masa añadida del aire.
- En `.owdr` (OpenISD) cada campo puede venir como `value` o dentro de
  `readings[].read_value`; el importador acepta ambos.
- No commitee ficheros `.wdr`/`.owdr` de terceros sin licencia; los tests del
  importador usan cadenas sintéticas propias.

## Estilo

- Rust estable, MSRV 1.75, sin dependencias nuevas salvo necesidad clara.
- `cargo fmt` y `clippy -D warnings` son obligatorios.
- Documentación y comentarios en español, consistentes con el resto del
  repositorio.
- Cambios mínimos y reversibles; no reescriba módulos que ya están validados.
- Los ejemplos deben compilar con `cargo check --all-targets`.

## Auditoría física correctiva

Consultar `docs/audits/ACOUSTIC_AUDIT.md` y sus cuatro informes asociados.
SPL/Ze/Xd siguen pasando sus oráculos; potencia/rendimiento legados son
estimaciones con limitaciones medidas, no un balance energético general.
No cambiar calibración ni ampliar las garantías sin resolver ese modelo.

## Segunda pasada energética

Estado vigente: docs/audits/RADIATION_MODEL_CONTRACT.md. Los campos legados
power_w/acoustical_power_w son absorción de cargas explícitas (pueden ser
subtotales), no potencia de campo. try_energy_at conserva calidad radiativa,
fuente/bornes/pérdidas y eficiencias opcionales. Matched no define SPL; fase
legada NaN en silencio y eficiencia NaN para 0/0 son ausencias deliberadas.
Conservar calibración, oráculos y optimizaciones de la primera pasada.
