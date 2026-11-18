# Guía para agentes

Este documento describe el repositorio, los comandos, las invariantes y el
flujo para extender la librería sin romper la validación de producción.

## Localizar el cambio

Consulte el [mapa de responsabilidades y pruebas](ARCHITECTURE.md). Parta del tipo que posee el dato y siga sus consumidores. Compruebe las ramas de `Design` antes de extender la fachada.

| Cambio | Punto de entrada y comprobación de impacto |
| --- | --- |
| Perfil, pérdida o solver | `horn_profile` define geometría persistente, `segment` la propaga y `thermoviscous` delimita el régimen. Revise extremos, matriz uniforme, convergencia, disipación, serialización y comparador externo. |
| Topología o red | Revise primero los elementos y constructores existentes. Dibuje tomas, fronteras y salidas de cada cara. Compare contra el solver especializado, continuidad, reciprocidad y potencias antes de contrastar con Hornresp. |
| Terminación, driver, magnitud o unidad | Siga la magnitud desde `radiation`, `driver` o `element` al punto específico, la fachada, `EnergyReport` y multivía. Verifique signos, pico/RMS, silencio y absorción sin campo. |
| Registro, driver importado o medición | Edite el lector propietario. Conserve unidades de origen, campos opcionales y errores. Pruebe registros antiguos, fase ausente, ida y vuelta donde se ofrece y ficheros malformados. |
| Optimizador | Distinga coste acústico, ajuste de parámetros medidos y optimización de circuitos. Compruebe presupuesto, restricciones y determinismo en el módulo correspondiente. |
| Fallo de validación o regresión | Reproduzca el test puntual con `--nocapture`. Fije excitación, área, boca, aire, fase, banda y malla. Compare fasores y energía antes de tocar calibración, tolerancias u oráculos. |
| Nueva salida pública | Revise compatibilidad de structs y enums, unidades, ausencias, documentación de usuario y Rustdoc. Ejecute las comprobaciones del [archivo raíz](../AGENTS.md). |

Extienda el elemento existente si comparten ecuaciones y condiciones de frontera. Use una variante explícita cuando cambie la interpretación física. Cree un módulo cuando haya responsabilidad y consumidores identificables. Escale las decisiones que alteren contratos públicos, convenciones o calibraciones. Un modelo con comparación parcial conserva la etiqueta experimental y la prueba fallida.

## Cómo añadir una topología

1. Implemente el modelo en `src/` con constructores claros y sin `unsafe`.
2. Genere un registro y un `All Chart Data` propios de Hornresp 60.70.
3. Guarde los datos en `tests/data` con nombres `topologia_a_record.txt` y
   `topologia_a_oracle.txt` (o `_spl`, `_ze`, `_xd`).
4. Añada un test `tests/<topologia>_production.rs` con las tolerancias
   puntuales:
   - SPL RMS ≤ 2 dB, máximo ≤ 3,5 dB.
   - `Ze` RMS ≤ 15 %, máximo ≤ 35 %.
   - `Xd` RMS ≤ 15 %, máximo ≤ 40 % (ignorar `Xd ≤ 0.02 mm`).
5. Documente el modelo en [Algoritmos](ALGORITHMS.md) y el alcance en [Validación](VALIDATION.md). Registre la procedencia junto al fixture.
6. Actualice `ROADMAP.md` si cambia el alcance.
7. Ejecute todos los comandos obligatorios.

Sin oráculo propio, la topología queda como **experimental** y no entra en v1.

## Validación de producción

- Las puertas v1 usan exports propios. Las comparaciones adicionales usan fixtures externos con revisión, procedencia y alcance explícitos.
- `OraclePoint::parse_tsv` acepta el `All Chart Data` de Hornresp con coma o
  punto decimal y devuelve `Result`; conserva la posición de las columnas y
  rechaza celdas inválidas o columnas faltantes.
- `validate` compara SPL, `Ze` y `Xd`; `Xd` ya es pico en todos los puntos.
- `validate` **no** compara `ze_phase_deg`; no declare una validación completa
  de la impedancia.
- Un oráculo vacío o con puntos inválidos no aprueba: `passes`/`passes_pointwise`
  exigen `n > 0`, `invalid_points == 0` y métricas finitas.
- `parse_csv` exige exactamente 4 columnas; `parse_tsv` usa índices fijos y
  admite columnas extra.
- `passes_pointwise` aplica las tolerancias puntuales, no solo RMS.
- Los tests de producción deben usar la misma calibración global para todos
  los diseños de la topología.

## Trampas conocidas

- **No** añada líneas crudas como `TH1 = 1` a un registro de texto para
  importarlo: el software de referencia puede crashear. El flag TH1 se activa en la UI con
  doble clic en `TH`; el registro exportado ya trae `TH1 = 1`.
- Los registros exportados usan **CRLF** y a veces coma decimal. Use
  `OraclePoint::parse_tsv` o un parser que tolere ambos.
- En OD, `Vrc` no actúa como cámara sellada en los registros validados: la
  parte trasera es un radiador directo (`rear_chamber_m3 = None`).
- En BP6S, el `QL` se aplica solo a la cámara delantera y el puerto trasero
  lleva corrección de salida embridada (`0.8216·a`).
- En cajas ventiladas, el puerto usa `EndCorrection::Inner(1.5)` en la
  validación; no confunda la corrección del puerto con la del radiador pasivo.
- El registro del radiador pasivo puede guardar `Cmp` como `4.00E-01`; use el
  valor del registro, no el que se haya pedido en la especificación.
- `System` reporta `Xd` de pico; `validation` ya no multiplica por `√2`.
- No cambie la firma pública de `DesignPoint` sin actualizar `design.rs`,
  `docs/INTEGRATION.md` y los ejemplos.
- En `.wdr` (WinISD) los valores son SI y el separador decimal puede ser coma;
  `Mms` es masa total, así que el importador resta la masa añadida del aire.
- En `.owdr` (OpenISD) cada campo puede venir como `value` o en los valores del objeto
  `readings`, bajo `read_value`; el importador acepta ambos.
- No commitee ficheros `.wdr`/`.owdr` de terceros sin licencia; los tests del
  importador usan cadenas sintéticas propias.

## Estilo

- Rust estable, MSRV 1.75, sin dependencias nuevas salvo necesidad clara.
- `cargo fmt` y `clippy -D warnings` son obligatorios.
- Documentación y comentarios en español, consistentes con el resto del
  repositorio.
- Cambios mínimos y reversibles; no reescriba módulos que ya están validados.
- Los ejemplos deben compilar con `cargo check --all-targets`.
