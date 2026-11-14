# RFC — reutilización de conocimiento de Speaker Workshop

**Estado:** propuesta de la primera PR (Fase A + Fase B)
**Rama:** `feat/speakerworkshop-integration`
**Fecha:** 2026-11-13
**Referencias:** `AUDITORIA_PORTABILIDAD_SPEAKER_WORKSHOP_RUST.md` y
`INSTRUCCIONES_CLAUDE_CODE.md`, ambas aportadas por el mantenedor.

## 1. Decisión

No se traduce código de Speaker Workshop. Se reimplementan las matemáticas
públicas del análisis de circuitos en un crate nuevo, con oráculos analíticos
independientes.

- El crate raíz `speakerbox-acoustics` conserva su paquete, su API y sus
  oráculos: no se toca ningún solver acústico.
- El repositorio pasa a ser un **Cargo Workspace** con el crate raíz como
  paquete y los subsistemas nuevos como crates hermanos en `crates/`.
- La primera entrega es **solo Fase A + Fase B**: red RLC, solver y parser
  `.CIR`. Curvas/T-S (Fase C), crossover/optimizador (Fase D) y audio (Fase E)
  quedan fuera de esta PR, por decisión de las instrucciones.

## 2. Por qué un crate hermano y no módulos en `src/`

- **Aislamiento de la API acústica.** El crate raíz es pre-1.0 pero validado
  contra oráculos de producción; un módulo eléctrico dentro de `src/` ampliaría
  `lib.rs` y acoplaría dos ciclos de vida distintos.
- **Frontera comprobable.** Un crate hermano no puede importar módulos privados
  del raíz. El compilador garantiza que no hay dependencia circular ni acceso a
  `src/calibration.rs` o `src/system.rs`.
- **Coste cero para el consumidor acústico.** Quien no use filtros no compila
  el crate nuevo ni arrastra su código a wasm.
- **MSRV y WASM se verifican por separado**, con el mismo 1.75 y el mismo
  target.

Dependencias: `speakerbox-circuits` depende solo de `num-complex`. No depende
del crate acústico. El acústico tampoco depende de él en esta fase: no hay
arista entre los dos paquetes, por lo que una dependencia circular es imposible
por construcción.

## 3. Tipos y contratos

### Nodos y componentes

```rust
pub struct NodeId(pub u32);            // 0 = GROUND
pub enum ComponentKind {
    Resistor { ohm: f64 },
    Inductor { henry: f64, dcr_ohm: f64 },
    Capacitor { farad: f64 },
    Load(LoadImpedance),               // Constant(Complex64) | Curve(ComplexCurve)
}
pub struct Component { pub a: NodeId, pub b: NodeId, pub kind: ComponentKind }
pub struct VoltageSource { pub positive: NodeId, pub negative: NodeId, pub volts: Complex64 }
```

Unidades SI estrictas. Los constructores validan: `R > 0`, `L > 0`,
`dcr >= 0`, `C > 0`, carga pasiva con `Re(Z) > 0` **en todos los puntos** de una
curva medida, nodos existentes y `a != b`. Ningún valor se recorta en
silencio.

El parser `.CIR` acepta valores negativos finitos (polaridad de fuente,
reactancia capacitiva) y deja el signo físico a los constructores, que
devuelven el error con su línea. Los ids de nodo están acotados a
`MAX_UNKNOWNS − 1 = 511`; el solver rechaza más de 512 incógnitas antes de
reservar la matriz densa.

### Solver

MNA con fuentes ideales de tensión, eliminación gaussiana con pivoteo parcial y
verificación de residual:

```text
A(ω)·x(ω) = b(ω)
x = [tensiones de nodo ≠ tierra, corrientes de fuente]
```

- Nunca se invierte la matriz. Un pivote nulo devuelve
  `CircuitError::Singular { freq_hz }`.
- El residual relativo `‖Ax−b‖∞ / (‖A‖∞·‖x‖∞+‖b‖∞)` se calcula y se rechaza
  por encima de `1e-8` con `CircuitError::Inaccurate`.
- `pivot_ratio` (pivote mínimo / máximo) se devuelve como aviso de
  condicionamiento, no como número de condición exacto.
- La corriente de fuente se define entrando por el terminal positivo: una
  fuente que entrega potencia da valor negativo, y `Zin = −V/I`.

### Medidas

- `Circuit::solve(f)` → `Solution` (tensiones, corrientes, residual,
  `pivot_ratio`).
- `Circuit::sweep(&[f])`, `input_impedance(f)`, `voltage_transfer(f, out, ref)`,
  `transfer_curve(...)`.
- `Zin` exige exactamente una fuente; `voltage_transfer` exige referencia no
  nula. Ambos devuelven error tipado, no infinitos.

### Curvas y netlist

- `ComplexCurve`: pares frecuencia-valor, interpolación lineal en `log10(f)`,
  **extrapolación prohibida** (`CurveOutOfRange`).
- `parse_cir` / `to_cir`: nodos numéricos conservan su número; los nombrados
  reciben ids por encima del mayor numérico; sufijos SI `f p n u m k meg g`
  con unidad opcional; errores por línea; se ignoran directivas `.`.

## 4. Errores (extracto)

`InvalidFrequency`, `EmptyCircuit`, `NoSource`, `UnknownNode`, `SelfLoop`,
`InvalidComponent`, `InvalidSource`, `UnusableComponent`, `CurveTooShort`,
`CurveNotIncreasing`, `CurveOutOfRange`, `Singular`, `Inaccurate`,
`NonFiniteResult`, `SourceCount`, `ZeroReference`, `NotSerializable`,
`Unsupported`.

