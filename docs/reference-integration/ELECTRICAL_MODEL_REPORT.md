# M2 — semiinductancia

`CoilModel::Inductance` conserva Re+jωLe. La selección explícita
`SemiInductance { ke }` usa Re+Ke√(jω) y sustituye Le, sin sumarlo.
Ke tiene unidades Ω√s; para ω≥0 la rama principal es (1+j)√(ω/2).
Su parte real es positiva: calor adicional |I|²Ke√(ω/2), además de Re|I|².
EnergyReport expone calor DC, exceso y total; todos los solvers usan la
misma impedancia de bobina y frecuencia. Los bancos escalan Ke igual que Re/Le.
Los importadores mantienen el modelo clásico; no infieren Ke de T/S ni de Le.

Compatibilidad: los constructores conservan resultados; los literales externos
`Driver { ... }` deben añadir `coil_model: CoilModel::Inductance`.
No se ha implementado un modelo combinado sin datos independientes de ajuste.
Pruebas: DC, parte real/imaginaria exactas, pasividad, independencia de Le,
rechazo de Ke negativo, calor adicional y balance energético con Rg no nula;
suite energética y regresiones de integración. No se afirma validación ZMA
experimental: no hay medición con procedencia/referencia suficiente disponible.

Cobertura adicional: escala eléctrica de Ke en bancos serie/paralelo y balances
semiinductivos en Bandpass, TappedHorn y OffsetHorn (no sólo System).
