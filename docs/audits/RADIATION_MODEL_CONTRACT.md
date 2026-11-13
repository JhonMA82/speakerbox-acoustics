# Contrato de radiación y resultados científicos

Base: `5e43883` (primera auditoría). Implementación de segunda pasada:
`11f9d6c`. Contrato independiente de interfaz gráfica y geometría 3D.

## Fasores, unidades y referencias

Se usa exp(jωt), fasores **RMS** y SI. Para una carga, p es su presión y U
entra desde la red hacia esa carga. La potencia activa es:

`P = Re(p·conj(U)) = |U|² Re(Z)`, con p=Z·U.

No hay factor 1/2 porque los fasores son RMS. Excursión de los puntos legados
continúa siendo de pico en mm: Xpico=√2·|Udriver|/(Sd·ω)·1000. No se usa esa
excursión de pico sin dividir por √2 al reconstruir potencia.

La coordenada de los segmentos aumenta hacia la boca:
`dp/dx=-jωρU/S`, `dU/dx=-(g+jωS/(ρc²))p`.
Una salida del recinto trasero recibe p=-Ud·Zrear y Uout=p/Zbranch. Su signo
es opuesto al de la cara frontal. En el stub cerrado, U geométrica positiva
hacia la toma no coincide con el caudal que entra desde la toma al stub.

Las presiones de `EnergyReport.terminations` son **de superficie/nodo**, con
la misma referencia eléctrica y de caudal que el solucionador. No son
presiones a 1 m. La fase reducida histórica `phase_deg` corresponde a p_observada/j;
la presión observada estimada añade +90°. No se incluye un trayecto común
al observador ni su retardo. El nuevo `AcousticObservation` ya usa fase de
presión física y no requiere sumar otros +90°.

## Dos cargas de salida diferentes

### Pistón en pantalla infinita

`Z=(ρc/S_eff)(R1+jX1)·radiation_scale`.
`S_eff=S_geom·mouth_radiation_area_factor` para bocinas.
La terminación usada para energía es exactamente la misma que se usa para
transformar impedancias. El flujo absorbido por esa carga es una potencia
de radiación del **modelo de pistón**. Factores de carga distintos de uno
son empíricos: su interpretación radiativa se etiqueta aproximada.

`solid_angle` **no cambia** esa carga de pistón. El campo legado puede
usar Ω=4π,2π,π,π/2, pero ello no crea una solución exacta de baffles finitos,
espacio libre, imágenes en paredes o esquinas. La potencia activa de la red
no multiplica por 2π/Ω. Ese factor solo subsiste en la presentación de SPL
calibrada, preservada y explícitamente aproximada.

### Absorbente adaptado

`Horn::matched_mouth=true` usa `Z=ρc/S_geom` real. La adaptación plana exige
`radiation_scale == mouth_radiation_area_factor`; otras combinaciones se
rechazan, porque alterarían la impedancia característica y producirían
reflexión. Los factores iguales se cancelan algebraicamente antes de
multiplicar áreas, evitando overflow/underflow espurios.

La potencia absorbida se obtiene con p y U de esa terminación. No se la
convierte en presión de pistón. `System::try_response_at` rechaza un SPL
indefinido de este modo; `try_energy_at` devuelve el balance con calidad
MatchedAbsorber, potencia radiada None y eficiencia radiativa None.
`Design::try_observation_at` devuelve presión/SPL/fase None. No se presenta
el absorbente como un radiador silencioso ni se simula una segunda carga.

## Potencia eléctrica, pérdidas y eficiencia

- Fuente ideal: `Psource=Re(Eg·conj(I))`.
- Resistencia de fuente: `PRg=Rg·|I|²`.
- Bornes del altavoz: `Vdriver=Eg-Rg·I`, `Pdriver=Re(Vdriver·conj(I))`.
- Pérdidas de bobina: `Pcoil=Re·|I|²`.
- Pérdidas de suspensión: `Pmech=Rms·|Ud/Sd|²`.
- Pérdidas acústicas se calculan a partir de cada elemento disipativo.
  Masa, compliancia, inductancia y masa añadida de aire ideales almacenan
  energía reactiva; no se cuentan como disipación activa.

La potencia disipada en el propio altavoz es Pcoil+Pmech en este modelo;
Pdriver incluye además la potencia entregada a la red acústica.

`EnergyReport` conserva estas cantidades separadas. `balance_error_w` y
`acoustic_balance_error_w` son diagnósticos, nunca pérdidas inventadas.

`efficiency_vs_source_pct = 100·Pradiada/Psource`.
`efficiency_vs_driver_terminals_pct = 100·Pradiada/Pdriver`.
El numerador hereda `radiated_power.quality`; si es aproximado la eficiencia
lo es también. Si falta radiación o el denominador es cero, devuelve None.

