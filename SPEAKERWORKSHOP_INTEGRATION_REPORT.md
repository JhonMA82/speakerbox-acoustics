# Informe de integración — Speaker Workshop, Fase A + B

**Fecha:** 2026-11-13
**Rama:** `feat/speakerworkshop-integration`
**Worktree:** `.worktrees/feat/speakerworkshop-integration`
**Base:** `c1e79e1` (`main`)
**Alcance:** Fase A (baseline, contratos, RFC) y Fase B (red RLC, solver MNA,
parser `.CIR`). Las fases C, D y E no se implementan aquí.

## 1. Baseline registrada

`cargo test --workspace` sobre `c1e79e1`, antes de tocar código: **verde, 0
fallos**. No se editó ningún archivo de `src/`, `tests/` ni `tests/data/` del
paquete raíz; el `git diff --stat` de la rama solo muestra `Cargo.toml`,
`Cargo.lock`, documentación y archivos nuevos.

## 2. Cambios por archivo

| Archivo | Cambio |
|---|---|
| `Cargo.toml` | `[workspace]` con `members = ["crates/speakerbox-circuits"]` y `resolver = "2"`. El `[package]` raíz no cambia. |
| `Cargo.lock` | Añade el miembro `speakerbox-circuits`. |
| `crates/speakerbox-circuits/Cargo.toml` | Crate nuevo, MSRV 1.75, licencia dual, dependencia única `num-complex`. |
| `crates/speakerbox-circuits/src/error.rs` | `CircuitError` y `CircuitResult`, sin pánicos en rutas públicas. |
| `crates/speakerbox-circuits/src/model.rs` | Nodos, R/L/C, cargas constantes o medidas, fuentes ideales y validación. |
| `crates/speakerbox-circuits/src/solver.rs` | MNA complejo, pivoteo parcial, residual y `Solution`. |
| `crates/speakerbox-circuits/src/analysis.rs` | Barrido, `Zin`, transferencia y curva de transferencia. |
| `crates/speakerbox-circuits/src/curves.rs` | `ComplexCurve` con interpolación logarítmica y extrapolación prohibida. |
| `crates/speakerbox-circuits/src/cir.rs` | Parser/serializador `.CIR` con sufijos SI y errores por línea. |
| `crates/speakerbox-circuits/tests/` | 32 pruebas de integración y fixture `series_rlc.CIR`. |
| `crates/speakerbox-circuits/examples/ladder_sweep.rs` | Ejemplo ejecutable. |
| `docs/SPEAKERWORKSHOP_REUSE_RFC.md` | RFC de fronteras, contratos y licencia. |
| `docs/AGENTS.md`, `README.md`, `CHANGELOG.md` | Documentación del workspace y del crate. |
| `02-DOCS/wiki/sdd/specs|plans/speakerworkshop-integration.md` | Spec y plan de la ejecución. |

## 3. Fronteras entre crates

```text
speakerbox-acoustics (raíz, API intacta)
└── sin dependencia de crates/

crates/speakerbox-circuits
├── depende de num-complex
└── no depende de speakerbox-acoustics
```

No hay arista entre los dos paquetes: la dependencia circular es imposible por
construcción, no por convención.

## 4. Diferencias con el legacy

| Legacy | Aquí |
|---|---|
| `matrix_Invert` seguido de `1/Z` sin comprobar el retorno | eliminación gaussiana con `Result`; pivote nulo = `Singular` |
| pivote = primer valor distinto de cero | pivoteo parcial por magnitud |
| error como `BOOL`, mensajes modales y estado global | `CircuitError` tipado, sin UI ni estado global |
| coeficientes de filtros y familias como tablas | no se importan: Fase D, con verificación analítica previa |
| cifras de impedancia ajustadas al software antiguo | ninguna: los oráculos son fórmulas de teoría de circuitos |
| posición de símbolos en el canvas | ausente: el netlist no guarda geometría |

## 5. Resultados reales

Comandos ejecutados en el worktree.