La división compleja de `Zin` y de la transferencia es estable por escala: un
cociente finito no se convierte en `NaN` por desbordar `|z|²`. Los mensajes de
error sanean los tokens de entrada (sin caracteres de control, 80 caracteres) y
el parser acumula todos los errores de sintaxis y de construcción, no solo los
de la primera fase.

Sin `panic!`, `unwrap` ni `expect` en rutas de entrada pública.

## 5. Riesgos del legacy que **no** se heredan

| Riesgo auditado | Decisión |
|---|---|
| `matrix_Invert` ignora el fallo de singularidad (OPS-01) | eliminación con `Result`; el llamador no puede ignorar el fallo |
| pivote = primer valor distinto de cero (OPS-02) | pivoteo parcial por magnitud |
| división por impedancia de rama sin protección (OPS-03) | `UnusableComponent` si `|Z| = 0` o no es finito |
| coeficientes de filtros heredados como oráculo | fuera de alcance (Fase D) |
| estado global y UI en el cálculo | el crate no tiene estado global ni UI |
| `AfxMessageBox` y errores `BOOL` | `Result` tipado |

## 6. Licencia y procedencia

- Speaker Workshop se publica como «Zope license» sin versión inequívoca y sin
  archivo de licencia en el árbol auditado. **No se copia ni se traduce código**
  hasta verificar el texto aplicable.
- Este crate es reimplementación independiente: cada fórmula procede de la
  teoría de circuitos (MNA, impedancia de R/L/C, divisor de tensión) y del
  planteamiento del fixture `test.CIR` descrito en la auditoría, que es dato de
  topología y valores, no código.
- Los oráculos de prueba son analíticos y se calculan en el propio test; no se
  generan con el solver.

## 7. Decisiones abiertas (no resueltas aquí)

1. ¿`speakerbox-circuits` se publica como crate propio o queda como miembro
   interno del workspace? **Interno por ahora.**
2. Referencia de presión compleja, polaridad y retardo para combinar drivers:
   pendiente. `DesignPoint.phase_deg` sigue siendo fase de velocidad
   volumétrica y no se usa aquí.
3. Normalización de curvas ZMA/FRD (1 V, 2.83 V, 1 W): fuera de alcance; el
   parser no asumirá calibración.
4. ¿Importar `.SWD`? No: requiere proyecto de serialización MFC.
5. Fase C (ajuste de impedancia y T/S) y Fase D (crossover y optimizador):
   PR separadas.

## 8. Criterios de aceptación de esta PR

- `cargo fmt --all --check` limpio.
- `cargo test --workspace` con todos los tests preexistentes en verde, sin
  tocar tolerancias.
- `cargo clippy --workspace --all-targets -- -D warnings` limpio.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` limpio.
- `cargo check --lib --target wasm32-unknown-unknown` compila.
- `cargo +1.75.0 check --workspace --all-targets` compila.
- Fixture RLC con `f0` analítica y `Zin(f0) ≈ 10 Ω`.
- Suite de entradas inválidas: sin pánicos, errores tipados.

## 9. Extensión Fase C — identificación (PR separada)

Esta fase llega en una rama propia (`feat/speakerworkshop-identification`)
apilada sobre la anterior, porque las instrucciones prohíben mezclar la
identificación de drivers con el solver eléctrico.

### Crate

`crates/speakerbox-measure`, que depende de `speakerbox-circuits` (curvas) y
de `num-complex`. El crate eléctrico no depende de él; el acústico tampoco.

### Contratos

- **FRD/ZMA**: tres columnas (Hz, magnitud, fase en grados). FRD lleva dB y se
  convierte a amplitud compleja; ZMA lleva ohmios. No se asume calibración.
  Errores por línea; sin extrapolación. Los serializadores devuelven `Result` y
  rechazan puntos que el parser no podría releer (magnitud nula o no
  representable).
- **Ajuste**: exige `Re(Z) > 0` en todos los puntos; una curva de respuesta no
  es una impedancia.
- **Circuito equivalente**: `R1∥L1a + Re+jωL1b + R2∥L2∥C2`, siete parámetros
  positivos. `l1b` admite cero.
- **Ajuste**: Levenberg–Marquardt sobre `ln(p)`, residual complejo relativo
  `(Z_modelo−Z_medida)/|Z_medida|`, Jacobiano por diferencias centrales,
  límites físicos por parámetro, `FitReport` con coste inicial/final,
  iteraciones, convergencia, RMSE de módulo y fase circular, y avisos
  (`NoResonance`, `PoorFit`, `ParameterAtBound`).
- **T/S parciales**: `Fs=1/(2π√(L2C2))`, `Qms=R2√(C2/L2)`,
  `Qes=Qms·Re/R2`, `Qts=Qms·Qes/(Qms+Qes)`. `Qes` es convención del circuito,
  no medida independiente.
- **Experimentos**: masa añadida → `Mms` y `Cms`; caja sellada → `Vas`. No se
  deduce `Bl`, `Sd` ni nada que el experimento no determine.
- **Sin acoplamiento**: no se convierte el resultado a `Driver` ni se toca
  `src/driver.rs`.

### Criterios

Los mismos de §8, más: recuperación de parámetros conocidos con datos
sintéticos, aviso ante curvas planas o sin resonancia, y rechazo de
experimentos incoherentes (`Fs_cargada ≥ Fs_libre`, `Fs_caja ≤ Fs_libre`).
