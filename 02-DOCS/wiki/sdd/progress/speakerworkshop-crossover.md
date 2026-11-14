# Progreso — crossover y optimización (Fase D)

**Rama:** `feat/speakerworkshop-crossover`
**Spec:** [speakerworkshop-crossover.md](../specs/speakerworkshop-crossover.md)
**Informe:** [`SPEAKERWORKSHOP_CROSSOVER_REPORT.md`](../../../../SPEAKERWORKSHOP_CROSSOVER_REPORT.md)

## Ledger

| Fecha | Hecho | Evidencia |
|---|---|---|
| 2026-11-13 | Respuesta analítica Butterworth/LR | `crossover::tests`, 1e-12 en corte |
| 2026-11-13 | Síntesis pasiva LP/HP 1–4 | escalera vs analítica, 1e-9 absoluto |
| 2026-11-13 | L-pad y Zobel | `Zin = Z0`, dispersión Zobel `< 3 %` |
| 2026-11-13 | Optimizador LM | RMSE `< 0.05 dB`, coste decreciente |
| 2026-11-13 | Workspace | 350 tests, 0 fallos |
| 2026-11-13 | Calidad | fmt, clippy y rustdoc limpios |
| 2026-11-13 | MSRV/WASM | contenedor `rust:1.75` |

## Decisiones

- Solo Butterworth se sintetiza pasivamente: es la única tabla que se puede
  verificar contra la transferencia sin copiar coeficientes.
- Linkwitz-Riley se expone como respuesta analítica, no como red pasiva.
- El optimizador es de una salida y de magnitud; multivía queda fuera.

## Cola

- Fase E: inventario de audio I/O, sin implementación.
- Referencia de presión compleja para simulación multivía.
