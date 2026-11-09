# Integración de `speakerbox-acoustics`

Guía práctica para usar la librería desde una aplicación nativa o web. Para
las ecuaciones y la validación completa, ver `docs/ALGORITHMS.md`. Para el
mapa del repositorio y las reglas de contribución, ver `docs/AGENTS.md`.

## 1. Añadir la dependencia

```toml
[dependencies]
speakerbox-acoustics = { path = "../speakerbox-acoustics" }
```

El núcleo es Rust puro, sin UI ni dependencias del sistema. Compila para
Linux, macOS, Windows y `wasm32-unknown-unknown`.

```bash
cargo check --lib --target wasm32-unknown-unknown
cargo test --workspace
```

## 2. Modelo mental

```text
Driver ──► topología ──► respuesta
             │
             ├── Segment / Horn        guía de onda
             ├── Chamber               volumen de aire
             ├── Port                  puerto ventilado
             └── PassiveRadiator       radiador pasivo
```

| Tipo | Qué representa | Constructor habitual |
|---|---|---|
| `Driver` | altavoz | `Driver::from_ui_units(...)` |
| `Segment` | tramo de bocina | `Segment::new(flare, largo, área_entrada, área_salida)` |
| `Horn` | bocina completa | `Horn::new(vec![...])` |
| `Chamber` | recinto | `Chamber::new(volumen_m3)` |
| `Port` | puerto | `Port::new(área_m2, largo_m, EndCorrection::...)` |
| `PassiveRadiator` | radiador pasivo | `PassiveRadiator::new(área, cms, mmd, rms)` |
| `System` | radiador directo, sellada, ventilada, pasivo o bocina | `System::new(air, driver)` |
| `TappedHorn` | TH / TH1 | `TappedHorn::new_4seg_th*` |
| `OffsetHorn` | OD / OD1 / OD2 | `OffsetHorn::new_4seg_od*` |
| `Bandpass` | BP4 / BP6P / BP6S | `Bandpass::new_bp4`, `new_bp6`, `new_bp6_series` |
| `Design` | fachada común para la UI | `Design::System(...)`, `Design::Bandpass(...)`, etc. |

## 3. Unidades y convenciones

- **SI** en todo el API: m², m, m³, kg, N/m, T·m, Ω, H, Pa.
- Los constructores `from_ui_units` aceptan las unidades de la UI del software de referencia:
  cm², cm, litros, gramos, mH, mm.
- `spl_db`: presión de potencia a 1 m, dB re 20 µPa.
- `ze_mag`, `ze_phase_deg`: impedancia eléctrica de entrada.
- `xd_mm`: desplazamiento de **pico** (`√2`·RMS) en todas las topologías.
- `solid_angle`: estereorradianes. `4π` espacio libre, `2π` media esfera,
  `π` cuarto de espacio, `π/2` octavo de espacio.
- `phase_deg`: fase de la velocidad volumétrica radiada; la presión lejana
  añade +90°. `Design::response` desenrolla la fase y calcula el retardo de
  grupo.
- `response_at` devuelve un punto neutro si el diseño no es resoluble;
  `try_response_at` devuelve `SolveError`.
- `try_response_at` valida aire, driver, geometría, cámaras, puertos y todos
  los campos del resultado. `response_at` devuelve el punto neutro documentado
  si algo no es válido.
- Los constructores no recortan valores: use `validate()` o `try_response_at`
  para detectar entradas no físicas.
- `validate` compara SPL, `Ze` y `Xd`; **no** compara `ze_phase_deg`.

## 4. Recetas por topología

### 4.1 Caja sellada, ventilada o radiador pasivo

```rust
use speakerbox_acoustics::{
    Air, Chamber, Driver, EndCorrection, PassiveRadiator, Port, System,
};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);

// Sellada: Vb = 50 L.
let mut sealed = System::new(Air::default(), driver);
sealed.solid_angle = std::f64::consts::TAU;
sealed.rear_chamber = Some(Chamber::new(50.0e-3));

// Ventilada: Vb = 100 L, puerto de 100 cm² y 10 cm, QL = 7.
let mut vented = System::new(Air::default(), driver);
vented.solid_angle = std::f64::consts::TAU;
vented.rear_chamber = Some(Chamber::new(100.0e-3));
vented.port = Some(Port::new(
    100.0e-4,
    0.10,
    EndCorrection::Inner(1.5), // corrección calibrada para el software de referencia
));
vented.rear_ql = Some(7.0);

// Radiador pasivo: Vb = 100 L, Sp = 350 cm², Cmp = 0.4 m/N,
// Mmp = 40 g, Rmp = 4 N·s/m.
let mut passive = System::new(Air::default(), driver);
passive.solid_angle = std::f64::consts::TAU;
passive.rear_chamber = Some(Chamber::new(100.0e-3));
passive.passive_radiator =
    Some(PassiveRadiator::new(350.0e-4, 0.4, 40.0e-3, 4.0));
```

