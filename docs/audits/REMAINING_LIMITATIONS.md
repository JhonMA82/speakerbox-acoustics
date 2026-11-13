# Limitaciones y riesgos residuales

Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b`.
Último commit de implementación: `b462560`.

## Bloqueo físico de prioridad alta: potencia y radiación general

La validación ampliada **no permite declarar el motor físicamente cerrado**.
Los valores legados de potencia y rendimiento no están validados con la misma
exactitud que SPL/Ze/Xd. Se preservaron y se etiquetaron como estimaciones,
sin ajustar calibración para ocultar la desviación. Los errores RMS de potencia
frente a Hornresp alcanzan ~323 % en OD y ~215–217 % en TH; ver tablas.

Causas identificadas (no se atribuye toda la desviación a una única causa):

- La carga de pistón es 2π para cualquier Ω. En cambio, el campo de bocinas
  escala potencia por 2π/Ω y después presión por 1/sqrt(Ω). No son una
  solución común de condiciones de frontera para 4π, π y π/2.
- `spl_offset_db` corrige SPL y no corrige la potencia reportada. OD además
  escala la presión directa trasera sin escalar su carga mecánica.
- System directo y BP convierten la presión combinada compacta en potencia
  equivalente; OD suma potencias independientes. Ninguna regla es una
  integral general de intensidad para fuentes de posición arbitraria.
- Los puertos concentrados no tienen resistencia de radiación explícita;
  los PR no añaden aquí su radiación externa a la carga de membrana. Sus
  contribuciones de presión no garantizan balance radiado exacto. Rms del
  PR representa disipación mecánica y no debe confundirse con radiación.

**Bloqueo técnico:** la API carece de posiciones, orientaciones, pantalla,
impedancias mutuas y un criterio de observación que identifique una única
solución espacial. Cambiar unilateralmente la carga para reproducir Ω o la
calibración produciría un modelo físico distinto sin ese contrato. Hace falta
definir explícitamente un modelo de radiación (compacto con acoplamiento,
pistón/pantalla o imágenes), conservar el modelo legado como opción y generar
oráculos de energía correspondientes. No se ha inventado esa geometría.

Las soluciones complejas nuevas de OD/BP exponen potencia absorbida por la
red y potencia eléctrica de entrada por separado. Su balance con pérdidas de
bobina/fuente y suspensión está comprobado. Eso no certifica que la estimación
legada de campo sea potencia acústica integral. La prueba AC-001 demuestra
conservación en el caso identificable: una boca, 2π, sin calibración ni pérdidas
de pared, incluyendo Rserie.

## Fase y observación

`phase_deg` es la fase de presión reducida por j, no la presión física sin
conversión. Añadir +90° para comparar con UPhase de los exports. Se mantiene
la referencia histórica para compatibilidad. No se modela el trayecto común
al observador (ni su retardo ~1/c a 1 m). OD ya usa la suma total, pero aún
presenta ~20° RMS contra las referencias; no se declara fase acústica validada.
TH conserva desviaciones de retardo hasta ~9,5 ms RMS en todo el export,
que incluye resonancias altas. En bocina normal algunos errores de retardo
suben ligeramente con la derivada centrada: no se ha forzado a la fórmula
histórica para mejorar el indicador.

La suma coherente presupone fuentes compactas, igual distancia y misma
referencia temporal. No se puede extrapolar a salidas en caras alejadas,
dipolos, directividad polar ni arreglos espaciales. Una cámara sellada no
aporta salida trasera. El revestimiento tampoco radia por sí mismo.

Las cancelaciones exactas dan fase/retardo no definidos. La nueva API usa
`Option`; las rutas legadas de barrido usan NaN explícito. Cerca de una
cancelación no nula, la derivada puede estar mal condicionada; no se aplica
un suelo artificial. El usuario debe refinar el barrido. El desenrollado no
puede recuperar vueltas si el salto entre muestras excede 180°.

## Geometría y métodos numéricos

Hypex negativo selecciona la primera raíz de contracción. No se ofrece una
selección de segunda raíz con cintura. Para S1=S2 se elige el límite constante.
Custom se muestrea al preparar y en cada etapa RK4; ninguna cantidad finita
de muestras prueba positividad o continuidad de una closure arbitraria.
Las funciones deben ser puras: un préstamo Rust no impide mutabilidad interior
mediante atomics/mutex capturados. No hay caché que presuponga su identidad.

Se permiten saltos de área entre tramos como **unión ideal** de longitud cero,
con presión y caudal continuos; no son geometrías suavizadas. No se calculan
modos transversales ni pérdidas de contracción brusca. Para un conducto
continuo, el generador paramétrico debe imponer S2[i]=S1[i+1].

RK4 conserva el límite histórico de 60.000 pasos y no tiene estimador de error
adaptativo. No garantiza precisión para longitudes/frecuencias arbitrariamente
altas ni flares extremos. Solvers nodales conservan criterios de pivoteo
absolutos; una representación en admitancias puede singularizarse exactamente
en resonancias de tramo. En lugar de fabricar ceros, las rutas estrictas
rechazan puntos no resolubles. Falta una formulación escalada/MNA para cubrir
estos casos exactos y una caracterización de condicionamiento global.

Algunos helpers legados de bajo nivel no devuelven Result (matrix, loads,
impedancias); trabajar con entradas validadas y con `try_response_at`.
`response_at` conserva el punto neutro contractual ante error; en integración
científica debe usarse la variante estricta.

## Parámetros y bancos

DriverBank conserva el equivalente de pistón de área total: no separa la masa
propia de aire de cada cono de la impedancia mutua. Su masa de aire equivalente
escala como N^(3/2), no como N; no representa conos separados en campo libre.
DVC presupone el equivalente eléctrico definido en la API; no conoce la
inductancia mutua entre bobinas. Isobárico presupone acoplamiento rígido, sin
volumen ni pérdidas en la cámara entre conos. Push-pull es solo lineal.
No hay oráculos permanentes multi-driver.

Parámetros preexistentes con alcance restringido: rear_ql de System solo se
aplica cuando hay puerto con sintonía; BP4 ignora el routing trasero. No se
usaron estas combinaciones en la validación. El modelo Delany-Bazley conserva
su dominio empírico y su tratamiento de frecuencia baja; no se certifica
fuera de él. El importador no detecta toda inconsistencia entre parámetros
T/S redundantes y no deduce una conexión DVC a partir de metadatos.

## Cobertura y rendimiento

No existen datos de fase/potencia/velocidad de puerto para BP en los exports
separados. Las velocidades de puerto se verifican por reconstrucción de
fasores y área, no contra Hornresp. No se han inventado columnas ni curvas.
No se añadieron tolerancias de producción para magnitudes nuevas.

MSRV y WASM se comprobaron por compilación, no ejecución en navegador.
Windows/macOS y otros procesadores no se probaron. Los tiempos proceden del
mismo entorno Linux x86_64, con carga compartida y sin afinidad ni aislamiento.
No hubo perfil de hardware con contadores; la atribución del ahorro se basa
en eliminar integraciones identificables, equivalencia y medidas repetidas.
No se incorporó paralelismo ni caché persistente.
