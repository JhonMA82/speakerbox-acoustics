# Algoritmos de cálculo

Documento de referencia del motor. La implementación es **independiente** y se
basa en ecuaciones públicas de acústica: ecuación de Webster, radiación de un
pistón, parámetros T/S y matrices de transmisión. Se validó punto por punto
contra exports propios de Hornresp 60.70; no se copió código de la aplicación.
Los exports propios son los únicos datos externos usados como oráculo.

> Para integrar la librería en una aplicación, use
> [`INTEGRATION.md`](INTEGRATION.md). Para las tablas de producción, use
> [`VALIDATION.md`](VALIDATION.md). Este documento es la referencia de
> ecuaciones y calibración.

---

## 1. Referencia y método

Constantes y modelo físico:

- aire: `c = 344 m/s`, `rho = 1.205 kg/m³`, `p_ref = 20 µPa`;
- diafragma: pistón circular plano rígido;
- bocinas: ecuación de Webster por segmentos y matrices de transmisión 2×2;
- radiación: impedancia de pistón con `J1` y `H1`;
- altavoz: modelo electromecánico y parámetros T/S;
- recintos: compliancias, puertos y radiadores pasivos.

La validación se hace contra `File > Export > All Chart Data` de Hornresp
60.70, con diseños propios, comparando SPL, `Ze` y `Xd` punto por punto. Las
tolerancias y resultados están en [`VALIDATION.md`](VALIDATION.md). El formato
del registro de 734 bytes está en [`RECORD_FORMAT.md`](RECORD_FORMAT.md).

---

## 2. Modelo matemático

### 2.1 Ecuación de Webster

Para una guía de onda de área `S(x)` y número de onda `k = 2πf/c`:

```text
d/dx ( S(x)·dp/dx ) + k²·S(x)·p = 0
```

Con `U = S/(jωρ)·dp/dx` se obtiene el sistema de estado:

```text
dp/dx = j·ω·ρ·U / S(x)
dU/dx = j·ω·S(x)·p / (ρ·c²)
```

### 2.2 Matriz de transferencia

Se integra el sistema con **RK4** para dos estados base, `[1,0]` y `[0,1]`,
con paso `Δx` tal que `k·Δx ≈ 0,03` (mínimo 192 pasos por segmento). El
resultado es

```text
[p2]   [a b] [p1]
[U2] = [c d]·[U1]
```

y la impedancia se propaga hacia atrás con

```text
Z1 = (b − d·Z2) / (c·Z2 − a)
```

Para un tubo cilíndrico la solución exacta es

```text
a = d = cos(kL)
b = +j·Z0·sin(kL)
c = +j·sin(kL)/Z0,      Z0 = ρc/S
```

y las pruebas comparan RK4 contra ella.

### 2.3 Leyes de área

| Perfil | `S(x)` | Corte |
|---|---|---|
| Cilíndrico | `S1` | — |
| Cónico | `(r1 + (r2−r1)·x/L)²` | — |
| Exponencial | `S1·exp(m·x)`, `m = ln(S2/S1)/L` | `fc = m·c/(4π)` |
| Parabólico | `S1 + (S2−S1)·x/L` | — |
| Hiperbólico-exponencial | `S1·(cosh(x/xT) + T·sinh(x/xT))²` | `fc = c/(2π·xT)` |
| Personalizado | `S(x)` definida por el usuario | — |

`xT` se resuelve por bisección para que `S(L) = S2`. `T = 0` es catenoidal;
`T = 1/√2` reproduce el perfil de Le Cléac'h. Tractrix, Bessel y guía OS se
pueden pasar como `Flare::Custom`.

### 2.4 Radiación

Pistón de radio `a = sqrt(S/π)` en pantalla infinita:

```text
Za = (ρc/S)·(R1(x) + j·X1(x)),   x = 2ka
R1 = 1 − 2·J1(x)/x
X1 = 2·H1(x)/x
```

