# M3 — línea distribuida

API `TransmissionLine`: segmentos Webster existentes, nodo de driver configurable,
extremo inicial cerrado y salida de pistón, cerrada, presión nula ideal o resistencia
especificada. `at_position` corta exactamente la ley de área (radio lineal sigue
siendo radio lineal), conserva pérdidas y admite 0..100% de recorrido. Las zonas
son segmentos con Rf independiente, sin convertir densidad de relleno en Rf.

Convención: [p2,U2]=M[p1,U1], exp(jωt), U positivo hacia la boca.
Rama anterior cerrada: Yleft=−C/A (caudal entrando hacia atrás).
Rama posterior: Yright=(A−CZL)/(DZL−B); cierre: −C/D.
pnode=Ud/(Yleft+Yright), Uleft+Uright=Ud. La carga mecánica incluye
además el pistón exterior. Todos los estados son RMS; desplazamiento publicado
es pico. La presión compacta a 1 m es jρω(Um−Ud)/(2π), sin trayecto común.
No es una predicción espacial para fuentes separadas; cargas radiativas
independientes no incluyen impedancia mutua. Resistencia artificial sólo
especifica absorción y su eficiencia radiativa queda no disponible.

Pliegues: matriz serie B=−(R+jωM), con parámetros dimensionales suministrados,
pérdida R|U|² y energía reactiva no disipativa. El recorrido se describe mediante
longitudes. No se inventan pérdidas turbulentas sin geometría/datos de flujo.
No se coloca elemento serie exactamente en el nodo de inyección, pues requiere
especificar de qué lado del driver está situado.

Pruebas ejecutadas: impedancia analítica del tubo abierto/cerrado para driver
0/25/33/49/100%; modos impares de cuarto de onda; continuidad de presión,
conservación de caudal, cierre de boca, reciprocidad y subdivisión de cono;
inversión de polaridad; balance con zonas de Rf, áreas variables y pliegue R+M;
API Design y barridos. Los polos exactos ideales no resolubles dan error explícito.
La topología sigue experimental respecto a validación externa: las pruebas son
analíticas/invariantes, no un oráculo Hornresp TL ni mediciones espaciales.

Seguimiento numérico: las admitancias se combinan mediante productos cruzados,
Zline=Aleft·g/(Aleft·h−Cleft·g). Esto evita dividir por cero en nodos de presión;
una cancelación del denominador a escala de redondeo devuelve SingularSystem.
Para la rama posterior, g=DZL−B, h=A−CZL (cierre: g=D, h=−C).
La presión de extremo inicial se obtiene directamente como g·Ud/denominador.
Una resistencia artificial no añade una fuente exterior de SPL. Se comprueban
los polos impares de cuarto de onda acercándose por dos refinamientos de f:
la impedancia crece aproximadamente ×10 y el caudal del driver decrece ×0.1.

Contrato de terminación: una resistencia arbitraria y la frontera ideal de
presión nula usan `TerminationKind::PrescribedResistance`, nunca
MatchedAbsorber (que implica ausencia de reflexión). La etiqueta incorrecta
se reprodujo con la prueba existente y se corrigió sin cambiar estados ni
potencias (boundary-kind-before/after.log). Los matches exhaustivos de
TerminationKind deben incluir la nueva variante.
