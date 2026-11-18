# Algoritmos de cálculo

Documento de referencia del motor. La implementación es **independiente** y se
basa en ecuaciones públicas de acústica: ecuación de Webster, radiación de un
pistón, parámetros T/S y matrices de transmisión. Se validó punto por punto
contra exports propios de Hornresp 60.70; no se copió código de la aplicación.
Las integraciones añaden fixtures de procedencia externa, identificados en [Validación](VALIDATION.md).

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

Con `U = −S/(jωρ)·dp/dx` se obtiene el sistema de estado:

```text
dp/dx = −j·ω·ρ·U / S(x)
dU/dx = −j·ω·S(x)·p / (ρ·c²)
```

### 2.2 Matriz de transferencia

Para una sección variable se integra el sistema con **RK4** para dos estados base, `[1,0]` y `[0,1]`,
con paso inicial basado en el módulo de la constante de propagación compleja (mínimo 192 pasos por segmento). El
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
b = −j·Z0·sin(kL)
c = −j·sin(kL)/Z0,      Z0 = ρc/S
```

La ruta uniforme usa una matriz cerrada, también con pérdidas. Las pruebas contrastan el caso uniforme con soluciones analíticas.

### 2.3 Leyes de área

| Perfil | `S(x)` | Corte |
|---|---|---|
| Cilíndrico | `S1` | — |
| Cónico | `π·(r1 + (r2−r1)·x/L)²` | — |
| Exponencial | `S1·exp(m·x)`, `m = ln(S2/S1)/L` | `fc = m·c/(4π)` |
| Parabólico | `S1 + (S2−S1)·x/L` | — |
| Hiperbólico-exponencial | `S1·(cosh(x/xT) + T·sinh(x/xT))²` | `fc = c/(2π·xT)` |
| Personalizado | `S(x)` definida por el usuario | — |

`xT` se resuelve analíticamente con el dominio y la elección de raíz
descritos en la sección de auditoría, para que `S(L) = S2`. `T = 0` es catenoidal;
`T = 1/√2` es el valor histórico para registros `Hyp` que omiten el parámetro.
`HornProfileSpec` aporta tractrix, tractrix escalada, OS y exponencial-cónica
con parámetros persistentes. La construcción de Le Cléac’h se describe en
[la auditoría de perfiles](horn-simulation-integration/DECISIONES.md#hypex-tractrix-y-le-cléach).

«Parabólico» (`Par`) es el nombre del software de referencia para un **área
lineal** entre extremos; no es una parábola matemática en el radio ni en el
área. El radio circular resultante es `sqrt(S(x)/π)`, que no es lineal.

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

La carga implementada conserva `Za(2π)` para cualquier `Ω`. Los solvers
aplican un escalado de campo/potencia por separado; no es un modelo exacto
de espacio libre ni de imágenes en esquinas. Ver límites de la auditoría.

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
Z_frente = Z_cámara_garganta ∥ R_fuga ∥ (R_serie + Z_entrada_bocina)
Z_trasera = recinto ∥ puerto ∥ radiador pasivo
```

Para una bocina de una sola salida, la ruta clásica de observación usa el caudal de boca y la impedancia de pistón del área geométrica. A 1 m, antes del ajuste empírico de nivel:

```text
p_rms = |U_boca|·sqrt(Re(Z_pistón)·2πρc/Ω²)
SPL   = 20·log10(p_rms/p_ref)
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

En el BP6S, el puerto trasero añade la corrección de extremo embridada de
Hornresp en su salida interior (`0.8216·a`). El `QL` del registro se aplica
solo a la cámara delantera, escalado por la relación de sintonías
`QL_front = QL·fb_front/fb_rear` cuando la trasera sintoniza más bajo. La prueba de producción compara las filas dentro de su banda declarada.

### 2.11 Radiador directo con recinto

Para una caja sellada, ventilada o con radiador pasivo, `System` usa
`horn = None` y modela el recinto trasero como una compliancia en paralelo con
su puerto y/o radiador pasivo:

```text
Z_front = Z_rad(Sd)
Z_rear  = Z_c ∥ Z_port ∥ Z_pr
U_d     = Sd·Bl·Eg / (Ze·(Zm + Sd²·(Z_front + Z_rear)) + Bl²)
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

La suma anterior usa presión reducida. Multiplicar por j recupera la fase física. Para una caja sellada solo contribuye el diafragma. Para el puerto ventilado se usa la corrección de extremo calibrada
`EndCorrection::Inner(1.5)`, equivalente al extremo interior embridado más el
exterior sin brida. El radiador pasivo usa su propia área, masa, compliancia y
resistencia.

## Evidencia por diseño

Los fixtures, bandas, umbrales y comandos de reproducción se mantienen en [Validación](VALIDATION.md).

## 4. Incógnitas y límites

* La implementación es **independiente**: usa ecuaciones públicas de acústica,
  no una transcripción del código de la aplicación de referencia.