`J1` es la Bessel de primera especie (crate `libm`) y `H1` la Struve de orden 1.
`H1` se calcula con serie de potencias para `x < 25` y desarrollo asintótico
truncado en el término mínimo para `x ≥ 25` (DLMF 11.6.1).

Para un ángulo sólido `Ω` se aplica `Za(Ω) = (2π/Ω)·Za(2π)`, de modo que
`Ω = 4π` (espacio libre) da la mitad, `Ω = 2π` la pantalla infinita, `Ω = π`
el doble y `Ω = π/2` el cuádruple.

### 2.5 Altavoz

Ecuaciones electromecánicas:

```text
Eg = I·(Re + Rg + jωLe) + Bl·v
(Rms + jωMmd + 1/(jωCms))·v = Bl·I − Sd·p_carga
```

La carga acústica se refleja como `Sd²·Zac`. La velocidad volumétrica vale

```text
U = Sd·Bl·Eg / (Ze·(Zm + Sd²·Zac) + Bl²)
```

con `Ze = Re + Rg + jωLe` y `Zm = Rms + jωMmd + 1/(jωCms)`.

Parámetros T/S (con `Mms = Mmd + M_aire`, `M_aire = (8/3)·ρ·a³`):

```text
Fs  = 1/(2π·sqrt(Mms·Cms))
Qms = 2π·Fs·Mms/Rms
Qes = 2π·Fs·Mms·Re/Bl²
Qts = Qms·Qes/(Qms+Qes)
Vas = ρc²·Sd²·Cms
η0  = (4π²/c³)·(Fs³·Vas/Qes)
SPL1W  = 112.2 + 10·log10(η0)
SPL2.8 = SPL1W + 10·log10(8/Re)
```

### 2.6 Recintos, puertos y radiador pasivo

```text
C_recinto = V/(ρc²)
Z_recinto = 1/(jωC)                     (sin pérdidas)
R_recubrimiento = ρc/(α·S)              (en paralelo con C)
M_puerto = ρ·(L + ΔL)/S
ΔL_interior = 0.85·a
ΔL_brida    = 2·0.8216·a
ΔL_sin_brida= 2·0.6133·a
```

Radiador pasivo:

```text
Ma = Mmd/Sd²,   Ca = Cms·Sd²,   Ra = Rms/Sd²
Za = Ra + jωMa + 1/(jωCa)
```

### 2.7 Red y salidas

```text
Z_carga = Z_frente + Z_trasera
Z_frente = cámara de garganta + Z_entrada(bocina)   (o radiación directa)
Z_trasera = recinto ∥ puerto ∥ radiador pasivo
```

La bocina se propaga hasta la boca y la potencia radiada es
`P = |U_boca|²·Re(Z_rad)`. El SPL de potencia a 1 m:

```text
p_rms = sqrt(P·ρc/Ω)
SPL   = 20·log10(p_rms/20µPa)
```

La impedancia eléctrica es `Zin = Ze + Bl²/(Zm + Sd²·Zcarga)`. El retardo de
grupo se obtiene de la fase desenrollada: `GD = −dφ/dω`.

### 2.8 Tapped horn

El tapped horn se representa con la red nodal del `.aks` que exporta Hornresp:

```text
S1 (cerrado) ── tramo 1 ── S2 ── tramo 2 ── S3 ── tramo 3 ── S4 ── tramo 4 ── S5 ── radiación
                              │                                  │
                              └──────────── driver ──────────────┘
```

Para un TH de 4 tramos las tomas son S2/S4; para TH1 son S2/S3. Cada tramo
aporta su matriz de transmisión `[p, U]` convertida a admitancias nodales. La
boca se termina con la impedancia de pistón y el extremo S1 con la admitancia
de un tramo cerrado (`−C/A`).

El driver se modela como una fuente de presión `ps = Bl·Eg/(Sd·Ze)` en serie
con la impedancia mecánica reflejada:

```text
zd = (Zm + Bl²/Ze) / Sd²,     Zm = Rms + jω(Mmd + M_aire) + 1/(jωCms)
U  = ps / (zd + Z_carga)
Ze = Ze_coil + Bl² / (Zm + Sd²·Z_carga)
```

