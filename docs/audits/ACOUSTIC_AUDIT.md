# Auditoría de consistencia física

Base: `558e2fa4dee76e396ff13c00b744c5630b84bc3b` (main). Rama local:
`fix/acoustic-physical-consistency`. Árbol inicial limpio. Fecha local: 2026-11-12.
Sin push, merge ni modificaciones de calibración u oráculos.

## Línea base real

Se leyeron AGENTS.md, REVIEW.md, README.md, docs/AGENTS.md,
docs/ALGORITHMS.md, docs/VALIDATION.md, LIBRARY_STABILIZATION_REPORT.md y
los tres informes SDD de estabilización antes de cambiar el motor.

El entorno inicialmente carecía de Rust: todos los comandos solicitados
fallaron con código 127 (`raw/baseline-*.log`). Se instaló Rust 1.85.0,
rustfmt, clippy, MSRV 1.75.0 y el target wasm32-unknown-unknown.
La repetición sobre el motor intacto está en `raw/base-*.log`:
fmt, tests debug/release, rustdoc, MSRV y WASM pasan. Clippy falla por dos
`needless_range_loop` preexistentes en la sustitución hacia atrás de tapped.rs.
Métricas Hornresp: `raw/base-8.log`. Cinco mediciones release:
`raw/base-perf-*.log`; no se usa la medición histórica de otra máquina.

## Confirmación por regresión

`cargo test --test acoustic_consistency -- --nocapture`: diez pruebas fallan
sobre la base, antes de modificar el motor (`raw/regressions-before.log`).
La suite cubre AC-001/002/003/004/005/006/007 y Qts con Rms=0.
AC-008 requiere distinguir la carga de pistón en pantalla de la aproximación
empírica de campo/potencia; no se cambiará la calibración para igualar curvas.

Este registro se ampliará con implementación y verificación final.

## Hallazgos confirmados

| ID | Severidad | Ubicación | Confirmación y resolución |
|---|---|---|---|
| AC-001 | Alta | system::loads/solve_point | Rserie no cargaba el diafragma. Rama R+Zhorn y derivaciones coherentes; pruebas de carga y balance. |
| AC-002 | Alta | offset::solve_point | Puerto y PR cargaban pero no radiaban. Caudales complejos -Ud·Zrear/Zsalida y suma compacta; cámara sellada sin salida. |
| AC-003 | Alta | bandpass::solve_point | polarity validada e ignorada. Excitación Eg·polarity; corriente calculada desde Ze, sin alterar el modelo eléctrico. |
| AC-004 | Alta | segment::validate/solve_xt/area | Cilindro y longitud cero incompatibles aceptados; Hypex positivo contraído no reproducible. Rechazo y dominio explícito; primera raíz de contracción con T<0. |
| AC-005 | Alta | element::parallel | Antirresonancia se transformaba en corto. Forma escalada sin producto, abierto ideal y try_parallel con error. |
| AC-006 | Alta | bandpass::port_matrix | B concentrado con signo opuesto a Webster. B=-Z y misma longitud acústica interior que Pipe. |
| AC-007 | Alta | offset::solve_point | Fase de boca no correspondía al SPL combinado. Ahora arg(p_total/j). |
| AC-008 | Alta / límite de modelo | radiation y potencias de solvers | Documentación contradictoria de Ω; cancelación de R1 a x pequeño y factor 2 en X1 asintótico. Series corregidas; modelo general de energía pendiente, explicitado. |
| AC-009 | Media | driver::qts | Rms=0 daba ∞/∞. Suma de inversos de Q. |
| AC-010 | Alta | tapped::solve | Eg=0 perdía Z de la red. Resuelve a 1 V, obtiene carga y escala solo fuente/caudales. |
| AC-011 | Alta | tapped/offset, corriente | Polaridad invertía velocidad pero no referencia de fuerza de contra-FEM. I=Eg/Ze; potencia eléctrica invariante con polaridad. |
| AC-012 | Alta | segment::area(Custom) | max(0) ocultaba NaN/negativos. Sin recorte; validación de extremos, muestras y etapas RK4. |
| AC-013 | Media | phase; system/design::response | Diferencia atrasada asignada al punto actual en rejilla irregular. Derivada cuadrática, índices ordenados/deduplicados y cancelaciones explícitas. |
| AC-014 | Media | driver_bank::equivalent_driver | Driver inválido se devolvía como Ok; count podía truncarse. Validación base/derivado y conversión u32 comprobada. |
| AC-015 | Alta | tapped::neck_impedance | Para tap_a>1 solo incluía el primer tramo del stub. Cascada completa contrastada con OD independiente. |
| AC-016 | Media | TransferMatrix::transform_impedance | Terminación abierta producía ∞/∞. Límite -D/C explícito. |
| AC-017 | Media | radiation::struve_h1 | Rama asintótica real negativa usaba Y1(x<0)=NaN; H1 es par. Uso de abs(x). |
| AC-018 | Media | driver_import::import_owdr | Le presente no numérica se sustituía por cero. Rechazo de campos numéricos presentes malformados. |
| AC-019 | Alta | system::solve_point | Bocina con puerto/PR trasero omitía la salida trasera del SPL. Suma compacta con la misma polaridad de recinto. |
| AC-020 | Media | system::loads | Fuga Some(0) se ignoraba. Ahora corto físico; None la desactiva. |

