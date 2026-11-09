# Validación de producción

Resultados de la validación punto por punto contra exports propios de Hornresp
60.70. La tolerancia común es:

- SPL: RMS ≤ 2 dB, máximo ≤ 3,5 dB.
- `Ze`: RMS ≤ 15 %, máximo ≤ 35 %.
- `Xd`: RMS ≤ 15 %, máximo ≤ 40 % (se ignoran puntos con `Xd ≤ 0,02 mm`).

La banda de producción se declara en cada test: 10 Hz–1 kHz para OD, BP y
cajas; TH y bocina normal usan todos los puntos disponibles del export.

`validate` compara SPL, `Ze` y `Xd`. **No compara la fase de impedancia**
(`ze_phase_deg`): el campo se parsea y se conserva, pero no hay una tolerancia
validada para esa magnitud; no se debe declarar una validación completa de la
impedancia. Un oráculo vacío o con puntos inválidos no aprueba
(`n > 0` e `invalid_points == 0`).

| Topología | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| Bocina normal | 1,91–1,94 / 3,15–3,23 dB | 6,0–6,4 / 23,2–26,8 % | 7,1–9,3 / 23,6–35,3 % |
| TH (S2/S4 y S2/S3) | 0,52–0,55 / 1,59–2,69 dB | 6,7–7,1 / 15,9–30,9 % | 3,5–4,5 / 15,8–19,0 % |
| OD / OD1 / OD2 | 0,85 / 1,59–1,77 dB | 4,7–4,8 / 11,4–12,0 % | 4,0–4,3 / 16,4–18,9 % |
| BP4 / BP6P / BP6S | 0,21–0,26 / 0,82–1,50 dB | 0,2–1,4 / 0,7–6,0 % | 0,4–3,7 / 2,0–33,0 % |
| Caja sellada / ventilada / PR | 0,00–0,38 / 0,00–0,97 dB | 0,0–2,5 / 0,0–9,3 % | 0,0–1,0 / 0,0–3,6 % |

Los detalles por diseño están en las secciones siguientes. Las ecuaciones y
la calibración están en [`ALGORITHMS.md`](ALGORITHMS.md). Los datos y oráculos
viven en [`tests/data/README.md`](../tests/data/README.md).

## Validación contra Hornresp 60.70

El motor se ha validado **punto por punto** contra exports reales de la
aplicación (`File > Export > All Chart Data`) para **tres diseños distintos**:

* **Diseño 1 (por defecto):** `S1=80`, `S2=350`, `S3=2400 cm²`;
  `L12=60`, `L23=75 cm`; `Mmd=20 g`; `Vtc=900 cc`, `Vrc=14 L`.
* **Diseño 2:** `S1=100`, `S2=500`, `S3=3000 cm²`; `L12=50`, `L23=80 cm`;
  `Mmd=30 g`; mismo `Vtc`/`Vrc` y misma calibración global.
* **Diseño 3:** `S1=120`, `S2=600`, `S3=3200 cm²`; `L12=55`, `L23=70 cm`;
  `Sd=400 cm²`, `Mmd=25 g`; `Vrc=18 L`, `Vtc=1200 cc`, `Atc=400 cm²`.

Los diseños 2 y 3 se generaron en la UI de Hornresp (botón **Add**, edición de
campos y `Calculate`), no ajustando el modelo al oráculo. Los tres usan **la
misma calibración global**.

Con la calibración [`Calibration::validated_60_70`] (que compensa los modos
axiales de cámara, la terminación de boca y la referencia de nivel que la
aplicación resuelve internamente):

| Métrica | Diseño 1 (533) | Diseño 2 (533) | Diseño 3 (533) |
|---|---:|---:|---:|
| SPL RMS | **1,91 dB** | **1,94 dB** | **1,93 dB** |
| SPL máximo | 3,15 dB | 3,23 dB | 3,17 dB |
| `Ze` RMS | 6,4 % | 6,0 % | 6,2 % |
| `Ze` máximo puntual | **26,8 %** | **23,9 %** | **23,2 %** |
| `Xd` RMS | 9,3 % | 7,1 % | 7,8 % |
| `Xd` máximo puntual | **35,3 %** | **23,6 %** | **25,1 %** |
| Sin calibrar (SPL RMS) | 4,90 dB | 4,92 dB | ~4,9 dB |
| Tolerancias puntuales | ✅ | ✅ | ✅ |

Los máximos puntuales de `Ze`/`Xd` bajaron de **79,4 %/52,1 %** (diseño 1) y
**56,3 %/45,6 %** (diseño 2) a **26,8 %/35,3 %** y **23,9 %/23,6 %** al modelar
el área efectiva de radiación de la boca (factor 2,00) y refinar la terminación.
El diseño 3 confirma que la calibración **generaliza** a una tercera geometría,
otra masa móvil y otras cámaras.