`System` suma las presiones complejas del diafragma, del puerto y del
radiador pasivo. Para una caja sellada no hay salidas traseras.

### 4.2 Bocina normal

```rust
use speakerbox_acoustics::{Air, Chamber, Driver, Flare, Horn, Segment, System};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let mut horn = System::new(Air::default(), driver);
horn.solid_angle = 0.5 * std::f64::consts::PI;
horn.horn = Some(Horn::new(vec![
    Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
    Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
]));
horn.throat_chamber = Some(Chamber::new(4.0e-3));
horn.rear_chamber = Some(Chamber::new(14.0e-3));
```

### 4.3 BP4, BP6 paralelo y BP6 serie

```rust
use speakerbox_acoustics::{Bandpass, Driver, EndCorrection, Port};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let port = |area_cm2: f64, length_cm: f64| {
    Port::new(
        area_cm2 * 1.0e-4,
        length_cm * 1.0e-2,
        EndCorrection::UnflangedBoth,
    )
};

let bp4 = Bandpass::new_bp4(
    driver, 20.0e-3, 10.0e-3, port(50.0, 10.0),
);
let bp6p = Bandpass::new_bp6(
    driver, 20.0e-3, 10.0e-3, port(50.0, 10.0), port(40.0, 8.0),
);
let bp6s = Bandpass::new_bp6_series(
    driver, 20.0e-3, 10.0e-3, port(50.0, 10.0), port(40.0, 8.0),
);
```

`Bandpass::set_ql(ql)` aplica el `QL` de un registro. En BP6 serie
se aplica solo a la cámara delantera, que es la que radia.

### 4.4 Tapped horn TH y TH1

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

`new_4seg_th` usa tomas S2/S4; `new_4seg_th1` usa S2/S3.

### 4.5 Offset driver OD / OD1 / OD2

```rust
use speakerbox_acoustics::{Driver, Flare, OffsetHorn, Segment};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let od = OffsetHorn::new_4seg_od(driver, vec![
    Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
    Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
    Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
    Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
]);
```

`new_4seg_od` toma S2, `new_4seg_od1` toma S3 y `new_4seg_od2` toma S4.
Para un driver desplazado sin cámara trasera, deje
`rear_chamber_m3 = None`; el modelo suma el radiador trasero directo.

### 4.6 Bancos de drivers (cajas dobles, torres, isobaric)

`DriverBank` convierte N drivers idénticos en un `Driver` equivalente que los
modelos acústicos existentes ya saben usar:

```rust
use speakerbox_acoustics::{
    Air, Chamber, Driver, DriverBank, Mechanical, System, Wiring,
};

let base = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let bank = DriverBank::new(base, 4)
    .with_wiring(Wiring::SeriesParallel)   // 2 en serie × 2 en paralelo
    .with_mechanical(Mechanical::Standard);
let driver_eq = bank.equivalent_driver().expect("banco válido");

let mut sys = System::new(Air::default(), driver_eq);
sys.solid_angle = std::f64::consts::TAU;
sys.rear_chamber = Some(Chamber::new(40.0e-3));
```

Opciones:

```rust
let bank = DriverBank::new(base, 4)
    .with_wiring(Wiring::Parallel)          // todos en paralelo
    .with_mechanical(Mechanical::Isobaric)  // 2 pares isobáricos
    .with_isobaric_coils(Wiring::Series);    // bobinas del par en serie
```

- `equivalent_driver()` escala `Sd`, `Mmd`, `Cms`, `Rms`, `Re`, `Le` y `Bl`.
- `bank_resistance()` devuelve la resistencia DC en frío que ve el amplificador.
- `suggested_volume_scale()` devuelve `N` para standard/push-pull y `N/4` para
  isobaric, como guía para conservar la alineación.
- `with_dvc(DvcWiring::Parallel | Series)` aplica el cableado de un driver de
  doble bobina. El `Driver` base debe representar **una bobina**.
- `Mechanical::PushPull` no cambia la respuesta lineal; el beneficio es la
  cancelación de distorsión par, que este modelo lineal no calcula.
- `Mechanical::Bessel` y `DvcWiring::Separate` devuelven error: Bessel necesita
  el modelo de array y Separate necesita dos canales independientes.
- El equivalente es **axial y de baja frecuencia**. Para line arrays con
  interferencia y directividad vertical hace falta la capa `RadiatorArray`
  (roadmap).

### 4.7 Importar drivers WinISD `.wdr` y OpenISD `.owdr`