| Verificación | Resultado |
|---|---|
| `cargo test --workspace` | 310 tests, 30 suites, 0 fallos |
| `cargo test -p speakerbox-circuits` | 66 tests (32 unitarios, 32 de integración, 2 doc-tests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpio |
| `cargo fmt --all --check` | limpio |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | limpio |
| `cargo check --workspace --all-targets` con Rust 1.75 (contenedor) | limpio, `Finished dev profile` |
| `cargo check --workspace --lib --target wasm32-unknown-unknown` con Rust 1.75 | limpio, `check_exit=0`, 10 `.rmeta` en `target/wasm32-unknown-unknown/debug/deps` |

Errores numéricos observados en las pruebas:

- Fixture RLC: `f0 = 2250.790791 Hz`; `Re(Zin) = 10 Ω` con `|Im(Zin)| < 1e-9 Ω`.
- RLC serie contra forma cerrada en 4 frecuencias: error relativo `< 1e-12`.
- División resistiva: error `< 1e-15` en el cociente complejo.
- Escalas de 1 mΩ a 1 GΩ: error relativo `< 1e-9`.
- Puente equilibrado: `|V2 − V3| < 1e-12 V` y `Zin` dentro de `1e-10` relativo.
- Residual aceptado por el solver: `≤ 1e-8` relativo; las pruebas observan
  `1e-15` en el caso resistivo.

## 6. Licencia y procedencia

- No se ha copiado ni traducido código de Speaker Workshop. La licencia
  histórica («Zope license», sin versión inequívoca) no se ha verificado contra
  el texto aplicable, así que la ruta es reimplementación independiente.
- Origen de cada fórmula: análisis nodal modificado, impedancia de R/L/C y
  divisor de tensión, todas de teoría de circuitos.
- El fixture `series_rlc.CIR` reproduce la topología descrita en la auditoría
  (R = 10 Ω, L = 1 mH, C = 5 µF). Es dato de topología, no código.
- Los oráculos de los tests se calculan con fórmulas dentro del propio test; no
  se generan con el solver que se valida.

## 7. Límites conocidos

- `input_impedance` exige exactamente una fuente; una red con dos fuentes
  devuelve `SourceCount` en vez de un valor ambiguo.
- Las cargas deben ser pasivas (`Re(Z) > 0`). Una carga puramente reactiva se
  rechaza; no es un caso de crossover con driver.
- El serializador `.CIR` no admite fuentes con parte imaginaria ni cargas
  medidas: devuelve `NotSerializable` en vez de perder información.
- Una fuente de tensión con valor cero no define `Zin`: `ZeroReference`.
- El solver es denso y `O(n³)`. Adecuado para el tamaño de un crossover
  (decenas de incógnitas); no está pensado para redes de miles de nodos.
- No hay contrato de presión compleja, polaridad, retardo ni calibración
  acústica. `DesignPoint.phase_deg` no se toca ni se reinterpreta.

## 8. Tareas aplazadas

- **Fase C:** parser FRD/ZMA, modelo `EquivalentImpedance`, ajuste restringido,
  `FitReport`, métodos de masa añadida y caja sellada.
- **Fase D:** topologías HP/LP, L-pad, Zobel y optimizador con Jacobiano
  verificado.
- **Fase E:** audio I/O, FFT/MLS y `speakerbox-measure`.
- **Multivía:** requiere la referencia de presión compleja que la spec deja
  abierta; no se simula un conjunto sin ella.

## 9. Uso mínimo

```rust
use speakerbox_circuits::{parse_cir, Circuit};

let circuit = parse_cir("V1 1 0 1\nR1 1 2 10\nL1 2 3 1m 0\nC1 3 0 5u\n")?;
let z = circuit.input_impedance(2250.790_791)?;
eprintln!("Zin = {z:?}");
```

```bash
cargo test -p speakerbox-circuits
cargo run -p speakerbox-circuits --example ladder_sweep
```

## 10. Verificación MSRV y WASM

Contenedor `rust:1.75` (la máquina anfitriona tiene Rust 1.99.1 y no tiene
`rustup`, así que la MSRV se comprueba en Docker).

```text
$ docker run --rm -v "$PWD":/w -w /w -e CARGO_TARGET_DIR=/tmp/target175 \
    rust:1.75 sh -c "cargo check --workspace --all-targets"
    Checking speakerbox-circuits v0.1.0 (/w/crates/speakerbox-circuits)
    Checking speakerbox-acoustics v0.2.0 (/w)
    Finished dev [unoptimized + debuginfo] target(s) in 1m 31s
```

```text
$ docker run --rm -v "$PWD":/w -w /w -e CARGO_TARGET_DIR=/tmp/targetwasm \
    rust:1.75 sh -c "rustup target add wasm32-unknown-unknown && \
    cargo check --workspace --lib --target wasm32-unknown-unknown"
    Checking speakerbox-circuits v0.1.0 (/w/crates/speakerbox-circuits)
    Checking speakerbox-acoustics v0.2.0 (/w)
    Finished dev [unoptimized + debuginfo] target(s) in 1m 08s
```

Confirmación del artefacto (segunda pasada, mismo contenedor):

```text
check_exit=0
10        # ficheros .rmeta en target/wasm32-unknown-unknown/debug/deps
```

Las dependencias (`num-complex`, `num-traits`) y el crate raíz compilan con
Rust 1.75 sin cambios de código.
