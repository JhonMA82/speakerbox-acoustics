# Roadmap

## Objetivo

Mantener una librería acústica **de producción**, no una colección de modelos
sin validar. Cada topología que se marque como cerrada debe tener:

1. modelo físico implementado;
2. export propio de Hornresp 60.70;
3. test de producción puntual (SPL, `Ze`, `Xd`);
4. datos y documentación en `tests/data` y `docs/ALGORITHMS.md`.

La exportación de fabricación (DXF/SVG, STEP/STL, lista de corte, BOM) es una
fase posterior y pertenece a la capa CAD/CAM, no al núcleo acústico.

---

## Alcance v1

### Validado a producción

| Topología | Constructor / API | Evidencia |
|---|---|---|
| Bocina normal | `System` + `Horn` | `tests/production_accuracy.rs`, 3 diseños |
| Offset driver OD/OD1/OD2 | `OffsetHorn` | `tests/od_production.rs`, 3 diseños |
| Tapped horn TH (S2/S4) | `TappedHorn::new_4seg_th` | `tests/th_production.rs`, 2 diseños |
| Tapped horn TH1 (S2/S3) | `TappedHorn::new_4seg_th1` | `tests/th_production.rs`, 1 diseño |
| BP4 | `Bandpass::new_bp4` | `tests/bp_production.rs` |
| BP6 paralelo | `Bandpass::new_bp6` | `tests/bp_production.rs` |
| BP6 serie | `Bandpass::new_bp6_series` | `tests/bp_production.rs` |
| Caja sellada / ventilada / radiador pasivo | `System` | `tests/system_production.rs`, 3 diseños |

Tolerancias comunes: SPL RMS ≤ 2 dB / máximo ≤ 3,5 dB; `Ze` ≤ 15 % RMS /
35 % puntual; `Xd` ≤ 15 % RMS / 40 % puntual. La banda de producción se
declara en cada test: 10 Hz–1 kHz para OD, BP y cajas; TH y bocina normal usan
todos los puntos disponibles del export.

**Alcance v1 cerrado:** no quedan topologías validadas pendientes de oráculo.
Esto no implica SemVer 1.0: la crate sigue en 0.x y su API pública puede
cambiar entre versiones menores.

### Fuera de alcance v1

No se implementan en v1:

- BP8 serie/paralelo.
- BPA/BPB/BPC.
- DBR/ABC/APR.
- TH con cámara de garganta o trasera.
- Multiple entry (ME1/ME2).
- Compound, stubbed y paraflex.
- Perfiles tractrix/Bessel/OS en forma cerrada.
- Directividad, mapas polares y compresión de potencia.
- Exportación de fabricación.

---

## Fase 1 — v1 cerrado

### 1.1 TH1 (S2/S3) — completado

- `tests/data/th1_s2s3_record.txt` y `th1_s2s3_oracle.txt`.
- Test `th1_s2s3_meets_production_tolerances` en `tests/th_production.rs`.
- Métricas: SPL 0,55/2,69 dB; `Ze` 6,7/15,9 %; `Xd` 3,5/15,8 %.
- Documentado en `README.md`, `docs/ALGORITHMS.md` y `tests/data/README.md`.

### 1.2 Caja sellada / ventilada / radiador pasivo — completado

- `tests/data/sealed_a_*`, `vented_a_*`, `pr_a_*`.
- Test `tests/system_production.rs`.
- Métricas: sellada 0,00/0,00 dB; ventilada 0,06/0,20 dB; radiador pasivo
  0,38/0,97 dB.
- Documentado en `README.md`, `docs/ALGORITHMS.md` y `tests/data/README.md`.

### 1.3 Congelación v1 — completado

- Alcance v1 etiquetado en `README.md`.
- CI con `cargo test --workspace`, clippy, rustdoc, wasm y MSRV.
- Regla activa: no se aceptan nuevas topologías sin oráculo propio.

---

## Fase 2 — CAD/CAM y fabricación

Es la siguiente fase de producto, no de acústica:

- DXF/SVG de paneles y plantillas.
- STEP/STL de la caja/bocina.
- Lista de corte y BOM.
- Tolerancias de fabricación (kerf, espesor, holguras).
- Plantillas de montaje y orden de ensamblaje.
- CAM/G-code mediante software externo a partir de DXF/STEP.

