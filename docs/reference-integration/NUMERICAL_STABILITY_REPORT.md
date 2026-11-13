# M1 — propagación compleja

Base: 3e9e15d, rama feature/complete-acoustic-reference-integration.
La regresión `completion_numerics` falló antes del cambio (EXIT=101): el caso
10 Hz, 0.8 m, Rf=1e8 devolvía respuesta aparentemente válida.

Con exp(jωt), d[p,U]/dx=[[0,a],[b,0]][p,U],
a=−(Rf+jωρ)/S y b=−(loss+j)ωS/(ρc²). γ²=ab.
En sección constante M=I cosh(γL)+A L sinhc(γL), sin RK4.
Para sección variable se resuelve RK4 con |γ|L/step≤0.03 y refinamiento
hasta cambio normalizado <1e-7; máximo 60000 pasos. La cuadratura disipativa
integra Rf|U|²/S + loss ωS|p|²/(ρc²), independientemente del flujo residual.

Dominio de matrices de transferencia: |Re γ|L≤8; fuera devuelve error en
`try_matrix`, NaN en la API histórica infalible, detectado por los solvers
estrictos. La cota limita la amplificación de redondeo exp(2|Re γL|)ε;
no representa un límite físico del material. Tramos/cascadas muy atenuantes
requieren una formulación de dispersión, fuera de este cambio. No se afirma
validez de una cascada arbitraria sólo porque cada tramo cumpla la cota.

Pruebas: solución compleja exacta frente RK4 32/128 pasos, reducción de error,
Rf 0/1/1000/100000, 10/100/1000 Hz y distintas longitudes; flujo de potencia
frente integral positiva; perfil cónico frente 4096 pasos; estrés explícitamente
rechazado. Se conserva la ruta histórica sin pérdidas para evitar coste adicional.

Seguimiento: también se comprueba la suma de atenuaciones en las rutas estrictas
System/TH/OD/TL; subdividir un tramo no permite eludir el diagnóstico. Se añadió
una regresión de 80 tramos individualmente admisibles cuya cascada es inválida.
La API `Segment::matrix` ya no representa f≤0 como una identidad física; use
`try_matrix` para diagnóstico. Se verifica finitud de las matrices analíticas.
