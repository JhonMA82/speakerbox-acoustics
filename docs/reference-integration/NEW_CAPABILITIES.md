# Capacidades implementadas
`Segment::with_flow_resistivity(Rf)` configura resistencia serie distribuida,
Rf en Pa·s/m², cero por defecto. Darcy lineal homogéneo; no identifica un
material poroso a partir de densidad ni reemplaza modelos empíricos medidos.
Se conserva la pérdida shunt anterior. No usar simultáneamente ambos para
representar la misma pérdida sin una identificación física independiente.

`reference_design::sealed_alignment(Fs,Qts,Vas,Qtc)` calcula propuesta ideal
con F3 exacta del filtro de segundo orden; Qtc>Qts. No incluye Le ni fugas.

`reference_design::uniform_transmission_line(driver,L,S,x/L,Rf)` devuelve
OffsetHorn sin calibración empírica, convertible a Design. Extremo inicial
cerrado, boca radiativa, cara exterior del driver directa, posición [0,1).
Pruebas en 0,25,33 y 49 %. No es otra arquitectura TMM ni una topología
certificada por Hornresp. No soporta todavía boca cerrada ni pliegues.