El núcleo acústico entrega geometría y parámetros; la capa CAD/CAM decide cómo
se construye.

---

## Fase 3 — Multi-driver, arrays y topologías diferidas

### 3.1 `DriverBank` — Nivel A completado

Implementado en `src/driver_bank.rs`:

- N drivers idénticos.
- Cableado paralelo, serie y serie-paralelo.
- Isobaric (compound) por pares, con bobinas en serie o paralelo.
- Push-pull (misma respuesta lineal; la distorsión par no se modela).
- DVC paralelo/serie.
- `equivalent_driver()`, `bank_resistance()` y `suggested_volume_scale()`.

Validación: tests unitarios e integración propios. Además, comprobación local
contra los casos `Nd = 2P/2S` de `speaker-box-genius` (2 drivers sellada
paralelo/serie y 2 drivers ventilada), con SPL ~0,3 dB RMS, `Ze` ~3–4 % RMS y
`Xd` ~3 % RMS. Los fixtures de ese repositorio no se copian al repo por su
licencia; falta un export propio de Hornresp multi-driver para el test de
producción permanente.

Pendiente de esta capa:

- Bessel 5/6 (necesita array espacial).
- DVC Separate (dos canales independientes).
- Validación de producción con exports propios de 2/4 drivers y isobaric.

### 3.2 Importación WinISD/OpenISD — completado

Implementado en `src/driver_import.rs`:

- `.wdr` (WinISD): INI `[Driver]`, SI, coma o punto decimal.
- `.owdr` (OpenISD): JSON con `specs.woofer`, soporta `value` y `readings`.
- Derivación de `Cms`, `Mms`, `Rms` y `Bl` desde T/S cuando faltan.
- Conversión `Mmd = Mms − masa añadida de aire`.
- Metadatos: marca, modelo, `numVC`, `VCCon`, `Znom`, `Xmax`, `Pe`.
- Test unitarios con ficheros sintéticos; sin copiar fixtures de terceros.

### 3.3 `RadiatorArray` — pendiente

Para line arrays y torres con separación entre drivers:

- Posiciones, delays y ganancias por radiador.
- Suma compleja en campo lejano.
- Directividad vertical/horizontal.
- Bessel como caso particular de array.

### 3.4 Topologías diferidas

Orden sugerido si hay demanda real:

| Prioridad | Topología | Coste | Motivo |
|---|---|---|---|
| P1 | BP8 serie/paralelo | Alto | Extensión natural de BP6; menos común |
| P2 | TH con cámara de garganta/trasera | Medio-alto | Útil para TH con `Vrc/Lrc/Vtc/Atc` |
| P3 | Multiple entry ME1/ME2 | Alto | Muy específico; requiere solver nuevo |
| P4 | BPA/BPB/BPC | Alto | Variantes de bandpass de nicho |
| P5 | DBR/ABC/APR | Medio | Recintos exóticos |
| P6 | Compound/stubbed/paraflex | Alto | Bocinas plegadas y variantes de nicho |

Criterios de priorización:

1. Demanda real de usuarios de la app.
2. Existencia de un export propio de Hornresp.
3. Coste de implementación y validación.
4. Valor para el flujo CAD/CAM.

---

## Fase 4 — Física avanzada

Diferida hasta que el producto lo pida:

- Pérdidas viscotérmicas y absorbentes más detallados.
- Compresión de potencia y límites térmicos.
- Directividad, mapas polares e índice de directividad.
- Perfiles tractrix, Bessel, Le Cléac'h y OS en forma cerrada.
- Líneas de transmisión amortiguadas.
- Radiación no pistónica y modos del diafragma.

---

## Reglas para añadir una topología

1. Implementar el modelo y sus tests físicos.
2. Exportar un diseño propio desde Hornresp.
3. Añadir un test de producción puntual (no solo RMS).
4. Guardar los datos en `tests/data` con un README.
5. Documentar ecuaciones, calibración y límites en `docs/ALGORITHMS.md`.
6. Pasar CI completo: fmt, tests, clippy, rustdoc, wasm y MSRV.

Sin estos seis puntos, la topología queda como **experimental** y no entra en
el alcance v1.
