# Contrato de presión compleja para multivía

**Estado:** normativa. Tensión de referencia **2.83 V RMS aprobada** e implementada en `src/multiway.rs`.
**Fecha:** 2026-11-14
**Alcance:** define qué puede sumarse, con qué referencia y qué sigue siendo
una aproximación declarada. No añade código, ni cambia la API existente.

## 1. Por qué hace falta

El motor ya expone una presión compleja por diseño individual:

```text
p_i(f) = p_ref · 10^(SPL_i(f)/20) · e^(j·(φ_i(f)+90°))
```

`Design::try_observation_at` la devuelve en `pressure_estimate_pa`, con
`p_ref = 20 µPa`, `φ_i` la fase reducida del `DesignPoint` y sin trayecto común
al observador. Lo que **no** está definido es cómo se combinan dos o más de
esas presiones: nivel de excitación, distancia, polaridad, retardo y
condiciones de campo. Sumar sin fijar eso produce un número con apariencia
física y sin significado.

Este documento fija esa suma. Hasta que el mantenedor lo apruebe y exista una
API que lo implemente, **multivía no está soportado**: la suma de salidas
individuales sigue siendo una aproximación de fuentes compactas.

## 2. Lo que ya es normativo (verificado en código)

| Concepto | Valor | Fuente |
|---|---|---|
| Presión de referencia | `20 µPa` | `Air::p_ref` |
| Nivel | `SPL = 20·log10(p_rms/p_ref) + spl_offset_db` | `src/system.rs` |
| Fase de `DesignPoint` | fase de presión reducida por `j` | `src/design.rs` |
| Presión física | `phase_deg + 90°` | `try_observation_at` |
| Distancia | 1 m implícito en la definición de SPL | `DesignPoint::spl_db` |
| Ángulo sólido | estereorradianes, configurable por diseño | `System::solid_angle` |
| Polaridad | `eg` negativo invierte el diafragma | `System::eg` |
| Retardo de trayecto | **no modelado**; la fase es relativa a `Eg` | `REMAINING_LIMITATIONS.md` |
| Silencio | `SPL = -∞`, presión `0`, fase no definida | `AcousticObservation` |

## 3. La suma que se propone

Para `N` salidas de un mismo sistema de altavoces, en un punto de observación
común:

```text
p_total(f) = Σ_i  s_i · g_i · p_i(f) · e^(−j·2π·f·τ_i)
```

- `p_i(f)`: presión compleja del diseño `i` con **la misma** `eg`, `rg`,
  ángulo sólido y calibración.
- `s_i ∈ {−1, +1}`: polaridad declarada por el usuario. No se deduce del
  diagrama.
- `g_i ≥ 0`: ganancia de nivel por vía (atenuación pasiva, ajuste de DSP).
  No es una calibración: se declara y se documenta.
- `τ_i`: retardo en segundos, positivo = la salida llega **más tarde**.
  Incluye desplazamiento del centro acústico y retardo de DSP. El trayecto de
  1 m **no** se incluye: es común a todas las vías y no cambia la suma.

El resultado es una presión compleja a 1 m, en el mismo ángulo sólido, con la
misma referencia eléctrica. El SPL se obtiene como
`20·log10(|p_total|/p_ref)` sin `spl_offset_db` adicional.

## 4. Decisiones pendientes de aprobación

| Decisión | Opciones | Recomendación |
|---|---|---|
| Tensión de referencia | 2.83 V RMS (1 W en 8 Ω), 1 V RMS, normalizada a `eg = 1` | **Aprobado: 2.83 V RMS**; `sum_at` normaliza cada vía por `2.83/|eg|` |
| Distancia | 1 m, otra | **Adoptado: 1 m**, la única que el modelo define |
| Eje | on-axis, off-axis | **Adoptado: on-axis**; off-axis requiere directividad, que no se modela |
| Retardo del trayecto | incluido, excluido | **Adoptado: excluido**; es común a las vías |
| Interpolación | log-frecuencia compleja, lineal, ninguna | **Diferida al llamador**: la API es por frecuencia y no extrapola |
| Datos ausentes | suma parcial, `None` | **Adoptado: `None`**; nunca una suma parcial |

La referencia está aprobada y las demás filas quedan adoptadas por la
implementación. `sum_at` y `sum_sweep` son la API; las pruebas de §6 son
`src/multiway.rs` y `tests/multiway.rs`.

## 5. Cuándo la suma es válida

La aproximación presupone, y debe repetirse en la API:

- fuentes compactas frente a la longitud de onda;
- misma distancia al punto de observación;
- misma referencia temporal y eléctrica;
- radiación en el mismo ángulo sólido;
- ausencia de reflexiones y de acoplamiento mutuo entre fuentes;
- una sola posición de observación, sin directividad polar.

No cubre: caras radiando en direcciones distintas, dipolos, arrays espaciales,
off-axis, sala, ni suma de potencias (esto es presión coherente, no energía).

## 6. Criterios de aceptación para implementarlo

1. Dos diseños idénticos, misma fase y polaridad: `+6.0206 dB` de presión
   respecto a uno solo.
2. Dos diseños idénticos en oposición: cancelación; `|p| ≈ 0` y fase no
   definida (`None`), no `NaN` suelto.
3. Retardo: `τ_2 = τ_1 + 1/(2f)` invierte la suma a esa frecuencia.
4. Polaridad: `s_2 = −1` equivale a `τ` de medio periodo a la frecuencia de
   prueba.
5. Referencia: cambiar `eg` en todas las vías escala la suma de forma
   lineal; cambiar solo una no es una operación válida.
6. Datos ausentes en una vía: total `None`, nunca una suma parcial.
7. La suma no altera `DesignPoint`, `validate` ni los oráculos existentes.

## 7. Lo que no se afirma

- No hay calibración absoluta: `spl_offset_db` es empírico y no se extrapola
  a un sistema multivía.
- No se valida contra mediciones multivía: no existen oráculos propios.
- No se modela el trayecto acústico ni la fase del aire a 1 m.
- No se promete directividad, off-axis, sala ni arrays.

## 8. Validación con mediciones reales (pendiente)

El repositorio **no contiene ninguna medición multivía**. Los datos de
`tests/data/` son exports de Hornresp de un solo diseño y
`docs/audits/REMAINING_LIMITATIONS.md` ya declara que no hay oráculos
permanentes multi-driver. Sin ese dataset, la suma está verificada contra el
contrato y sus casos analíticos, no contra una medición del sistema completo.

Esta sección fija qué hace falta para cerrar esa validación sin inventar
resultados.

### Dataset mínimo

Un directorio por configuración, con:

| Fichero | Contenido |
|---|---|
| `woofer.frd` | SPL (dB re 20 µPa) y fase (grados) del grave solo |
| `tweeter.frd` | SPL y fase del agudo solo |
| `system.frd` | SPL y fase del sistema completo, con el crossover/DSP real |
| `meta.txt` | `distance_m`, `voltage_rms`, `polarity`, `gain`, `delay_s`, `time_reference`, `note` |

Requisitos de medida:

- mismos micrófono, distancia, eje y nivel de excitación (2.83 V RMS en cada
  terminal) para las tres capturas;
- misma referencia temporal y misma cadena de conversión;
- configuración quasi-anecoica con ventana temporal, sin recortes;
- fase exportada en grados, sin desenrollar a mano;
- el sistema completo medido con la polaridad, ganancia y retardo declarados
  en `meta.txt`.

### Protocolo

1. `speakerbox_measure::parse_frd` convierte cada FRD a presión compleja.
2. `sum_at` predice el sistema con los mismos `polarity`, `gain` y `delay_s`.
3. Comparar predicción y `system.frd` en la malla medida:
   - magnitud: RMS y máximo en dB;
   - fase: diferencia circular en grados, solo donde la magnitud medida esté
     a menos de 12 dB de su máximo.
4. Criterio de pase: RMS ≤ 1.5 dB y máximo ≤ 3 dB en magnitud; RMS ≤ 15° y
   máximo ≤ 30° en fase, excluyendo puntos a menos de 12 dB de un nulo.
5. Si falla, se documenta la discrepancia. **No se ajusta el modelo para
   hacerla pasar** ni se cambia la calibración sin revalidar los oráculos
   existentes.

### Estado

- Suma analítica: implementada y cubierta por `src/multiway.rs` y
  `tests/multiway.rs`.
- Comprobación **aritmética** con dos oráculos reales de un solo driver
  (OD1/OD2, export de Hornresp): `tests/multiway_oracles.rs`. Verifica la
  identidad exacta de composición con ganancia, polaridad y retardo, y que la
  suma no se desvía más que la suma de los errores individuales del modelo
  frente a su oráculo. **No es una validación multivía**: los diseños no se
  midieron juntos.
- Medición multivía: **pendiente de dataset**. Sin él no se puede afirmar
  «validado».
