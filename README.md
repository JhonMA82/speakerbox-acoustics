# speakerbox-acoustics

Motor de cálculo acústico en **Rust**: bocinas por segmentos, radiador
directo, cajas sellada/ventilada, radiador pasivo, tapped horn, offset driver y
bandpass. El núcleo es UI-agnóstico y compila para
nativo y `wasm32-unknown-unknown`.

> **Para integrar la librería:** [`docs/INTEGRATION.md`](docs/INTEGRATION.md) y
> [`examples/quickstart.rs`](examples/quickstart.rs).
>
> **Para mantenerla o extenderla:** [`AGENTS.md`](AGENTS.md) y
> [`docs/AGENTS.md`](docs/AGENTS.md).
>
> **Validación detallada:** [`docs/VALIDATION.md`](docs/VALIDATION.md).

## Estado

**Alcance v1 de topologías cerrado**, validado a producción contra exports
propios de Hornresp 60.70. La crate está en **0.2.0, pre-1.0**: «v1» describe
el alcance de validación, no una promesa de estabilidad SemVer 1.0.

- Bocina normal.
- Offset driver OD / OD1 / OD2.
- Tapped horn TH (S2/S4) y TH1 (S2/S3).
- BP4, BP6 paralelo y BP6 serie.
- Caja sellada, ventilada y radiador pasivo.

La fase 3 añade `DriverBank` (N drivers, cableado, isobaric, push-pull, DVC),
la importación de drivers WinISD `.wdr` / OpenISD `.owdr` y deja pendiente el
array espacial `RadiatorArray` para line arrays.

## Versión y estabilidad

- **Versión actual:** `0.2.0`, pre-1.0.
- **Madurez:** los modelos acústicos y sus oráculos de producción están
  validados; la API pública todavía puede cambiar en versiones menores.
- **Estable en la práctica:** constructores de topologías, unidades SI,
  `response_at`/`try_response_at`, `Design`, calibración validada y datos de
  `tests/data`.
- **Puede cambiar:** firmas de `OraclePoint::parse_tsv`/`parse_csv`, variantes
  de `SolveError`/`ImportError`/`ValidationError` y campos de `DesignPoint`.
- **Rust:** MSRV 1.75. **WASM:** `wasm32-unknown-unknown` verificado en CI.
- **Limitaciones conocidas:** `validate` no compara la fase de impedancia
  (`ze_phase_deg`); la calibración es empírica y ligada a la geometría
  validada; `DriverBank` aún no tiene export multi-driver permanente;
  `RadiatorArray` no está implementado.

El historial de cambios está en [`CHANGELOG.md`](CHANGELOG.md).

## Qué incluye

- **Altavoz**: `Re`, `Le`, `Bl`, `Mmd`, `Cms`, `Rms`, `Sd`; T/S (`Fs`, `Qms`,
  `Qes`, `Qts`, `Vas`, `η0`, SPL 1 W / 2.83 V), desplazamiento y masa añadida
  del aire.
- **Segmentos**: cilíndrico, cónico, exponencial, parabólico,
  hiperbólico-exponencial y perfil arbitrario `Custom(S(x))`.
- **Matriz 2×2** `[p, U]` por integración RK4 de Webster, con paso adaptado a
  la frecuencia.
- **Radiación**: pistón con Bessel `J1` y Struve `H1`; ángulos sólidos 4π, 2π,
  π y π/2.
- **Recintos, puerto y radiador pasivo**: compliancias, pérdidas, correcciones
  de extremo y modelo de tubo con resonancia de cuarto de onda.
- **Bocina normal**: `System` + `Horn`, con cámara de garganta y recinto
  trasero.
- **Tapped horn**: TH (S2/S4) y TH1 (S2/S3), solver nodal y fuente Norton.
- **Offset driver**: OD/OD1/OD2 con stub cerrado, camino a la boca y radiador
  trasero directo.
- **Bandpass**: BP4, BP6 paralelo y BP6 serie, con `Ql` y puerto como tubo o
  masa concentrada.
- **Bancos de drivers**: `DriverBank` para N drivers, paralelo/serie/
  serie-paralelo, isobaric, push-pull y DVC paralelo/serie.
- **Importación**: drivers WinISD `.wdr` y OpenISD `.owdr`, con derivación de
  T/S y metadatos.
- **Fachada `Design`**: `System`, `TappedHorn`, `OffsetHorn` y `Bandpass` bajo
  un `DesignPoint` común, con barrido logarítmico, fase desenrollada y retardo
  de grupo.
- **Salidas**: SPL, fase, retardo, `Ze`, impedancia acústica, `Xd`, potencia,
  rendimiento y velocidad de puerto.
