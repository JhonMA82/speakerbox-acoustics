# Spec — crossover y optimización (Fase D)

**Estado:** aprobada por instrucción del mantenedor (2026-11-13)
**Rama:** `feat/speakerworkshop-crossover`
**Referencia técnica:** [`docs/SPEAKERWORKSHOP_REUSE_RFC.md`](../../../../docs/SPEAKERWORKSHOP_REUSE_RFC.md) §10

## Intención

Diseñar filtros pasivos verificables y optimizar sus componentes contra una
respuesta objetivo, sin tocar los solvers acústicos y sin prometer simulación
multivía.

## Alcance

1. Respuesta analítica compleja de Butterworth (1–4) y Linkwitz-Riley (2, 4).
2. Síntesis pasiva Butterworth LP/HP 1–4, doblemente terminada.
3. L-pad con bypass explícito y Zobel.
4. Optimizador LM restringido contra magnitud en dB, con Jacobiano por
   diferencias centrales.
5. Pruebas unitarias y de integración que contrastan con la analítica.

## Fuera de alcance

- Síntesis pasiva de Bessel, Chebyshev, Legendre, Gaussian, Linear.
- Linkwitz-Riley pasivo (necesita secciones activas).
- Optimización de fase, de impedancia compleja o multivía.
- Referencia de presión compleja para combinar altavoces.

## Criterios de aceptación

| Criterio | Prueba |
|---|---|
| La escalera reproduce la analítica | `1e-9` absoluto, LP y HP, órdenes 1–4 |
| L-pad conserva `Z0` | `Zin` dentro de `1e-9`, nivel dentro de `1e-12` |
| Zobel aplana la impedancia | dispersión `< 3 %` |
| Optimizador reduce el coste | `final_cost < initial_cost`, RMSE `< 0.05 dB` |
| MSRV y WASM | contenedor `rust:1.75` |
