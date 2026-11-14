# Informe — Fase D: crossover y optimización

**Fecha:** 2026-11-13
**Rama:** `feat/speakerworkshop-crossover` (apilada sobre
`feat/speakerworkshop-identification`)
**Worktree:** `.worktrees/feat/speakerworkshop-crossover`
**Base:** `29e5f11`

## 1. Qué se entregó

| Área | Contenido |
|---|---|
| Prototipos | Butterworth y Linkwitz-Riley con respuesta compleja analítica, orden 1–4 |
| Síntesis pasiva | Escalera Butterworth LP/HP de orden 1–4, doblemente terminada |
| Utilidades | L-pad con bypass explícito y red Zobel |
| Optimizador | Levenberg–Marquardt en log-parámetros contra magnitud en dB, con límites |
| Pruebas | 13 nuevas (9 unitarias, 4 de integración) |

## 2. Contratos

- La escalera sintetizada incluye la resistencia de fuente y la carga `Z0`:
  la transferencia normalizada es `2·V(salida)/V(fuente)`.
- Los valores `g_k = 2·sin((2k−1)π/(2n))` se verifican contra
  `|H| = 1/√(1+ω^{2n})` en el propio test, no contra una tabla copiada.
- Linkwitz-Riley se expone como respuesta analítica (dos Butterworth de medio
  orden al cuadrado). **No** se sintetiza pasivamente: requiere secciones
  activas o buffers, y se rechaza con `Unsupported`.
- Bessel, Chebyshev, Legendre, Gaussian y Linear **no** se sintetizan: no hay
  tabla verificada aquí y no se copia la del legacy.
- El optimizador ajusta **una** salida con magnitud objetivo en dB. No hay
  optimización multivía ni objetivos de fase.

## 3. Resultados reales

| Verificación | Resultado |
|---|---|
| `cargo test -p speakerbox-circuits` | 95 pasan, 0 fallan |
| `cargo test --workspace` | 350 pasan, 0 fallan (31 suites) |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpio |
| `cargo fmt --all --check` | limpio |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | limpio |
| MSRV 1.75 y `wasm32-unknown-unknown` | contenedor `rust:1.75` (ver §6) |

Precisión observada:

- La escalera sintetizada coincide con la respuesta analítica en `1e-9`
  absoluto para órdenes 1–4, LP y HP, de `0.1·fc` a `10·fc`.
- En la frecuencia de corte, `|H| = 1/√2` con error `< 1e-12`.
- L-pad: nivel dentro de `1e-12` del valor pedido y `Zin = Z0` dentro de
  `1e-9`, incluido el bypass de 0 dB.
- Zobel: variación de `|Zin|` menor que el 3 % en 24 puntos de 100 Hz a
  20 kHz; el test de integración exige además reducir a menos del 30 % la
  dispersión sin compensar.
- Optimizador: RMSE `< 0.05 dB` recuperando una red detunada un 15 %, con
  coste final menor que el inicial.
- Un objetivo construido con campos públicos incoherentes se rechaza con
  error tipado, no con pánico; las sondas del Jacobiano respetan los límites.
- Relaciones `f/fc` enormes devuelven el límite asintótico (`0` en LP, `1` en
  HP), no `NaN`.
- Ejemplo `crossover_sweep` (2000 Hz, orden 2): red y analítica coinciden a
  seis decimales en todos los puntos, con `−3.010300 dB` y `−90.00°` en el
  corte.

## 4. Diferencias con el legacy

| Legacy | Aquí |
|---|---|
| Tablas de coeficientes por familia sin verificar | solo Butterworth, con `g_k` derivado y comprobado contra la función de transferencia |
| L-pad dividiendo por `(1−r2)` sin proteger 0 dB | bypass explícito (`Option<f64>`) y validación de atenuación |
| Zobel dependiente de curvas y UI | fórmula pura y verificación sobre `Z(f)` |
| Optimizador con `goto`, saturaciones a `1e-11`/`1e3` | LM con límites por variable, tolerancia y avisos |
| Un solo driver como limitación oculta | una salida declarada; multivía fuera de alcance |

## 5. Límites conocidos

- Solo síntesis pasiva Butterworth. Las demás familias quedan como respuesta
  analítica (Linkwitz-Riley) o no están implementadas.
- El optimizador es de magnitud, no de fase ni de impedancia compleja.
- No se modela el altavoz como carga en la optimización más allá de la
  impedancia compleja que ya se puede poner como componente.
- Sin referencia de presión compleja no hay simulación multivía: no se
  promete lo que no se puede calcular.
- Sin audio I/O ni FFT: Fase E.

## 6. MSRV y WASM

Contenedor `rust:1.75`:

```text
$ cargo check --workspace --all-targets
    Checking speakerbox-circuits v0.1.0 (/w/crates/speakerbox-circuits)
    Checking speakerbox-measure v0.1.0 (/w/crates/speakerbox-measure)
    Checking speakerbox-acoustics v0.2.0 (/w)
    Finished dev profile
```

```text
$ rustup target add wasm32-unknown-unknown && \
  cargo check --workspace --lib --target wasm32-unknown-unknown
    Finished dev profile
```

## 7. Uso

```rust
use speakerbox_circuits::{synthesize, FilterFamily, FilterKind, FilterSpec};

let spec = FilterSpec {
    family: FilterFamily::Butterworth,
    kind: FilterKind::LowPass,
    order: 2,
    cutoff_hz: 2000.0,
    impedance_ohm: 8.0,
};
let ladder = synthesize(&spec)?;
let response = ladder.normalized_transfer(2000.0)?;
assert!((response.norm() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
```