- **Registro de 734 bytes**: lector del `.dat` binario de la aplicación de referencia.

## Instalación

```toml
[dependencies]
speakerbox-acoustics = { path = "../speakerbox-acoustics" }
```

Nativo y wasm:

```bash
cargo test --workspace
cargo check --lib --target wasm32-unknown-unknown
```

## Uso rápido

### Driver y caja

```rust
use speakerbox_acoustics::{Air, Chamber, Driver, System};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let mut sys = System::new(Air::default(), driver);
sys.solid_angle = std::f64::consts::TAU;
sys.rear_chamber = Some(Chamber::new(50.0e-3));

let p = sys.response_at(50.0);
println!("{:.2} dB SPL, Ze = {:.2} Ω", p.spl_db, p.ze_mag);
```

Para caja ventilada añada `Port` y `rear_ql`; para radiador pasivo añada
`PassiveRadiator`. Recetas completas en
[`docs/INTEGRATION.md`](docs/INTEGRATION.md).

### Bandpass

```rust
use speakerbox_acoustics::{Bandpass, Driver, EndCorrection, Port};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let port = Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth);
let bp4 = Bandpass::new_bp4(driver, 20.0e-3, 10.0e-3, port);
let p = bp4.response_at(50.0);
```

`Bandpass::new_bp6` construye el paralelo; `new_bp6_series` el serie.

### Tapped horn y offset driver

```rust
use speakerbox_acoustics::{Driver, Flare, Segment, TappedHorn};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let segments = vec![
    Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
    Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
    Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
    Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
];
let th = TappedHorn::new_4seg_th(driver, segments.clone());
let th1 = TappedHorn::new_4seg_th1(driver, segments);
```

`OffsetHorn::new_4seg_od`, `new_4seg_od1` y `new_4seg_od2` cubren OD/OD1/OD2.

### Bancos de drivers

```rust
use speakerbox_acoustics::{Driver, DriverBank, Wiring};

let base = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let bank = DriverBank::new(base, 4).with_wiring(Wiring::SeriesParallel);
let equivalent = bank.equivalent_driver().expect("banco válido");
```

El equivalente se pasa a `System`, `Bandpass`, `TappedHorn` u `OffsetHorn`.
Soporta isobaric, push-pull y DVC; Bessel y DVC Separate necesitan capas
adicionales.

### Importar drivers WinISD / OpenISD

```rust
use speakerbox_acoustics::import_driver;

let text = std::fs::read_to_string("driver.wdr").expect("leer driver");
let imported = import_driver(&text).expect("driver válido");
let driver = imported.driver;
```

Autodetecta `.wdr` y `.owdr`; `import_wdr`/`import_owdr` fuerzan el formato.
La importación desde texto funciona en nativo y wasm; `import_driver_file` lee
desde disco y solo se compila en nativo. Un `Mms` que no supera la masa de
aire, propiedades del aire no positivas o un `numVC`/`VCCon` no entero
devuelven error en lugar de fabricar parámetros.

### Barrido con `Design`

```rust
use speakerbox_acoustics::{Bandpass, Design, Driver, EndCorrection, Port};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let bp = Bandpass::new_bp4(
    driver,
    20.0e-3,
    10.0e-3,
    Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth),
);
let design = Design::Bandpass(bp);
for p in design.sweep_log(20.0, 200.0, 12) {
    println!("{:.1} Hz: {:.2} dB", p.freq_hz, p.spl_db);
}
```

`response_at` devuelve un punto neutro si el diseño no es resoluble;
`try_response_at` devuelve `SolveError`.

## Validación

Tolerancias puntuales: SPL RMS ≤ 2 dB / máximo ≤ 3,5 dB; `Ze` ≤ 15 % RMS /
35 % puntual; `Xd` ≤ 15 % RMS / 40 % puntual.

`validate` compara SPL, `Ze` y `Xd`. **No compara la fase de impedancia**
(`ze_phase_deg`): el campo se parsea y se conserva, pero no hay tolerancia
validada. Un oráculo vacío, incompleto o con valores no físicos no puede
aprobar: `passes`/`passes_pointwise` exigen `n > 0` e `invalid_points == 0`.
`OraclePoint::parse_tsv`/`parse_csv` devuelven `Result` y conservan la
correspondencia de columnas.

