# Spec — identificación de impedancia (Fase C)

**Estado:** aprobada por instrucción del mantenedor (2026-11-13)
**Rama:** `feat/speakerworkshop-identification`
**Referencia técnica:** [`docs/SPEAKERWORKSHOP_REUSE_RFC.md`](../../../../docs/SPEAKERWORKSHOP_REUSE_RFC.md) §9

## Intención

Convertir una curva de impedancia medida en el circuito equivalente de siete
parámetros, sin tocar `Driver` ni los solvers acústicos, y sin inventar
magnitudes que los datos no determinan.

## Alcance

1. Crate `crates/speakerbox-measure`, dependiente de `speakerbox-circuits`.
2. Parser y serializador FRD/ZMA (Hz, magnitud, fase en grados).
3. `EquivalentImpedance` en SI con validación y evaluación.
4. Ajuste Levenberg–Marquardt en log-parámetros con residual complejo,
   límites físicos y `FitReport`.
5. `TsEstimate` parcial: `Fs`, `Qms`, `Qes`, `Qts` derivados del circuito.
6. Experimentos: masa añadida (`Mms`, `Cms`) y caja sellada (`Vas`).
7. Pruebas unitarias y de integración, con fixture generado por una
   implementación independiente.

## Fuera de alcance

- Conversión del ajuste a `Driver` (faltan `Bl`, `Sd` y base física).
- `Vas` por masa añadida (requiere `Sd`).
- Calibración de curvas FRD (1 V, 2.83 V, 1 W).
- Fase D (crossover y optimizador) y Fase E (audio I/O).

## Contratos

- `DesignPoint.phase_deg` no se reinterpreta.
- El crate acústico no depende de `speakerbox-measure`.
- Toda entrada inválida devuelve `MeasureError` o `CurveParseError`; sin
  pánicos.
- La extrapolación de curvas sigue prohibida.

## Criterios de aceptación

| Criterio | Prueba |
|---|---|
| Recupera parámetros conocidos | fixture ZMA independiente, error `< 5 %` |
| El coste no empeora | `FitReport.final_cost < initial_cost` |
| Avisa de curvas sin resonancia | `FitWarning::NoResonance` |
| Rechaza experimentos incoherentes | masa que no baja `Fs`, caja que no la sube |
| MSRV y WASM | contenedor `rust:1.75` |