Los campos legados `power_w`/`acoustical_power_w` se conservan con semántica
corregida: **suma absorbida por las terminaciones explícitas**. No incluyen
pérdidas de PR, ni radiación inexistente en el circuito. Pueden ser un subtotal.
`efficiency_pct` es esa suma dividida por Psource (incluye Rg); no usarlo como
rendimiento radiativo integral sin consultar la calidad científica. En
excitación nula vale NaN (0/0), no un rendimiento cero fabricado.

## Salidas por topología

| Topología | Estado radiativo y potencia de red | Presión observada legada |
|---|---|---|
| System directo / sellado | Cara frontal, U=Ud, carga de pistón. Cámara ideal no disipa ni radia. | Presión de potencia equivalente a 1 m; Ω solo en presentación. |
| System bass reflex | Frente con pistón + puerto de masa pura. QL y revestimiento se disipan. Potencia radiada total None: falta carga exterior del puerto. | Suma compleja compacta del frente y puerto. |
| System PR | Frente con pistón + rama PR con masa/compliancia/Rms. Rms es disipación, no radiación. Potencia total None. | Suma compacta del frente y PR. |
| Bocina convencional | Flujo en carga de boca efectiva, pérdidas de pared/garganta/fuga/revestimiento separadas. | Campo equivalente calibrado de boca; si hay salidas traseras, suma compacta. |
| TH / TH1 | Una carga de boca; pérdidas de pared integradas. Masa de aire añadida ideal no disipa. | Misma presión equivalente y fase de boca reducida; factores empíricos conservados. |
| OD / OD1 / OD2 | Boca y, sin cámara, pistón trasero. Suma de cargas independientes, sin impedancia mutua. Con puerto/PR total radiado None. | Suma compleja de boca y cada salida externa, con fase total; también con toma en garganta. |
| BP4 Pipe | Un puerto exterior terminado en pistón; QL disipa. | Monopolo compacto del caudal del puerto. |
| BP6 paralelo Pipe | Dos puertos exteriores con sus propias cargas; suma energética solo del modelo independiente. | Suma compleja de presiones; puede cancelar sin que se anule la absorción de cargas. |
| BP6 serie Pipe | Solo puerto frontal radia. Puerto trasero interior conserva flujo entre cámaras. | Solo caudal de salida frontal. |
| BP Lumped | Inertancias y salida de presión nula sin resistencia radiativa: flujo activo a terminaciones cero en ese circuito; Pradiada None. | Monopolos estimados; esa estimación no prueba potencia radiada ni eficiencia. |

## Coherencia espacial y límites

No existen posiciones/orientaciones ni distancias individuales en estas
entradas. La suma legada supone fuentes compactas, misma distancia y misma
referencia temporal. Es una aproximación explícita, no una geometría deducida.
No se añade un desplazamiento o retardo ficticio para mejorar Hornresp.

En general `|Σp_i|²` contiene términos de interferencia. No es `Σ|p_i|²`.
La integral de intensidad requiere directividades, distancias y radiación
mutua. Por eso la suma de cargas independientes se etiqueta
IndependentLoadsApproximation (o CalibratedLoadApproximation si hay calibración),
aunque el circuito independiente conserve energía exactamente. La presión
cero en un punto no implica potencia radiada nula.

Cuando hay salidas sin carga de radiación, IncompleteRadiationModel expone
Pradiada=None. Un cero en potencia de una terminación de presión nula tiene
justificación de red; nunca equivale a afirmar que el dispositivo real no
radia. No se añadieron resistencias exteriores ni masas de aire para forzar
un balance o modificar los oráculos.

## Fase, cancelaciones y retardo

El módulo phase estima `-dφ/dω` con parábolas locales en frecuencias no
uniformes; usa secante con dos muestras. El contrato tipado de observación
ordena para derivar, deduplica y restaura el orden del llamador. Una sola
muestra, silencio, cancelación exacta o absorbente sin observación devuelve
retardo None. La fase numérica legada pasa a NaN en silencio.

`complex_phase_trace` recibe presión compleja física y una cota de error
numérico por muestra en Pa. Si |p| no supera esa cota, fase y derivada no
son resolubles. No se impone un umbral universal ni se recorta la presión:
una fuente muy débil sigue teniendo fase si está numéricamente resuelta.
No se pueden recuperar vueltas de fase perdidas por muestreo insuficiente.

## Fuentes y alcance

Las ecuaciones de potencia se derivan de trabajo/fuerza y continuidad de
potencia RMS. Referencia primaria de radiación de pistón, campo lejano y
potencia total: [COMSOL, Baffled Piston Radiation](https://doc.comsol.com/6.3/doc/com.comsol.help.models.aco.baffled_piston_radiation/baffled_piston_radiation.html).
No se copiaron datos ni código externos. La validación de red no certifica
geometría espacial, no linealidad, flujo turbulento ni funcionamiento fuera
del dominio unidimensional de Webster.