```rust
use speakerbox_acoustics::{import_driver, import_driver_file};

// Desde texto (la app lee el fichero como quiera).
let text = std::fs::read_to_string("driver.wdr").expect("leer driver");
let imported = import_driver(&text).expect("driver válido");
let driver = imported.driver;

// O directamente desde disco (solo nativo; en wasm use el lector de la app).
let imported = import_driver_file("driver.owdr").expect("driver válido");
```

El importador rechaza `Mms <= masa añadida del aire`, `c`/`rho` no positivas y
`numVC`/`VCCon` no enteros; no rellena parámetros físicos para ocultar datos
incorrectos. `import_driver_file` no se compila en wasm; `import_driver(&str)`
sí.

El importador autodetecta el formato:

- `.wdr`: INI con sección `[Driver]`, valores SI, coma o punto decimal.
- `.owdr`: JSON de OpenISD; los campos de `specs.woofer` pueden venir como
  `{ "value": ... }` o como `{ "readings": { ... "read_value": ... } }`.

`ImportedDriver` devuelve:

- `driver`: `Driver` listo para `System`, `Bandpass`, `TappedHorn`,
  `OffsetHorn` o `DriverBank`.
- `format`, `brand`, `model`, `comment`.
- `voice_coils` (`numVC`), `vc_connection` (`VCCon`), `nominal_ohms`,
  `xmax_mm`, `pe_w`.
- `warnings`: campos derivados o valores anómalos.

Conversión de unidades y derivación:

- `Cms` se toma del fichero o se deriva de `Vas` y `Sd`.
- `Mms` se toma del fichero o se deriva de `Fs` y `Cms`.
- `Rms` se deriva de `Qms`; `Bl` se deriva de `Qes`.
- `Mmd` se calcula como `Mms` menos la masa añadida del aire, porque el
  modelo acústico ya incluye la radiación.
- Si el fichero declara `numVC=2`, el `Driver` importado es el compuesto con
  el `VCCon` del archivo. Para cambiar el cableado use `DriverBank` con
  `DvcWiring` y un `Driver` base de una bobina.

Ejemplo ejecutable:

```bash
cargo run --release --example import_driver -- ruta/driver.wdr
```

## 5. Barrido y uso desde una UI

La fachada `Design` unifica todas las topologías:

```rust
use speakerbox_acoustics::{Design, Driver, Flare, Segment, TappedHorn};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let th = TappedHorn::new_4seg_th(driver, vec![
    Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
    Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
    Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
    Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
]);
let design = Design::TappedHorn(th);

for p in design.sweep_log(20.0, 200.0, 12) {
    let _spl = p.spl_db;
    let _ze = p.ze_mag;
    let _xd_peak = p.xd_mm;
}
```

`Design::response_at` y `Design::response` devuelven `DesignPoint`.
`response` desenrolla la fase y calcula el retardo de grupo.

| Campo | Significado |
|---|---|
| `freq_hz` | frecuencia (Hz) |
| `spl_db` | SPL de potencia a 1 m (dB re 20 µPa) |
| `phase_deg` | fase de la velocidad volumétrica radiada (grados) |
| `group_delay_ms` | retardo de grupo (ms), calculado por `response` |
| `ze_mag`, `ze_phase_deg` | impedancia eléctrica de entrada |
| `xd_mm` | desplazamiento de pico del diafragma (mm) |
| `power_w` | potencia acústica radiada (W) |
| `efficiency_pct` | rendimiento acústico (%) |
| `za_real`, `za_imag` | impedancia acústica de referencia (Pa·s/m³); `0.0` en `Bandpass` significa «no aplicable» |
| `port_velocity` | velocidad máxima de puerto (m/s); `None` significa «no aplicable» |

`Design::response_at` devuelve un punto neutro si el diseño no es resoluble;
`Design::try_response_at` propaga `SolveError`. `Design::validate()` valida la
topología. `sweep_log` devuelve un vector vacío si `fmin`/`fmax` no son
finitos y positivos, `fmax <= fmin` o `points < 2`; `try_sweep_log` devuelve
el error correspondiente.

## 6. Calibración y validación

La calibración empírica `Calibration::validated_60_70()` está pensada para el
registro binario de 734 bytes de la aplicación de referencia:

```rust
use speakerbox_acoustics::{Calibration, LegacyRecord};

fn load(path: &str) -> Result<speakerbox_acoustics::System, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let record = LegacyRecord::parse(&bytes)?;
    Ok(record.to_system_with(&Calibration::validated_60_70()))
}
```

Para validar un diseño contra un export de Hornresp:

```rust
use speakerbox_acoustics::{validate, Calibration, LegacyRecord, OraclePoint};

fn validate_export(
    record_path: &str,
    export_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(record_path)?;
    let record = LegacyRecord::parse(&bytes)?;
    let system = record.to_system_with(&Calibration::validated_60_70());
    let oracle = OraclePoint::parse_tsv(&std::fs::read_to_string(export_path)?)?;
    let report = validate(&system, &oracle);
    println!("SPL RMS = {:.2} dB", report.spl_rmse_db);
    Ok(())
}
```

Las tolerancias de producción usadas en este repositorio son:

- SPL: RMS ≤ 2 dB, máximo ≤ 3.5 dB.
- `Ze`: RMS ≤ 15 %, máximo ≤ 35 %.
- `Xd`: RMS ≤ 15 %, máximo ≤ 40 % (se ignoran puntos con `Xd ≤ 0.02 mm`).

La calibración es **empírica y ligada a la geometría validada**. Para otro
diseño, genere su propio export de Hornresp y valide con `validate`; no
suponga que los factores de calibración son universales.

`parse_tsv`/`parse_csv` devuelven `Result<Vec<OraclePoint>, ValidationError>`:
conservan la posición de cada columna y rechazan celdas obligatorias vacías o
no finitas, frecuencias no positivas y valores de referencia no físicos. Un
oráculo vacío no produce una lista vacía silenciosa. `validate` cuenta los
puntos rechazados en `report.invalid_points`; `passes`/`passes_pointwise`
exigen `report.is_valid()` (`n > 0` e `invalid_points == 0`). La fase de
impedancia (`ze_phase_deg`) se parsea pero **no se compara**.

## 7. Errores y robustez

```rust
use speakerbox_acoustics::{Bandpass, Driver, EndCorrection, Port, SolveError};

let driver = Driver::from_ui_units(
    350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1,
);
let bp = Bandpass::new_bp4(
    driver,
    20.0e-3,
    10.0e-3,
    Port::new(50.0e-4, 0.10, EndCorrection::UnflangedBoth),
);
match bp.try_response_at(50.0) {
    Ok(p) => println!("SPL = {:.2} dB", p.spl_db),
    Err(SolveError::InvalidFrequency(f)) => eprintln!("frecuencia inválida: {f}"),
    Err(e) => eprintln!("diseño no resoluble: {e}"),
}
```

Use `try_response_at` en bucles de UI donde el usuario puede dejar un campo
inválido. `response_at` es más rápido y devuelve un punto neutro
(`spl_db = -∞`) en esos casos. `try_response_at` distingue:

- `InvalidFrequency`: frecuencia no finita o `<= 0`.
- `InvalidParameter`: campo de entrada no finito o fuera de rango físico.
- `InvalidTopology`: configuración incoherente (toma fuera de rango, BP6 sin
  puerto trasero, puerto sin cámara, etc.).
- `InvalidResponse`: campo del resultado no finito o no físico.
- `SingularSystem`: sistema lineal singular o mal condicionado.

Llame a `validate()` en el tipo de topología si quiere comprobar la
configuración una sola vez antes de un barrido.

## 8. WASM y nativo

El núcleo de cálculo no usa hilos ni tiempo del sistema. Compila para wasm sin
cambios; la única función de filesystem (`import_driver_file`) solo se compila
en nativo:

```bash
cargo check --lib --target wasm32-unknown-unknown
```

Para wasm, lea el texto con la API del navegador y use `import_driver(&str)`.
Los tipos principales implementan `Send + Sync`; puede mover un `Design` a un
worker o hilo de cálculo y devolver los `DesignPoint` a la UI.

## 9. Checklist de integración

1. Elija la topología y construya el `Driver` con `from_ui_units` o `new`.
2. Use SI en el API; para la UI puede convertir desde cm²/cm/L/g/mH/mm.
3. Fije `solid_angle` (`2π` para media esfera, `π/2` para octavo de espacio).
4. Fije `eg`, `rg` y, si aplica, `spl_offset_db`/calibración.
5. Valide la configuración con `validate()` y barra con `Design::sweep_log` o
   `try_sweep_log`; use `try_response_at` para errores de usuario.
6. Recuerde que `xd_mm` es pico y que `phase_deg` es fase de velocidad.
7. Valide contra un export propio de Hornresp antes de llamar “producción” a
   una geometría nueva; recuerde que `validate` no compara la fase de
   impedancia.

## 10. Dónde mirar

- `README.md`: visión general, estado y uso de la librería.
- `docs/ALGORITHMS.md`: ecuaciones, calibración y tablas de validación.
- `docs/AGENTS.md`: mapa del repositorio, comandos e invariantes para agentes.
- `ROADMAP.md`: alcance v1 y topologías diferidas.
- `tests/data/README.md`: oráculos y parámetros de los diseños validados.
- `examples/quickstart.rs`: recorrido ejecutable por todas las topologías.
