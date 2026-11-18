# Modelos y decisiones

Base remota `dbcd8eeba183b1a8af7722e706c2bfa234206efc`, más el parche
local completado de `horn-simulation`. La base local verificada
`060b05613ebdadfb32bb7f2b3911e55d3f97ee09` fija ese estado.
Referencia Wavebox `4195c93f757a99c4bb2af456ff8dd8421a948510`.

| Etapa | Decisión y componente reutilizado |
| --- | --- |
| W1 | `AcousticNetwork` usa `Segment` y sus matrices Webster, `Chamber` concentrada, `Port` inercial, radiación de pistón y `EnergyReport`. Los solvers de `System`, `TappedHorn`, `OffsetHorn`, `Bandpass` y `TransmissionLine` conservan sus rutas y calibraciones. |
| W2 | PH1 describe dos caminos con sus tramos terminales, dos tomas del mismo diafragma y una unión. Los nodos extremos H1 y H5 quedan cerrados por continuidad de flujo. |
| W3 | DBR, BP8S y BPB tienen constructores distribuidos. Las pérdidas opcionales son resistencias acústicas SI especificadas por el consumidor. BPC y ABC se conservan como pruebas de diagnóstico. BP8P se expresa como red y queda sin oráculo. |
| W4 | Se añade el circuito eléctrico extendido y una suspensión FDD explícita para la red. Darcy, pared, Delany–Bazley y las zonas de `TransmissionLine` se conservan. El ajuste poroso de Wavebox queda documentado. |
| W5 | Se escala la respuesta acústica existente a partir de 1 V RMS. El EQ analógico vive en `speakerbox-circuits`, donde ya existen los prototipos de crossover y MNA para circuitos cargados. |
| W6 | El lector textual conserva el registro original y actualiza un subconjunto de campos por plantilla. El binario de 734 bytes, FRD/ZMA y los comparadores anteriores conservan su infraestructura. |

## Ensamblaje nodal

Las incógnitas son la presión de cada nodo y un caudal de entrada por
conducto. Para estados RMS `exp(+jωt)`, las filas de un tramo de `a` a `b`
imponen `p_b=A p_a+B U_a` y `U_b=C p_a+D U_a`. La primera ecuación
entra como restricción. En las filas de continuidad, el caudal que sale del
nodo `a` es `U_a` y el que sale del nodo `b` hacia ese tramo es `−U_b`.
En fronteras normales se suma `p/Z` a la continuidad. La frontera
`PressureRelease` añade una incógnita de caudal y una ecuación `p=0`. Un
extremo sin otras ramas cierra su caudal por continuidad. Esta forma mixta
equivale a eliminar los caudales y formar `Yp=q` cuando esa eliminación
es posible. Conserva la formulación en media onda y otras frecuencias donde
`B=0`.

El solver normaliza el caudal con una impedancia característica y equilibra
filas y columnas. Usa eliminación compleja con pivoteo parcial, comprueba
razón de pivotes escalados ≥ 10⁻¹² y residual relativo por ecuación ≤ 10⁻⁸
respecto a la norma de la fila, incógnitas y fuente. La razón es un diagnóstico,
con sensibilidad al escalado, no un número de condición exacto. Cada
componente debe tener una fuente definida y una referencia física en una
frontera o en un tramo compresible. Un tubo cerrado puede tener un polo
ideal de la red acústica desnuda. Al consultar la impedancia de esa red,
un polo sin solución numérica única devuelve un error en vez de añadir
admitancia ficticia. Límite denso de 256 incógnitas, coste cúbico.