`Z_carga` es la impedancia **diferencial** de la red pasiva entre las dos
tomas, con el driver retirado. Se obtiene inyectando una corriente unidad en
`tap_a` y extrayéndola en `tap_b`.

Detalle crítico de implementación: en la formulación nodal el driver es una
fuente de Norton. El terminal positivo debe estar en `tap_a`; si se invierte,
el término `yd·V` de la corriente se resta en lugar de sumarse y el
desplazamiento `Xd` sale hasta un 50 % alto en las resonancias, aunque el SPL
(solo magnitud) parezca correcto. Con la polaridad correcta,
`z_branch = (p_b − p_a)/U = −Z_carga` y la impedancia eléctrica utiliza
`Zm − Sd²·z_branch = Zm + Sd²·Z_carga`.

Calibración global del TH (`TappedHorn::base`):

```text
mouth_radiation_scale     = 3.20
mouth_radiation_area_factor = 3.00
spl_offset_db             = -4.91
```

### 2.9 Bocina de driver desplazado (OD / OD1 / OD2)

El driver se coloca en una toma intermedia (`S2`, `S3` o `S4`). El tramo
`S1→toma` es un stub cerrado en `S1`; el tramo `toma→S5` continúa hasta la
boca. La impedancia que ve el diafragma es:

```text
Z_stub = impedancia del stub S1→toma, con S1 cerrado
Z_path = impedancia del camino toma→S5, terminada en la radiación
Z_horn = Z_stub ∥ Z_path
Z_rear = cámara trasera (o radiación directa del lado trasero)
U_d    = ps / (Zd + Z_rear + Z_horn)
```

El stub se calcula cascando las matrices de los tramos `S1→toma` y cerrando
`S1` (`U1 = 0`):

```text
Z_stub = −A_total / C_total
```

El camino se calcula con `TransferMatrix::transform_impedance` desde la boca
hacia la toma. La presión en la toma es `p_toma = Z_horn·U_d`; la velocidad que
entra en el camino es `p_toma/Z_path` y se propaga con las matrices hasta `S5`.

Si no hay cámara trasera, el lado posterior del diafragma radia directamente.
La presión de campo lejano se suma a la de la boca:

```text
p_total  = p_boca + p_trasera
p_trasera = (rho·omega·|U_d| / Omega) · r(ka) · sqrt(escala)
r(ka)     = sqrt(2·R1) / (ka·(1 + R1))
```

donde `R1` es la parte real normalizada de la impedancia de radiación del
pistón. `r(ka)` vale 1 a baja frecuencia y tiende a `sqrt(2)/(2·ka)` a alta
frecuencia. Esta es la forma que reproduce la salida *Direct Radiator* de
Hornresp; la formulación de potencia con `2π/Ω` no reproduce la transición a
alta frecuencia.

La impedancia eléctrica es la misma fórmula general:

```text
Ze = Ze_coil + Bl² / (Zm + Sd²·(Z_horn + Z_rear))
```

Calibración común de OD (`OffsetHorn::base`):

```text
mouth_radiation_scale       = 4.50
mouth_radiation_area_factor = 4.75
spl_offset_db               = -5.75
rear_radiation_scale        = 4.50   (factor de potencia)
```

Estado: **validado a nivel de producción** en `src/offset.rs` con
`tests/od_production.rs` contra tres exports propios de Hornresp 60.70
(OD/OD1/OD2, 10 Hz–1 kHz):

| Caso | SPL RMS / máx | `Ze` RMS / máx | `Xd` RMS / máx |
|---|---:|---:|---:|
| OD-A | **0,85 / 1,59 dB** | **4,7 / 11,4 %** | **4,0 / 16,4 %** |
| OD1-B | **0,85 / 1,77 dB** | **4,7 / 12,0 %** | **4,1 / 17,2 %** |
| OD2-C | **0,85 / 1,61 dB** | **4,8 / 11,7 %** | **4,3 / 18,9 %** |

