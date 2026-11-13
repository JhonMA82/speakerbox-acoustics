# Rendimiento y comprobaciones

Tres ejecuciones release por versión, mismo ejemplo perf_sweep y entorno.
No se midieron por separado las APIs nuevas: no se anuncia aceleración.

| Caso | Antes ms (mediana) | Después ms (mediana) |
|---|---:|---:|
| System bocina 4 tramos 500 | 15.217 | 19.570 |
| System bocina 4 tramos 2000 | 64.855 | 65.608 |
| TappedHorn 4 tramos 500 | 34.036 | 35.124 |
| TappedHorn 4 tramos 2000 | 126.165 | 131.793 |
| Bandpass BP6 serie 500 | 0.334 | 0.353 |
| Bandpass BP6 serie 2000 | 1.588 | 1.637 |
| 100 cambios × 50 puntos 5000 | 170.801 | 191.785 |