* El modelo exacto de **pérdidas** (`Qa`, `QL`, `Fr`, `Tal`, recubrimientos) y
  de **compresión de potencia** de la aplicación de referencia no es público;
  se usan aproximaciones calibradas contra los exports.
* Bessel, Le Cléac’h y los modelos de onda esférica carecen de una implementación
  dedicada. `Flare::Custom` admite leyes de área definidas por el consumidor.
  El solver Webster conserva su aproximación 1D también para contornos OS.
* La **directividad** es la de un pistón equivalente; Hornresp la califica de
  indicativa. No se implementan mapas polares ni índice de directividad.
* El **tapped horn** de 4 tramos, sin cámaras y con tomas S2/S4 (TH) o S2/S3
  (TH1), está validado a nivel de producción. El **BP4**, el **BP6 paralelo**
  y el **offset driver** OD/OD1/OD2 también están validados a producción con
  exports propios. El **BP6 serie** y el **BP6 paralelo** están validados con
  exports propios. **Multiple entry**, los TH con cámara de garganta/trasera y
  los TH de más de 4 tramos no tienen un ensamblado específico validado.

## Auditoría de consistencia física (2026-11-12)

Convenciones para interpretar matrices, cargas y resultados:

- Fasores RMS con `exp(jωt)`, eje geométrico de entrada a salida y U positiva
  hacia la boca: `dp/dx=-jωρU/S`, `dU/dx=-jωSp/(ρc²)` sin pérdidas.
  En `[p2,U2]=M[p1,U1]`, el cilindro tiene B y C **negativos** imaginarios.
  El puerto concentrado tiene A=D=1, B=-jωρL/S, C=0.
- El cilindro sin pérdidas (incluido cualquier perfil analítico con S1=S2)
  usa ahora la solución exacta. Custom y tramos con pérdidas conservan RK4.
- Cámara de garganta y fuga derivan del nodo del diafragma; la rama de
  bocina contiene `Rserie+Zhorn`. Su caudal es `p_nodo/(Rserie+Zhorn)`.
- `phase_deg` conserva la referencia reducida: fase de presión dividida por
  j. La presión física añade +90° comunes. Offset usa la **suma total**, no
  solo U de boca. No se añade retardo de propagación al observador a 1 m.
- Las sumas de presión de varias salidas son una aproximación compacta,
  a igual distancia y sin fase espacial ni directividad angular. No se
  extrapolan a cajas grandes ni a mapas polares. La potencia estimada por
  estas sumas no es una integral espacial de intensidad.
- La carga de radiación es la de un pistón en pantalla infinita para todos
  los Ω admitidos. Ω cambia la estimación de campo, no esa carga. La
  calibración no la convierte en solución exacta de 4π o de una esquina.
- En Hypex, |T|<1: T>=0 solo permite expansión. Con T<0 la contracción
  requiere S2>=S1(1-T²); se elige la primera raíz (sin cintura oculta).
  S1=S2 se interpreta como perfil constante, xT infinito. Cilindro y tramo
  nulo requieren S1=S2. Custom no se recorta: debe ser positivo y coincidir
  con los extremos; se comprueba al validar y durante cada evaluación RK4.
- Un polo ideal del paralelo se representa por `(∞,0)`; `try_parallel`
  devuelve error cuando no existe resultado finito. No se convierte en cero.
- El retardo se estima con una derivada cuadrática local sobre frecuencias
  no uniformes; el desenrollado se corta ante cancelaciones exactas.

Las métricas ampliadas revelan limitaciones de potencia/rendimiento que las
pruebas históricas de SPL/Ze/Xd no detectan. Ver
[`audits/VALIDATION_REPORT.md`](audits/VALIDATION_REPORT.md) y
[`audits/REMAINING_LIMITATIONS.md`](audits/REMAINING_LIMITATIONS.md).


## Segunda pasada: balance energético y observación

El contrato vigente es [`audits/RADIATION_MODEL_CONTRACT.md`](audits/RADIATION_MODEL_CONTRACT.md).
Las cargas, estados y potencias se vinculan por `P=Re(p·conj(U))=|U|²Re(Z)`
con fasores RMS. `power_w`/`acoustical_power_w` reportan absorción de las
terminaciones explícitas, sin aplicar el escalado de observación. Pueden ser
un subtotal en modelos sin alguna carga exterior. SPL calibrado se conserva
independientemente y no se usa para cerrar un balance.

`EnergyReport` separa fuente, Rg, bornes, bobina, suspensión, pérdidas acústicas
y cargas. Las pérdidas de pared se integran como ∫g|p|²dx; el residual de
balance no se añade a ninguna pérdida. Solo se devuelve potencia radiada con
una calidad explícita; para una boca adaptada o un modelo incompleto es None.
Hay dos eficiencias opcionales, con denominador de fuente o de bornes.

Matched_mouth es un absorbente ρc/Sgeom y no determina presión a 1 m. Se
rechazan factores que cambien esa adaptación. La absorción sí es calculable
mediante try_energy_at; la observación tipada queda no disponible.