La equivalencia con la bocina normal cuando `tap = 0` se verifica a 1e-9 y el
stub multisegmento tiene una prueba contra el producto de matrices. Los `.aks`
confirman que `Vrc` no actúa como cámara sellada en estos registros; el modelo
usa `rear_chamber_m3 = None`. Los exports separados de Horn/Direct de OD1 no
son consistentes entre sí y no se usan: la validación de OD1 usa solo el
oráculo combinado.

### 2.10 Bandpass 4º y 6º orden

El driver está entre dos cámaras:

```text
BP4:  trasera sellada  ── diafragma ── delantera ventilada ── puerto ── exterior
BP6:  trasera ventilada ─ diafragma ── delantera ventilada ── puertos ─ exterior
```

Compliancias y puertos:

```text
C_r = V_r/(ρc²),   C_f = V_f/(ρc²)
Z_cr = 1/(jωC_r),  Z_cf = 1/(jωC_f)
Z_pr = jωM_r,      Z_pf = jωM_f          (M = ρL_eff/A)
```

Con `PortModel::Pipe` (por defecto), cada puerto es un segmento cilíndrico:

```text
Z_in_puerto = T_puerto · transform_impedance(Z_rad_boca)
U_salida    = U_boca del tubo, propagando (p_cámara, p_cámara/Z_in)
```

`PortModel::Lumped` usa la masa simple `jωM`. El modelo de tubo reproduce la
resonancia de cuarto de onda del puerto, que el modelo lumped no captura.

Cargas:

```text
BP4: Z_rear  = Z_cr
BP6: Z_rear  = Z_cr ∥ Z_pr
     Z_front = Z_cf ∥ Z_pf
     Z_load  = Z_rear + Z_front
```

Con `U_d` la velocidad volumétrica del diafragma:

```text
p_front = Z_front·U_d,   U_pf = p_front/Z_pf
p_rear  = −Z_rear·U_d,   U_pr = p_rear/Z_pr     (solo BP6)
U_total = U_pf + U_pr
p(1 m)  = ρ·jω·U_total/Ω
```

La impedancia eléctrica usa la fórmula general
`Ze = Ze_coil + Bl²/(Zm + Sd²·Z_load)`. La sintonía de cada puerto es
`fb = c/(2π)·sqrt(A/(L_eff·V))`. El `Ql` opcional se modela como una
resistencia en paralelo `R = Ql/(2π·fb·C)`.

`Xd` se reporta de pico (`√2`·RMS).

En el **BP6 serie** el puerto trasero conecta la cámara trasera con la
delantera y solo el puerto delantero radia. Se resuelve la red de dos puertas
con la matriz `M_r = [[A,B],[C,D]]` del puerto trasero:

```text
Y_cr = 1/Z_cr,   Y_f = 1/Z_front
den  = (D·Y_cr − C) + (A − B·Y_cr)·Y_f
p_f/U_d = (A − B·Y_cr − 1)/den
U_pr/U_d = Y_f·(p_f/U_d) − 1
p_r/U_d = D·(p_f/U_d) − B·(U_pr/U_d)
Z_load  = (p_f − p_r)/U_d
U_total = U_pf
```

Para `PortModel::Lumped`, `M_r = [[1, jωM_r], [0, 1]]`; para `Pipe`, es la
matriz del segmento cilíndrico. Esta formulación incluye la realimentación del
puerto trasero en la carga del diafragma, que el modelo de suma simple no
captura.

Estado: **validado a nivel de producción** en `src/bandpass.rs` y
`tests/bp_production.rs` contra exports propios de Hornresp 60.70:

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| BP4-A | **0,21 / 0,86 dB** | **0,2 / 0,8 %** | **0,4 / 2,0 %** |
| BP6P-A | **0,22 / 0,82 dB** | **0,2 / 0,7 %** | **0,5 / 2,5 %** |
| BP6S-A | **0,26 / 1,50 dB** | **1,4 / 6,0 %** | **3,7 / 33,0 %** |