El acoplamiento aplica caudales opuestos a las dos caras en una sola
resolución acústica. Refleja `Sd²·(p_front−p_rear)/U_d` al motor. Se
obtienen las presiones y caudales efectivos escalando el estado de la red
por el caudal complejo del diafragma. La energía computa el flujo activo
`Re(p·conj(U))`, las pérdidas distribuidas integradas, pérdidas de bobina,
suspensión y fugas. La estimación de campo usa el pistón equivalente de
la librería a 1 m, a partir de la velocidad de cada abertura. El observador
supone fuentes compactas y una fase geométrica común y carece de impedancias
mutuas. Cuando hay varias bocas, la potencia de cargas sigue siendo
distinta del cuadrado de la presión coherente. La impedancia eléctrica que
informa la red se mide en bornes y excluye Rg.

## Motor extendido y FDD

`CoilModel::ExtendedSemiInductance` usa
`Re′ + jωLeb + (jωLe ∥ Rss ∥ Ke√(jω))`. Re′ y Rss están en Ω, Leb y
Le en H, Ke en Ω·√s. La rama de raíz principal y los elementos positivos
dan una impedancia bloqueada pasiva. El límite de continua es Re′.
En alta frecuencia la parte real tiende a Re′ + Rss. La potencia DC
y el exceso de pérdidas se calculan a partir de la impedancia efectiva
y la corriente RMS. La escala de bancos de drivers aplica el mismo factor
a los cinco coeficientes de la impedancia. La variante es una adición pública
al enum. Quien haga un `match` exhaustivo sobre `CoilModel` debe añadir el caso.

`NetworkDriver::mechanical_damping` activa FDD con resistencia mecánica
explícita `Rms_f` en kg/s y pérdida de compliancia `Ams` en m/N. A
frecuencia positiva, `Cms* = Cms − j Ams`. La parte real de
`1/(jω Cms*)` vale `Ams / [ω(Cms² + Ams²)]` y es no negativa.
Ams y Rms_f deben ser no negativos. Esta forma representa un modelo
fenomenológico de pérdidas en frecuencia positiva. El término creciente
hacia continua y los parámetros ajustados de Hornresp requieren mantener
el alcance de banda del fixture. Se conserva el motor predeterminado cuando
no se selecciona FDD. Los dos modelos opcionales tienen una comparación
separada con curvas de Hornresp.

## Relleno poroso

`porous` y `filled_segment` en Wavebox modelan un medio equivalente de
poros cilíndricos con funciones de Bessel complejas. Devuelven `k` y `Zc`,
de los que se infieren `ρeff = k·Zc/ω` y `Ceff = k/(ω·Zc)`. El modelo
permite relleno desde un extremo con el signo de la fracción y luego
cuantiza su posición a la mitad de las celdas. La infraestructura existente
de zonas y subdivisión de TL cubre esa geometría de relleno.

Se evaluaron los tres archivos `absorbent_*.npz` de Wavebox, con 372
frecuencias entre 10 y 2,000 Hz por resistividad. Son valores `k,Zc`
extraídos de impedancias Hornresp, con dispersión y calibración en el mismo
entorno. La versión porosa da RMS complejo de **9.18–12.16 %** en `k` y
**9.95–10.17 %** en `Zc`. El modelo puro de resistividad da **18.45–20.06 %**
y **13.26–19.38 %**. Delany–Bazley da **9.24–12.52 %** y **10.34–17.53 %**.
La diferencia con Darcy es una combinación de inercia efectiva y transferencia
térmica, y la mejora en `Zc` frente a DB depende de la resistividad. En el
ensayo de 0.01–2,000 Hz resultaron positivas las partes reales de
`jωρeff` y `jωCeff`. La resistencia en baja frecuencia tiende a
`0.9737·σ`, donde 0.9737 es un coeficiente ajustado. La tortuosidad
1.0305 y la relación térmica 1.2058 también provienen de ese ajuste.

Se conserva como candidato para una calibración específica. Para adoptarlo
como modelo de relleno se necesitan caracterización del material, control
de ocupación y datos independientes de propagación o mediciones de tubos
con y sin relleno. [Código poroso de Wavebox](https://github.com/Padwaywaydonc/wavebox/blob/4195c93f757a99c4bb2af456ff8dd8421a948510/wavebox/acoustics.py).
