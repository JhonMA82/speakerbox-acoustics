# Contrato propuesto para un backend FEM externo

## Qué aporta y dónde llega

En `horn-simulation` el generador gira el contorno circular para formar un
volumen de aire y verifica la geometría de la pieza. Gmsh etiqueta garganta,
boca y paredes, y produce tetraedros P1 o P2 por banda. El solver DOLFINx
calcula presión compleja de Helmholtz y obtiene el caudal de garganta a
partir de la reacción de contorno. Las condiciones disponibles incluyen
presión uniforme de entrada, velocidad normal impuesta y paredes rígidas.
La opción de capa delgada agrega operadores tangenciales y térmicos sobre
las paredes dentro del dominio de la aproximación de Berggren.

En boca, la condición Robin local de pistón aproxima la radiación punto
a punto. La variante `modal_baffled` acopla el problema interior a una
matriz de Rayleigh de 16 modos radiales de una abertura circular en
pantalla infinita. Puede resolver variación radial que el plano de
Webster no representa. En esta revisión requiere geometría giratoria
verificada, discos centrados sin anillos, P1, pared rígida sin pérdidas,
un proceso MPI y `ka ≤ 30` para la cuadratura modal. Un horn rectangular,
una caja con varias salidas o una guía descentrada necesita otro
tratamiento de apertura, etiquetado y modos azimutales. Un STEP genérico
puede ensayarse con Robin local, dentro de un alcance documentado.

La referencia reporta 0.017 dB de diferencia de impedancia y menos de 0.19°
para la prueba modal contra mode matching en un caso entre 800 y 1,600 Hz.
Con Robin de pistón y frente plano 1D, el cono y el exponencial incumplieron
su tolerancia, con máximos de 0.94 y 1.53 dB para `ka ≤ 1.5`.
Refinar el cono de 43,927 a 116,690 celdas bajo 500 Hz cambió la impedancia
a lo sumo 0.009 dB y 0.065°. Una parte de la diferencia tiene que ver con
la curvatura de frente y la condición de radiación, con causas aún
abiertas entre `ka=0.5` y 1.5. El contraste respalda conservar 1D como
cribado y validar cada backend por su propia frontera física.
[Comparación con Openwind](https://github.com/timini/horn-simulation/blob/7dd9852aec8f60e1b35ae637ad28bd614efc2c1b/docs/CROSS_SOFTWARE_VALIDATION.md) · [Radiación modal](https://github.com/timini/horn-simulation/blob/7dd9852aec8f60e1b35ae637ad28bd614efc2c1b/docs/MODAL_APERTURE.md).

## Frontera de proceso

Un adaptador que viva fuera de la crate recibe una solicitud JSON y produce
un resultado JSON, con STEP, mallas y campos grandes como ficheros opcionales
identificados por hash. El motor Rust conserva el acoplamiento al driver y
puede consumir el resultado como segundo evaluador. Un fallo devuelve código
y detalle, junto con la revisión y el hash de la solicitud.

| Solicitud v1 | Resultado v1 |
| --- | --- |
| ID y hash canónico de la solicitud, versión de formato, identificador inmutable del backend. | Hash de la solicitud, versión del formato, revisión del solver, Gmsh y DOLFINx/PETSc. |
| `HornProfileSpec` o STEP con unidad, orientación y grupo de caras para cada puerto, pared y boca. | Hash de geometría y malla, áreas de CAD y de malla en SI, conteos de nodos y celdas. |
| Cuadrícula explícita en Hz, aire SI, excitación de garganta, `exp(+jωt)` y norma RMS. | Misma cuadrícula y medio. Por frecuencia, pares real/imaginario de presión/caudal de garganta y boca, `Z_a=p/U` y energía. |
| Condiciones de pared y boca, nivel de refinamiento, límites de nodos, tiempo y memoria. | Frontera efectiva, ajuste de discretización, residual algebraico, potencia de entrada, disipación/radiación informadas, advertencias y estado de convergencia. |
| Observadores opcionales, posiciones en metros desde el plano de boca y base de modos solicitada. | Presión de los observadores y coeficientes modales con norma definida, o motivo explícito de ausencia. |

El protocolo acepta solo solicitudes de una entrada acústica en su primera
versión. Una matriz de dispersión de múltiples puertos necesita una versión
posterior y excitaciones independientes. El adaptador debe verificar que la
banda producida coincide con la solicitud y rechazar resultados incompletos,
cambios silenciosos de modelo, campos no finitos y cancelaciones parciales.
El presupuesto se limita por candidato, malla, frecuencia y proceso. Las
tareas repetidas pueden reutilizar resultados con la misma malla y huella de
física. El solver actual genera cuadrículas geométricas por rango en lugar de
aceptar listas arbitrarias. El adaptador inicial debe exigir una cuadrícula
que el backend pueda reproducir exactamente o extender esa entrada.

## Criterios de adopción y coste

1. Verificar cilindro anecoico, cavidad cerrada y cono de referencia. Fijar
   condiciones equivalentes a 1D para el régimen de onda plana y registrar
   impedancia compleja, transferencia y potencia. Separar el error de
   discretización de una diferencia de frontera.
2. Para cada candidato final, resolver en una malla más fina y comprobar
   tolerancias predefinidas de magnitud y fase. Los criterios internos
   recomendados para un piloto son error de área de garganta y boca ≤ 1 %,
   residual ≤ 10⁻⁸, y variación ≤ 0.1 dB y 1° entre mallas en la banda aceptada.
   Para boca modal, repetir con más modos cuando cambie el patrón de radiación.
3. Para adoptar cada familia, contrastar al menos un conjunto de mediciones
   independientes con geometría, aire, fronteras, puerto de excitación y
   trazabilidad de sensor. Publicar CPU, memoria, tiempo por banda y
   proporción de rechazos antes de fijar un presupuesto de producción.

La evidencia de coste existente muestra 100,000 nodos como límite por
defecto y 266,000 nodos solicitados para malla P1 de 4.3 mm en un cono
de 500 mm a 8 kHz. Si la dimensión del elemento se reduce a la mitad,
un volumen tetraédrico cuasi-uniforme puede tener del orden de ocho
veces las celdas. La factorización dispersa y el pico de memoria crecen
según el llenado, por lo que no se extrapola linealmente. Se estima una
a dos semanas de ingeniería para el adaptador de un puerto, validación
de formato y CI aislado si ya existe un entorno FEM. La adopción modal
y las mediciones requieren trabajo adicional. El mantenimiento principal
será fijar las revisiones del entorno, conservar mallas y benchmarks y
volver a verificar cuando cambien el solver o sus bibliotecas.
Las cifras de malla y validación provienen de la referencia fijada. Esta
integración no ejecutó DOLFINx ni informa tiempos FEM propios.