La banda de producción del SPL es 10 Hz–1 kHz. Por encima, el modelo de puerto
sin pérdidas no reproduce los valles/picos finos de Hornresp; `Ze` y `Xd`
siguen siendo válidos en todo el rango.

En el BP6S, el puerto trasero añade la corrección de extremo embridada de
Hornresp en su salida interior (`0.8216·a`). El `QL` del registro se aplica
solo a la cámara delantera, escalado por la relación de sintonías
`QL_front = QL·fb_front/fb_rear` cuando la trasera sintoniza más bajo. Con
ello, el BP6S-A cumple las tolerancias puntuales en los 533 puntos.

### 2.11 Radiador directo con recinto

Para una caja sellada, ventilada o con radiador pasivo, `System` usa
`horn = None` y modela el recinto trasero como una compliancia en paralelo con
su puerto y/o radiador pasivo:

```text
Z_front = Z_rad(Sd)
Z_rear  = Z_c ∥ Z_port ∥ Z_pr
U_d     = ps / (Zd + Sd²·(Z_front + Z_rear))
```

El `QL` del recinto ventilado se modela como una resistencia en paralelo con
la compliancia:

```text
C   = V/(ρc²)
R   = QL/(2π·fb·C)
Z_c = 1/(jωC) ∥ R
fb  = c/(2π)·sqrt(Ap/(L_eff·V))
```

La presión radiada es la suma compleja de las salidas. Cada salida usa su
propia área de radiación:

```text
p_rear  = −U_d·Z_rear
U_port  = p_rear/Z_port
U_pr    = p_rear/Z_pr
p_total = p_d + p_port + p_pr
p_x     = U_x·sqrt(Re(Z_rad(A_x))·2πρc/Ω²)
```

El factor común `jωρ` no se escribe porque cancela en la suma de fases. Para
una caja sellada `p_total = p_d`, y el modelo coincide exactamente con
Hornresp. Para el puerto ventilado se usa la corrección de extremo calibrada
`EndCorrection::Inner(1.5)`, equivalente al extremo interior embridado más el
exterior sin brida. El radiador pasivo usa su propia área, masa, compliancia y
resistencia.

Validación de producción (`tests/system_production.rs`, 10 Hz–1 kHz):

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| Sellada-A | **0,00 / 0,00 dB** | **0,0 / 0,0 %** | **0,0 / 0,0 %** |
| Ventilada-A | **0,06 / 0,20 dB** | **0,7 / 3,1 %** | **0,4 / 2,6 %** |
| Radiador pasivo-A | **0,38 / 0,97 dB** | **2,5 / 9,3 %** | **1,0 / 3,6 %** |

Datos: `tests/data/sealed_a_*`, `vented_a_*` y `pr_a_*`.

---

## 3. Validación

| Prueba | Resultado |
|---|---|
| Matriz de tubo vs. solución exacta de Webster | error < 1e-6 relativo |
| Matriz exponencial vs. solución exacta de Webster | error < 1e-5 relativo (50 Hz–2 kHz) |
| Impedancia adaptada `Z0` preservada | error < 1e-9 |
| `Ze` en resonancia mecánica vs. `Re + Bl²/Rms` | exacto (< 1e-9) |
| `H1(1)`, `H1(2)`, `H1(5)` | 0.19846, 0.64676, 0.80781 |
| Continuidad serie/asintótica en `x=25` | < 1e-6 |
| Asíntotas de radiación a baja frecuencia | < 0.1 % |
| `F23` del registro vs. fórmula exponencial | 70.27 Hz (coincide) |
| Volumen geométrico de la bocina | 91.8 L |
| Mínimo de `|Ze|` de una caja ventilada | coincide con `Fb = 1/(2π√(Ma·C))` (< 10 %) |
| Respuesta del sistema por defecto | finita y positiva en 10 Hz–20 kHz |

El sistema por defecto (registro real) da `Fs = 51.61 Hz`, `Qts = 0.133`,
`Vas = 69.87 L`, `SPL = 100.34 dB (1 W)` y una respuesta que sube hasta
~110 dB entre 160 Hz y 1,25 kHz y cae por encima de 2,5 kHz, coherente con una
bocina de 1,35 m con boca de 2400 cm².

