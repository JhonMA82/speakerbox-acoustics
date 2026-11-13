# Validación física
RF distribuido: dp/dx=-(Rf+jωρ)U/S. Con el convenio exp(jωt),
k=(ω/c)sqrt(1-jRf/(ωρ)), Zc=(ρc/S)sqrt(1-jRf/(ωρ)).
La implementación integra las ecuaciones de estado existentes, no copia
el solver externo. Pérdida adicional ∫Rf|U|²/S dx, integrada por RK4,
comparada independientemente con Re(pin Uin*)-Re(pout Uout*).

Cuatro pruebas nuevas en tests/reference_integration.rs: matriz analítica
compleja, pérdida positiva y balance; identidad exacta Rf=0; raíz de media
potencia para Qtc Butterworth/Bessel/0.957; acoplamiento modal TL
Um/Ud=cos(kx)/(cos(kL)+j Zrad/Z0 sin(kL)), con balance amortiguado.
Rf=300 y frecuencias 10–1000 Hz en contraste de matriz. No certifica
resistividades arbitrariamente elevadas ni materiales reales.

Evidencia: soluciones analíticas (nivel 1) e invariantes de red. Los logs
Hornresp existentes son nivel 3. No hay medición externa acreditada (nivel 2)
ni comparación ejecutada entre motores (nivel 4). No se presentan salidas
externas como oráculos. El adaptador cruzado aún no está implementado.