Las 23 pruebas físicas de `tests/acoustic_consistency.rs` se ejecutaron en un
worktree separado del commit base y **fallan todas** (registro final de
regresión). Pasan sobre la implementación corregida. Hay pruebas adicionales
de propiedades, APIs y equivalencia en `tests/prepared_engine.rs` y phase.rs.
No todas las filas son fenómenos independientes: algunas comparten regresión.

## Matices y descartes

- **No confirmado:** que Ω esté escalando realmente la carga del pistón. El
  código la deja 2π; lo incorrecto era su documentación. Se preservó esa
  aproximación y se explicitan sus límites, sin ajustar constantes.
- **No se rechaza todo Hypex contraído:** con T<0 hay un dominio físico válido.
  La raíz antigua podía incluir una cintura no solicitada. La elección de
  primera raíz es deliberada y documentada.
- **No se exige continuidad de área universal:** un salto con p/U continuos
  es una unión ideal válida en una red 1D. Su pérdida real no está modelada.
- **No es regresión de excursión TH/BP:** el informe ampliado divide por los
  puntos de Xd realmente comparados; el reporte histórico dividía por todos.
- **No se afirma masa de aire duplicada universal:** System refleja radiación
  frontal; TH añade una masa lumped y OD la hace configurable. La distinción
  Mmd/Mms es explícita. La exactitud de OD+aire+placa requiere definir ambas
  caras/pantalla; no se elimina una masa calibrada sin justificar esa geometría.
- Cableado estándar, DVC e isobárico no muestran errores algebraicos en el
  dominio de los equivalentes lineales existentes. Falta validación espacial
  y multi-driver, no se la infiere de pruebas de un solo pistón.

## Alcance de revisión

Se inspeccionaron los módulos solicitados, los solvers y validadores, sus
constructores, conversiones SI, importadores y las pruebas/fixtures existentes.
No es una demostración formal de ausencia de errores ni una certificación de
acústica espacial. No se reescribió el motor ni se añadieron dependencias.

Fuentes primarias consultadas para contrastar convenciones y límites:
[NIST DLMF 11.2](https://dlmf.nist.gov/11.2),
[NIST DLMF 11.6](https://dlmf.nist.gov/11.6),
[BYU: Radiation Impedance](https://acoustics.byu.edu/animations-impedance).
Las derivaciones de matrices, Hypex y balance usan las ecuaciones y fasores
explícitos del repositorio; las pruebas no copian resultados del solver como
oráculos.

## Estado

Errores reproducidos: corregidos y cubiertos por regresiones. Las limitaciones
de potencia/radiación general siguen abiertas con bloqueo físico documentado;
**no se declara cumplido el criterio de cierre físico integral**. La rama
queda revisable, con oráculos intactos y controles técnicos ejecutados.