### 3.1 Validación contra Hornresp 60.70

Se exportaron **tres diseños** desde la propia aplicación
(`File > Export > All Chart Data`), con 533 puntos entre 10 Hz y 20 kHz cada
uno. El diseño 2 se creó con la UI (botón **Add**, edición de `S1..L23` y
`Mmd`, `Calculate`), no ajustando el modelo al oráculo:

* Diseño 1: `S1=80`, `S2=350`, `S3=2400 cm²`, `L12=60`, `L23=75 cm`, `Mmd=20 g`.
* Diseño 2: `S1=100`, `S2=500`, `S3=3000 cm²`, `L12=50`, `L23=80 cm`, `Mmd=30 g`.
* Diseño 3: `S1=120`, `S2=600`, `S3=3200 cm²`, `L12=55`, `L23=70 cm`,
  `Sd=400 cm²`, `Mmd=25 g`, `Vrc=18 L`, `Vtc=1200 cc`, `Atc=400 cm²`.

| Métrica | Diseño 1 | Diseño 2 | Diseño 3 |
|---|---:|---:|---:|
| SPL RMS | **1.91 dB** | **1.94 dB** | **1.93 dB** |
| SPL máximo | 3.15 dB | 3.23 dB | 3.17 dB |
| `Ze` RMS / máximo | 6.4 % / **26.8 %** | 6.0 % / **23.9 %** | 6.2 % / **23.2 %** |
| `Xd` RMS / máximo | 9.3 % / **35.3 %** | 7.1 % / **23.6 %** | 7.8 % / **25.1 %** |
| Sin calibrar | 4.90 dB | 4.92 dB | ~4.9 dB |

Puntos dorados del diseño 1 (44 puntos): SPL RMS **1.35 dB** (máx. 2.73),
`Ze` 20.0 % RMS, `Xd` 19.4 % RMS. La prueba
`tests/production_accuracy.rs` fija las tolerancias de producción
(SPL ≤ 2 dB RMS y 3.5 dB máx; `Ze` ≤ 15 % RMS y 35 % puntual; `Xd` ≤ 15 % RMS y
40 % puntual) y se ejecuta en cada
`cargo test`. La calibración empírica compensa los modos axiales de cámara, la
terminación de boca y la referencia de nivel; para otro diseño debe generarse
su propio export y validarse con `validate`/`compare_oracle`.

### 3.2 Correcciones críticas frente a versiones anteriores

Durante la validación se corrigieron errores que invalidaban el modelo:

1. **Signo del sistema de Webster**: el estado `U` debe definirse hacia el
   interior del segmento; con el signo opuesto la reactancia de la bocina
   salía con el signo invertido y la impedancia de entrada no era pasiva.
2. **Cámara de garganta en derivación**: la compliancia de `Vtc` va en
   paralelo con la impedancia de la bocina, no en serie (en serie bloqueaba el
   diafragma).
3. **Corriente de entrada a la bocina**: con cámara en paralelo, la velocidad
   que entra en la bocina es `p_garganta / Z_bocina`, no la velocidad total
   del diafragma.
4. **Potencia radiada**: la impedancia de radiación es la de pantalla infinita
   (2π); la potencia se escala por `2π/Ω` al radiar en un ángulo sólido `Ω`.
5. **Potencia eléctrica de entrada**: `Pin = Re(Eg·I*)`, no `|I|²·(Re+Rg)`;
   esta última subestimaba la potencia y producía eficiencias > 100 %.
6. **Orden y unidades del registro**: el orden real de los campos de cámara es
   `Vrc, Fr, Vtc, Lrc, Tal, Atc` (verificado en la UI): `Vrc=14 L`,
   `Fr=40000 mks rayls/m`, `Vtc=900 cc`, `Lrc=16 cm`, `Tal=4 cm`,
   `Atc=350 cm²`. Las longitudes de bocina van en cm y las áreas en cm².
   `Fta` es el ángulo tangente de boca.