AcousticObservation conserva presión estimada, SPL y fase del mismo resultado.
Una cancelación tiene fase/retardo None; los puntos legados usan NaN en fase
de silencio. La función complex_phase_trace permite cortar una derivada
cuando el módulo de presión no supera su cota de error numérico. La
excursión sigue siendo pico, mientras estados y potencias usan RMS.


## Finalización de integración: propagación, bobina y línea distribuida

Para Rf homogénea, el operador de estado es A=[[0,a],[b,0]],
a=−(Rf+jωρ)/S y b=−(loss+j)ωS/(ρc²). En sección uniforme,
M=I cosh(γL)+A·L·sinhc(γL), γ²=ab. En perfil variable se usa |γ| para
resolver tanto fase como atenuación y se comprueba convergencia de RK4.
Se limita la atenuación total aceptada por matrices de transferencia;
el diagnóstico no es un límite físico del material. Ver
[estabilidad numérica](reference-integration/NUMERICAL_STABILITY_REPORT.md).

La bobina puede usar Re+Ke√(jω), Ke en Ω√s, sustituyendo Le. Su parte real
adicional es disipación y se suma al calor de bobina con la frecuencia de
resolución; no se modifica el modelo mecánico ni los T/S importados.

TransmissionLine usa los mismos segmentos y matrices. En el nodo del driver,
Yleft=−Cleft/Aleft y Yright=(Aright−Cright ZL)/(Dright ZL−Bright).
La combinación se calcula mediante productos cruzados para evitar cocientes
singulares intermedios. Se conservan presión y caudales complejos; la potencia
proviene de las cargas y la cuadratura local, no del SPL compacto estimado.
La presión a 1 m comparte la referencia reducida de la fachada (presión física
= j·presión reducida). No se reconstruye geometría espacial de pliegues ni
radiación mutua a partir de longitudes solamente. Ver
[validación TL](reference-integration/TRANSMISSION_LINE_VALIDATION.md).

Alineamientos: [derivaciones y dominios](reference-integration/ALIGNMENT_VALIDATION.md).
Se separa la síntesis T/S ideal del solver completo; desviaciones y coste de
propuestas EBS se exponen en lugar de ocultarse con calibración.

## Convenciones de entrada y observación

| Magnitud | Referencia que debe conservar el consumidor |
| --- | --- |
| Frecuencia y aire | Frecuencia positiva en Hz, ω = 2πf en rad/s. `Air::default()` fija c = 344 m/s, ρ = 1,205 kg/m³ y p_ref = 20 µPa. |
| Geometría y motor | Longitud en m, volumen en m³, superficie en m², Mmd en kg, Cms en m/N. Mms incluye carga de aire. Al importar Mms use la conversión del importador una sola vez. |
| Impedancia | Eléctrica V/I en Ω, mecánica F/v en N·s/m, acústica p/U en Pa·s/m³. ρc es impedancia específica, ρc/S es impedancia característica referida a caudal. |
| Caudal y signo | Fasores RMS con exp(+jωt). U = S·v es positivo en el sentido axial entrada → salida. En una unión el extremo de salida de un tramo aporta −U a la suma saliente. El driver diferencial inyecta caudales opuestos. |
| Excitación | Eg es tensión de generador en circuito abierto, Rg resistencia serie en Ω. En el resumen de `System`, Ze incluye Rg. `Driver::input_impedance` y la impedancia de bornes de la red excluyen Rg. Invertir polaridad gira. la presión 180°. La suma multivía tiene referencia propia de 2,83 V RMS. |
| Campo | SPL = 20 log₁₀(│p_rms│/p_ref). El estimador de presión depende de la topología. Ω en sr fija el ángulo de observación (2π = medio espacio). El pistón usa su área física o el factor de área explícito del modelo calibrado. |
| Fase y desplazamiento | La fachada clásica informa fase de presión reducida. La presión física añade +90°. El fasor de campo de la red ya es físico. El retardo de grupo es −dφ/dω con φ en radianes. `xd_mm` expresa pico en mm y `peak_displacement_m` pico en m. |
| Energía | En una carga pasiva, la potencia activa es la parte real de presión por caudal conjugado con caudal hacia la carga. `EnergyReport` identifica absorción, pérdidas y calidad de radiación. Con denominador nulo, la eficiencia tipada queda ausente. |

El extremo cerrado impone caudal nulo y la liberación ideal de presión impone p = 0. Una carga adaptada absorbe potencia y carece de estimación de campo. Para usar varias fuentes compactas se suma presión compleja con su polaridad. Inferir potencia radiada integrada a partir de una única observación requiere hipótesis adicionales.

Consulte el [ensamblaje de redes, FDD y bobina extendida](wavebox-integration/MODELOS.md) y los [dominios de geometría, adaptador y capa límite](horn-simulation-integration/API.md) para sus condiciones específicas.