Por bandas (diseño 1, calibrado):

| Banda | SPL RMS | SPL máx | Ze RMS | Xd RMS |
|---|---:|---:|---:|---:|
| 20–70 Hz | 3,11 dB | — | 3,5 % | — |
| 70–300 Hz | 1,16 dB | — | 11,7 % | — |
| 300–1000 Hz | 2,06 dB | — | 9,1 % | — |
| 1–10 kHz | 0,79 dB | — | 0,3 % | — |

### Caja sellada / ventilada / radiador pasivo

Los radiadores directos con recinto se validaron con `System`:

* **Sellada:** `Vrc=50 L`, sin puerto.
* **Ventilada:** `Vrc=100 L`, `Ap=100 cm²`, `Lpt=10 cm`, `QL=7`.
* **Radiador pasivo:** `Vrc=100 L`, `Sp=350 cm²`, `Cmp=0,4 m/N`,
  `Mmp=40 g`, `Rmp=4,0 N·s/m`.

La presión radiada suma las contribuciones complejas del diafragma y de cada
salida trasera (puerto o radiador pasivo), cada una con su área de radiación.
Para el puerto ventilado se usa la corrección de extremo calibrada
`EndCorrection::Inner(1,5)`, que equivale al extremo interior embridado más el
exterior sin brida. El `QL` del recinto se modela como una resistencia en
paralelo con la compliancia, igual que en los bandpass.

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| Sellada-A | **0,00 / 0,00 dB** | **0,0 / 0,0 %** | **0,0 / 0,0 %** |
| Ventilada-A | **0,06 / 0,20 dB** | **0,7 / 3,1 %** | **0,4 / 2,6 %** |
| Radiador pasivo-A | **0,38 / 0,97 dB** | **2,5 / 9,3 %** | **1,0 / 3,6 %** |

La prueba [`tests/system_production.rs`] fija estas tolerancias puntuales en
10 Hz–1 kHz y se ejecuta en cada `cargo test`. Los datos están en
`tests/data/sealed_a_*`, `vented_a_*` y `pr_a_*`.

### Tapped horn (TH1 / TH2)

Los **tapped horns** se validaron contra dos exports reales:

* **TH1:** `S1=400`, `S2=400`, `S3=400`, `S4=1200`, `S5=1500 cm²`;
  `L12=0,40`, `L23=0,20`, `L34=1,20`, `L45=0,30 m`; `Con/Exp/Con/Con`.
* **TH2:** `S1=150`, `S2=400`, `S3=400`, `S4=900`, `S5=1200 cm²`;
  `L12=0,35`, `L23=0,25`, `L34=1,00`, `L45=0,30 m`; `Con/Exp/Con/Con`.

Ambas usan la opción **TH** de Hornresp (tomas S2/S4); `TH1`/`TH2` en el nombre
de los ficheros son etiquetas de los dos diseños, no el flag `TH1` (tomas
S2/S3). El flag **TH1** se validó aparte con la misma geometría que el primer
diseño; el constructor `new_4seg_th1` y el oráculo `th1_s2s3_oracle.txt`
cubren esa variante.

La red es la del `.aks` exportado por Hornresp: S1 cerrado, cuatro tramos en
serie y el diafragma entre **S2 y S4**. El solver nodal resuelve las presiones
en S2…S5; el driver es una fuente de presión `ps = Bl·Eg/(Sd·Ze)` en serie con
`zd = (Zm + Bl²/Ze)/Sd²`. La corriente que entra en la red es
`U = ps/(zd + Z_carga)`; la impedancia eléctrica es
`Ze = Ze_coil + Bl²/(Zm + Sd²·Z_carga)`, con `Z_carga` la impedancia
**diferencial** de la red pasiva entre las dos tomas. Para **TH1 (S2/S3)** las
tomas son S2 y S3; el solver y las fórmulas son las mismas.

La calibración global del TH (`TappedHorn::base`) es
`mouth_radiation_scale = 3.20`, `mouth_radiation_area_factor = 3.00` y
`spl_offset_db = -4.91`. Con ella, los 533 puntos de cada export cumplen:

| Métrica | TH1 (S2/S4) | TH2 (S2/S4) | Límite |
|---|---:|---:|---:|
| SPL RMS / máximo | **0,53 / 1,59 dB** | **0,52 / 1,63 dB** | ≤ 2 / 3,5 dB |
| `Ze` RMS / máximo | **7,0 / 29,9 %** | **7,1 / 30,9 %** | ≤ 15 / 35 % |
| `Xd` RMS / máximo | **4,3 / 18,6 %** | **4,5 / 19,0 %** | ≤ 15 / 40 % |