| Topología | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| Bocina normal | 1,91–1,94 / 3,15–3,23 dB | 6,0–6,4 / 23,2–26,8 % | 7,1–9,3 / 23,6–35,3 % |
| TH (S2/S4 y S2/S3) | 0,52–0,55 / 1,59–2,69 dB | 6,7–7,1 / 15,9–30,9 % | 3,5–4,5 / 15,8–19,0 % |
| OD / OD1 / OD2 | 0,85 / 1,59–1,77 dB | 4,7–4,8 / 11,4–12,0 % | 4,0–4,3 / 16,4–18,9 % |
| BP4 / BP6P / BP6S | 0,21–0,26 / 0,82–1,50 dB | 0,2–1,4 / 0,7–6,0 % | 0,4–3,7 / 2,0–33,0 % |
| Caja sellada / ventilada / PR | 0,00–0,38 / 0,00–0,97 dB | 0,0–2,5 / 0,0–9,3 % | 0,0–1,0 / 0,0–3,6 % |

Detalle por diseño, bandas y calibración:
[`docs/VALIDATION.md`](docs/VALIDATION.md). Datos y oráculos:
[`tests/data/README.md`](tests/data/README.md).

`DriverBank` está validado localmente contra los casos `Nd = 2P/2S` de
Hornresp (SPL ≈ 0,3 dB RMS, `Ze`/`Xd` ≈ 3–4 % RMS); falta un export propio
multi-driver para el test de producción permanente.

## Rendimiento

Medición reproducible con `examples/perf_sweep.rs` (20 Hz–200 Hz, barrido
logarítmico, validación incluida):

```bash
cargo run --release --example perf_sweep
```

En un x86_64 de escritorio, release: bocina de 4 tramos ≈ 89 µs/punto
(179 ms para 2.000 puntos), tapped horn ≈ 59 µs/punto (119 ms), BP6 serie
≈ 21 µs/punto (41 ms). Cien cambios de parámetro con barridos de 50 puntos
tardan ≈ 443 ms (≈ 4,4 ms por barrido). Para uso interactivo es razonable;
el cuello de botella es la integración RK4 por segmento y frecuencia, no la
validación. No hay caché ni paralelismo.

## Documentación

| Documento | Contenido |
|---|---|
| [`docs/INTEGRATION.md`](docs/INTEGRATION.md) | Guía de integración y recetas por topología |
| [`examples/quickstart.rs`](examples/quickstart.rs) | Recorrido ejecutable por todas las topologías |
| [`examples/import_driver.rs`](examples/import_driver.rs) | Importar `.wdr` / `.owdr` |
| [`docs/ALGORITHMS.md`](docs/ALGORITHMS.md) | Ecuaciones y calibración |
| [`docs/VALIDATION.md`](docs/VALIDATION.md) | Métricas detalladas de producción |
| [`docs/RECORD_FORMAT.md`](docs/RECORD_FORMAT.md) | Formato del registro de 734 bytes |
| [`AGENTS.md`](AGENTS.md) / [`docs/AGENTS.md`](docs/AGENTS.md) | Guía para agentes y extensión |
| [`ROADMAP.md`](ROADMAP.md) | Alcance v1 y topologías diferidas |
| [`CHANGELOG.md`](CHANGELOG.md) | Historial de versiones |

## Estructura

```text
src/
  air.rs, driver.rs, driver_bank.rs, driver_import.rs
  segment.rs, element.rs, system.rs
  tapped.rs, offset.rs, bandpass.rs
  design.rs, validation.rs, record.rs, calibration.rs
examples/   quickstart, bandpass_demo, import_driver, horn_demo, compare_*,
            design_demo
tests/      engine, bandpass, tapped, production_accuracy, driver_bank,
            driver_import, th_production, od_production, bp_production,
            system_production
docs/       INTEGRATION, ALGORITHMS, VALIDATION, RECORD_FORMAT, AGENTS
tests/data/ registros y oráculos propios
```

## Compatibilidad

- **MSRV 1.75**: `cargo +1.75.0 check --workspace --all-targets`.
- **wasm32**: `cargo check --lib --target wasm32-unknown-unknown`.
- **Importación wasm:** `import_driver(&str)` no toca el filesystem;
  `import_driver_file` solo existe en nativo.
- **CI**: fmt, check, tests, clippy, docs, MSRV y wasm.
- **Licencia dual**: MIT OR Apache-2.0.

## Origen

Implementación independiente basada en ecuaciones públicas de acústica
(Webster, radiación de pistón, T/S y matrices de transmisión). Se validó contra
exports propios de Hornresp 60.70; no se copió código de Hornresp ni de otros
repositorios. Los únicos datos externos son los exports propios usados como
oráculo de validación. El formato del registro de 734 bytes está documentado en
[`docs/RECORD_FORMAT.md`](docs/RECORD_FORMAT.md).