7. **Polaridad de la fuente del driver en el TH**: la fuente Norton debe
   inyectar corriente en `tap_a`. Con el signo opuesto, la red sigue dando un
   SPL de magnitud parecido, pero `Ze` y sobre todo `Xd` se desvían en las
   resonancias. Corregido el signo, `Xd` pasó de 48,8 %/59,9 % RMS a
   4,3 %/4,5 % RMS.

### 3.3 Validación de tapped horn

Se exportaron dos tapped horns completos desde Hornresp 60.70, con 533 puntos
cada uno:

* TH1: `S1=400`, `S2=400`, `S3=400`, `S4=1200`, `S5=1500 cm²`;
  `L12=0,40`, `L23=0,20`, `L34=1,20`, `L45=0,30 m`.
* TH2: `S1=150`, `S2=400`, `S3=400`, `S4=900`, `S5=1200 cm²`;
  `L12=0,35`, `L23=0,25`, `L34=1,00`, `L45=0,30 m`.

Ambas usan la opción TH (tomas S2/S4); los nombres TH1/TH2 identifican los dos
diseños exportados, no el flag TH1 (tomas S2/S3).

La geometría TH2 se contrastó con el esquema de Hornresp: **113,999 L** de
volumen, y las tomas S2/S4 con el `.aks` exportado. La calibración TH es
global (los dos diseños usan la misma):

| Métrica | TH1 (S2/S4) | TH2 (S2/S4) | Límite de producción |
|---|---:|---:|---:|
| SPL RMS / máximo | **0,53 / 1,59 dB** | **0,52 / 1,63 dB** | ≤ 2 / 3,5 dB |
| `Ze` RMS / máximo | **7,0 / 29,9 %** | **7,1 / 30,9 %** | ≤ 15 / 35 % |
| `Xd` RMS / máximo | **4,3 / 18,6 %** | **4,5 / 19,0 %** | ≤ 15 / 40 % |

También se validó la variante **TH1 (S2/S3)** con la geometría del primer
diseño (`TH1 = 1` en el registro, driver entre S2 y S3):

| Métrica | TH1 (S2/S3) | Límite de producción |
|---|---:|---:|
| SPL RMS / máximo | **0,55 / 2,69 dB** | ≤ 2 / 3,5 dB |
| `Ze` RMS / máximo | **6,7 / 15,9 %** | ≤ 15 / 35 % |
| `Xd` RMS / máximo | **3,5 / 15,8 %** | ≤ 15 / 40 % |

La prueba `tests/th_production.rs` fija estas tolerancias puntuales y se
ejecuta en cada `cargo test`. El arnés `examples/compare_th` imprime las
métricas para cualquier par de ficheros oráculo.

---

## 4. Incógnitas y límites

* La implementación es **independiente**: usa ecuaciones públicas de acústica,
  no una transcripción del código de la aplicación de referencia.
* El modelo exacto de **pérdidas** (`Qa`, `QL`, `Fr`, `Tal`, recubrimientos) y
  de **compresión de potencia** de la aplicación de referencia no es público;
  se usan aproximaciones calibradas contra los exports.
* Los perfiles **tractrix, Bessel, Le Cléac'h, OS y spherical wave** no tienen
  forma cerrada en esta versión: se modelan con `Flare::Custom` o con
  hiperbólico-exponencial equivalente.
* La **directividad** es la de un pistón equivalente; Hornresp la califica de
  indicativa. No se implementan mapas polares ni índice de directividad.
* El **tapped horn** de 4 tramos, sin cámaras y con tomas S2/S4 (TH) o S2/S3
  (TH1), está validado a nivel de producción. El **BP4**, el **BP6 paralelo**
  y el **offset driver** OD/OD1/OD2 también están validados a producción con
  exports propios. El **BP6 serie** y el **BP6 paralelo** están validados con
  exports propios. **Multiple entry**, los TH con cámara de garganta/trasera y
  los TH de más de 4 tramos no tienen un ensamblado específico validado.