La variante **TH1 (S2/S3)**, con la geometría del primer diseño, también
cumple:

| Métrica | TH1 (S2/S3) | Límite |
|---|---:|---:|
| SPL RMS / máximo | **0,55 / 2,69 dB** | ≤ 2 / 3,5 dB |
| `Ze` RMS / máximo | **6,7 / 15,9 %** | ≤ 15 / 35 % |
| `Xd` RMS / máximo | **3,5 / 15,8 %** | ≤ 15 / 40 % |

La prueba [`tests/th_production.rs`] fija esas tolerancias puntuales y se
ejecuta en cada `cargo test`. Los ficheros `tests/data/th1_oracle.txt`,
`th2_oracle.txt`, `th1_s2s3_oracle.txt`, `th1_s2s3_record.txt`, `th1_za.txt`,
`th2_za.txt`, `th1_record.txt` y `th2_akabak.aks` conservan los datos de
validación.

### Offset driver (OD / OD1 / OD2)

La bocina de driver desplazado está implementada en `src/offset.rs`:

* El stub `S1→toma` se calcula con `S1` cerrado: `Z_stub = −A_total/C_total`.
* El camino `toma→S5` se termina en la radiación de la boca.
* La carga en la toma es `Z_stub ∥ Z_camino`.
* El lado trasero puede ser una cámara sellada o un **radiador directo**. En
  este último caso se suma su presión de campo lejano a la de la boca:
  `p_total = p_boca + p_trasera`.
* La presión del radiador trasero usa la velocidad del diafragma y el factor
  de directividad de potencia de Hornresp:
  `p_trasera = (ρ·ω·|U_d|/Ω)·r(ka)·√escala`, con
  `r(ka) = √(2·R1)/(ka·(1+R1))`.
* La cámara trasera (o el radiador directo) se suma en serie con la impedancia
  mecánica del driver.

Calibración común de OD/OD1/OD2 en `OffsetHorn::base`:

```text
mouth_radiation_scale      = 4.50
mouth_radiation_area_factor = 4.75
spl_offset_db              = -5.75
rear_radiation_scale       = 4.50   (factor de potencia)
```

Estado: **validado a nivel de producción** contra exports propios de Hornresp
60.70 (`tests/od_production.rs`), con la banda 10 Hz–1 kHz:

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| OD-A | **0,85 / 1,59 dB** | **4,7 / 11,4 %** | **4,0 / 16,4 %** |
| OD1-B | **0,85 / 1,77 dB** | **4,7 / 12,0 %** | **4,1 / 17,2 %** |
| OD2-C | **0,85 / 1,61 dB** | **4,8 / 11,7 %** | **4,3 / 18,9 %** |

Los `.aks` de OD y OD1 confirman que `Vrc` no actúa como cámara sellada en
estos registros: la parte trasera es un radiador directo. El oráculo de OD1
usa solo la curva combinada; sus exports separados de Horn/Direct no son
consistentes entre sí y no se usan. La fachada `Design` expone la topología
como `Design::OffsetHorn`.

### Bandpass (BP4 / BP6)

Implementado en `src/bandpass.rs`:

* **BP4**: cámara trasera sellada + cámara delantera ventilada; la salida es
  el puerto delantero.
* **BP6**: cámara trasera ventilada + cámara delantera ventilada; las dos
  salidas se suman en fase compleja.
* El diafragma ve `Z_trasera + Z_delantera`; cada cámara es una compliancia en
  paralelo con su puerto.
* **Puerto como tubo** por defecto (`PortModel::Pipe`): resonancia de cuarto de
  onda y radiación en la boca. `PortModel::Lumped` queda disponible para el
  modelo de masa simple `jωM`.
* `Ql` opcional por cámara; corrección de extremo del puerto; velocidad de
  cada puerto.
* La salida usa presión lejana `p(1 m) = ρ·jω·U_total/Ω`.
* `Xd` se reporta de **pico** (`√2`·RMS), como Hornresp.
* **BP6 paralelo**: ambos puertos radian al exterior; las dos presiones se
  suman en fase compleja.
* **BP6 serie**: el puerto trasero descarga en la cámara delantera y solo el
  puerto delantero radia. La red se resuelve con la matriz de transmisión del
  puerto trasero y las admitancias de ambas cámaras, de modo que la carga del
  diafragma incluye la realimentación acústica entre cámaras.

Estado: **validado a nivel de producción** contra exports propios de Hornresp
60.70 (`tests/bp_production.rs`):

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| BP4-A | **0,21 / 0,86 dB** | **0,2 / 0,8 %** | **0,4 / 2,0 %** |
| BP6P-A | **0,22 / 0,82 dB** | **0,2 / 0,7 %** | **0,5 / 2,5 %** |
| BP6S-A | **0,26 / 1,50 dB** | **1,4 / 6,0 %** | **3,7 / 33,0 %** |

