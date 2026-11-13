# M4 — optimización funcional

Implementación independiente sin dependencias: Nelder–Mead acotado (coordenadas
normalizadas) y DE/rand/1/bin, F=0.8, CR=0.9 y generador determinista con semilla.
`minimize` recibe límites, parámetros bloqueados, presupuesto, tolerancia,
función objetivo y callback de cancelación. Conserva el mejor candidato válido;
None/NaN/error del evaluador descarta un candidato. No muta diseños originales.
Los resultados distinguen convergencia geométrica, presupuesto agotado,
cancelación y ausencia de candidatos válidos. Convergencia no garantiza óptimo
global. Caché local exacta evita reevaluaciones; no sobrevive al cálculo.

`acoustic_cost`: objetivos/limitaciones duras de SPL (ambos lados para target),
excursión pico, velocidad de puerto, potencia y rendimiento. Éstos últimos
requieren calidad SingleBaffledPiston; no optimiza una eficiencia aproximada
como si fuese validada. Extensión y uniformidad se expresan mediante objetivos
por frecuencia. Dimensiones, volumen y restricciones geométricas se expresan
en el callback que construye/valida el candidato antes de llamar al solver.

Pruebas observadas: ambos algoritmos alcanzan error cuadrático <1e-10 en
paraboloide; Rosenbrock <1e-8; DE Rastrigin <1e-6; reproducibilidad, bloqueos,
límites, cancelación, presupuesto, todos inválidos; ajuste acústico recupera
25 litros de caja sellada a <0.01 litro y coste SPL <1e-8.
No hay afirmación de optimización global garantizada ni parametrización CAD.