La banda de producción del SPL es **10 Hz–1 kHz** (banda útil de un bandpass de
subwoofer). Por encima, el modelo de puerto sin pérdidas no reproduce los
valles/picos finos de Hornresp; `Ze` y `Xd` siguen siendo válidos en todo el
rango. El modelo sigue Fincham (1983)/Geddes (1989).

El **BP6 serie** (`Bandpass::new_bp6_series`) queda también **validado a
producción** con su export propio. La diferencia clave frente al paralelo es
que el puerto trasero descarga en la cámara delantera: se resuelve con su
matriz de transmisión y la admitancia de ambas cámaras. El extremo interior del
puerto trasero añade la corrección de salida embridada de Hornresp
(`0.8216·a`), y el `QL` se aplica solo a la cámara delantera, que es la que
radia.

La prueba [`tests/bp_production.rs`] fija el criterio de producción
**SPL RMS ≤ 2 dB / máx ≤ 3,5 dB; `Ze` ≤ 15 % RMS y ≤ 35 % puntual; `Xd` ≤ 15 %
RMS y ≤ 40 % puntual** y se ejecuta en cada `cargo test`. Los tres casos lo
cumplen en 10 Hz–1 kHz.

### Uso en producción

* Use `record.to_system_with(&Calibration::validated_60_70())` para el registro
  por defecto de Hornresp 60.70.
* Para **otro diseño**, genere su propio export y valide con
  `examples/compare_oracle` y la función `validate`. La calibración empírica
  es válida para la geometría validada; no sustituye a una validación del
  diseño concreto.
* Los núcleos numéricos están verificados contra soluciones exactas (Webster
  cilíndrico/exponencial < 1e-5, `J1`/`H1`, asíntotas, sintonía de caja
  ventilada).

### Límites conocidos

El alcance v1, las topologías diferidas y el orden sugerido están en
[`ROADMAP.md`](../ROADMAP.md).

* El **tapped horn** de 4 tramos con tomas S2/S4 (TH) o S2/S3 (TH1) y sin
  cámaras `Vrc/Lrc/Vtc/Atc` está validado a nivel de producción. El **offset
  driver** OD/OD1/OD2 también está validado a producción con tres exports
  propios. Los **bandpass BP4/BP6 paralelo/BP6 serie** y las **cajas
  sellada/ventilada/radiador pasivo** están validados a producción con exports
  propios. Con esto, el **alcance v1 de topologías queda cerrado**. No están
  implementados multiple entry, los TH con cámara de garganta/trasera, ni los
  perfiles tractrix/Bessel/OS cerrados.
* Los **bancos de drivers** (`DriverBank`) cubren N drivers en un mismo
  recinto, cableado paralelo/serie/serie-paralelo, isobaric, push-pull y DVC
  paralelo/serie. La comprobación local contra Hornresp `Nd = 2P/2S` da SPL
  ≈ 0,3 dB RMS y `Ze`/`Xd` ≈ 3–4 % RMS; falta un export propio multi-driver
  para el test de producción permanente. Los **line arrays** con separación
  entre drivers necesitan `RadiatorArray`, todavía pendiente.
* La directividad es la de un pistón equivalente; el propio Hornresp la
  describe como indicativa.
* El modelo de pérdidas del recinto es una aproximación; el ajuste empírico
  absorbe parte de esa diferencia en la geometría validada.

---

## Alcance y límites

* El motor implementa un **modelo físico independiente** (Webster, pistón,
  matrices de transmisión y red acústica). No es una copia de la aplicación de
  referencia; usa fórmulas públicas de acústica.
* La **frecuencia de corte exponencial** (`F23 = 70,27 Hz`) y el volumen
  geométrico de la bocina (~91,8 L; la captura del esquema suma 104,292 L con el
  recinto) se usan como calibración observable contra la aplicación real.
* Las pérdidas internas (`Qa`, `QL`, `Fr`, `Tal`, recubrimientos) son
  parámetros simplificados; el modelo exacto de la aplicación de referencia no
  es público.
* La calibración del tapped horn se validó con dos geometrías reales de 4
  tramos sin cámaras. Para un TH con otra topología (cámaras, más tramos,
  tomas distintas) debe generarse su propio export y validarse con
  `examples/compare_th`.
* Tractrix, Bessel, Le Cléac'h y guía OS se pueden introducir con
  `Flare::Custom`; no se incluyen sus perfiles cerrados.
* La directividad es la de un pistón equivalente y el propio Hornresp la
  describe como «indicativa, no exacta».
* No se distribuye código de la aplicación de referencia; solo se usan
  fórmulas públicas de acústica y datos propios de validación.

Licencia: MIT OR Apache-2.0.
