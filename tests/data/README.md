# Datos dorados

Los ficheros de este directorio se generaron con **Hornresp 60.70** a partir de
diseños propios.

## Diseño 1 — registro por defecto

- `default_record.bin`: registro real de 734 bytes creado por Hornresp al
  arrancar (bocina cónica + exponencial).
- `default_oracle.csv`: 44 puntos (20 Hz–10 kHz) del export
  `File > Export > All Chart Data`.
- `default_oracle_full.txt`: export completo de 533 puntos (10 Hz–20 kHz).

Parámetros: `S1=80`, `S2=350`, `S3=2400 cm²`; `L12=60`, `L23=75 cm`;
`Mmd=20 g`; `Vtc=900 cc`; `Vrc=14 L`.

## Diseño 2 — segundo diseño real

Generado en la UI de Hornresp: **Add**, edición de `S1`, `S2`, `S3`, `L12`,
`L23` y `Mmd`, y **Calculate**. El export se hizo con
`File > Export > All Chart Data`.

- `design2_record.bin`: registro de 734 bytes reconstruido con los valores
  usados por la UI (Hornresp no persistió los campos editados en el `.dat`,
  pero el cálculo y el export sí usaron esos valores).
- `design2_oracle.txt`: export completo de 533 puntos (10 Hz–20 kHz).

Parámetros: `S1=100`, `S2=500`, `S3=3000 cm²`; `L12=50`, `L23=80 cm`;
`Mmd=30 g`; `Vtc=900 cc`; `Vrc=14 L`.

## Diseño 3 — tercera geometría

Generado en la UI de Hornresp (**Add**, edición y **Calculate**). Export completo
`design3_oracle.txt` (533 puntos) y ficha etiquetada `design3_record.txt`.

Parámetros: `S1=120`, `S2=600`, `S3=3200 cm²`; `L12=55`, `L23=70 cm`;
`Sd=400 cm²`, `Mmd=25 g`; `Vrc=18 L`, `Vtc=1200 cc`, `Atc=400 cm²`.

- `design3_record.bin`: registro de 734 bytes con esos valores.
- `design3_oracle.txt`: export completo de 533 puntos (10 Hz–20 kHz).

## Uso

```bash
cargo test --test production_accuracy -- --nocapture

cargo run --release --example compare_oracle -- \
  tests/data/design2_record.bin tests/data/design2_oracle.txt
```

## BP4 — cuarto orden

- `bp4_a_record.txt`: registro exportado desde la UI con `Export Hornresp Record`.
- `bp4_a_spl.txt`, `bp4_a_ze.txt`, `bp4_a_xd.txt`: exports de 533 puntos
  (10 Hz–20 kHz) de las gráficas Acoustical Power, Electrical Impedance y
  Diaphragm Displacement.

Parámetros: cámara delantera `Vc1=15 L`, trasera `Vc2=12 L`; puerto delantero
`Ap1=38.48 cm²` (7 cm de diámetro) y `Lp1=15 cm`; `QL=7`; driver DSA215-8
(`Sd=211.20 cm²`, `Bl=10.65`, `Cms=5.91E-04`, `Rms=4.11`, `Mmd=31.43 g`,
`Le=1 mH`, `Re=5.90 Ω`); `Ang=2.0 x Pi`, `Eg=2.83 V`.

## BP6 paralelo — sexto orden

- `bp6_a_record.txt`: registro con `BP6P=1` (paralelo).
- `bp6_a_spl.txt`, `bp6_a_ze.txt`, `bp6_a_xd.txt`: exports de 533 puntos.

Parámetros: delantera `Vc1=15 L`, trasera `Vc2=20 L`; puerto delantero
`Ap1=38.48 cm²` / `Lp1=15 cm`; puerto trasero `Ap2=28.27 cm²` (6 cm) /
`Lp2=20 cm`; `QL=7`; mismo driver.

La validación de producción se ejecuta con:

```bash
cargo test --test bp_production -- --nocapture
```

## BP6 serie — validado

- `bp6s_a_record.txt`: registro con `BP6S = 1`, misma geometría que
  `bp6_a_record.txt` (Vc1=15 L, Vc2=20 L, Ap1=38,48 cm²/Lp1=15 cm,
  Ap2=28,27 cm²/Lp2=20 cm, QL=7).
- `bp6s_a_spl.txt`, `bp6s_a_ze.txt`, `bp6s_a_xd.txt`: exports de 533 puntos.

En la carpeta `Export` de Hornresp, los ficheros de Ze y Xd se guardaron
intercambiados (`bp6s_a_ze.txt` contenía Xd y `bp6s_a_xd.txt` contenía Ze).
En `tests/data` están corregidos con los nombres correctos.

Estado: **validado a nivel de producción** con `tests/bp_production.rs`:

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| BP6S-A | **0,26 / 1,50 dB** | **1,4 / 6,0 %** | **3,7 / 33,0 %** |

El modelo usa la matriz de transmisión del puerto trasero y la admitancia de
ambas cámaras. La salida interior del puerto trasero añade la corrección
embridada `0.8216·a`; el `QL` se aplica solo a la cámara delantera, escalado
por `fb_front/fb_rear`.

## Caja sellada / ventilada / radiador pasivo

Tres diseños con el mismo driver que TH/OD/BP:

- `sealed_a_record.txt`, `sealed_a_oracle.txt`: caja sellada, `Vrc=50 L`,
  sin puerto.
- `vented_a_record.txt`, `vented_a_oracle.txt`: caja ventilada, `Vrc=100 L`,
  `Ap=100 cm²`, `Lpt=10 cm`, `QL=7`.
- `pr_a_record.txt`, `pr_a_oracle.txt`: radiador pasivo, `Vrc=100 L`,
  `Sp=350 cm²`, `Cmp=4,00E-01 m/N`, `Mmp=40 g`, `Rmp=4,0 N·s/m`.

El registro del radiador pasivo guarda `Cmp = 4.00E-01`; se usa tal cual, en
m/N. El puerto ventilado se modela en la librería con
`EndCorrection::Inner(1.5)`, que reproduce la corrección combinada de Hornresp.

Estado: **validado a nivel de producción** con `tests/system_production.rs`
(10 Hz–1 kHz):

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| Sellada-A | **0,00 / 0,00 dB** | **0,0 / 0,0 %** | **0,0 / 0,0 %** |
| Ventilada-A | **0,06 / 0,20 dB** | **0,7 / 3,1 %** | **0,4 / 2,6 %** |
| Radiador pasivo-A | **0,38 / 0,97 dB** | **2,5 / 9,3 %** | **1,0 / 3,6 %** |

## TH / TH1 — tapped horns

- `th1_record.txt`, `th1_oracle.txt`, `th1_za.txt`: primer diseño TH con
  tomas **S2/S4** (flag `TH = 1`).
- `th2_oracle.txt`, `th2_za.txt`, `th2_akabak.aks`: segundo diseño TH con
  tomas S2/S4.
- `th1_s2s3_record.txt`, `th1_s2s3_oracle.txt`: variante **TH1** con tomas
  **S2/S3** (flag `TH1 = 1`), misma geometría que el primer diseño.

TH1 y TH2 son etiquetas de los dos diseños S2/S4; no deben confundirse con el
flag TH1 de S2/S3. La validación de producción está en
`tests/th_production.rs`:

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| TH1 (S2/S4) | **0,53 / 1,59 dB** | **7,0 / 29,9 %** | **4,3 / 18,6 %** |
| TH2 (S2/S4) | **0,52 / 1,63 dB** | **7,1 / 30,9 %** | **4,5 / 19,0 %** |
| TH1 (S2/S3) | **0,55 / 2,69 dB** | **6,7 / 15,9 %** | **3,5 / 15,8 %** |

## OD / OD1 / OD2 — driver desplazado

- `od_a_record.txt` + `od_a_oracle.txt` + `od_a.aks` (OD, driver en S2).
- `od1_b_record.txt` + `od1_b_oracle.txt` + `od1_b.aks` (OD1, driver en S3).
- `od2_c_record.txt` + `od2_c_oracle.txt` (OD2, driver en S4; Hornresp no
  permite exportar AkAbak para OD2).
- `od_horn_spl.txt`, `od_direct_spl.txt`: descomposición de OD en salidas
  *Horn* y *Direct Radiator*.
- `od2_horn_spl.txt`, `od2_direct_spl.txt`, `od2_combined_spl.txt`:
  descomposición de OD2. La suma compleja de Horn y Direct reproduce
  `od2_c_oracle.txt`.
- `od1_horn_spl.txt`, `od1_direct_spl.txt`, `od1_combined_spl.txt`: export
  de OD1 **no usado** para la validación. Las tres curvas no son consistentes
  entre sí (probablemente se exportaron con otra selección de salida o desde
  otro registro). La validación de OD1 usa solo el oráculo combinado
  `od1_b_oracle.txt`.

Los `.aks` confirman que la parte trasera del driver es un **radiador directo**
(`Vrc` no actúa como cámara sellada en estos registros). El modelo `OffsetHorn`
usa `rear_chamber_m3 = None` para esa topología y suma la presión del radiador
trasero con la de la boca.

Estado: **validado a nivel de producción** con `tests/od_production.rs`
(10 Hz–1 kHz, 533 puntos):

| Caso | SPL RMS / máx | Ze RMS / máx | Xd RMS / máx |
|---|---:|---:|---:|
| OD-A | **0,85 / 1,59 dB** | **4,7 / 11,4 %** | **4,0 / 16,4 %** |
| OD1-B | **0,85 / 1,77 dB** | **4,7 / 12,0 %** | **4,1 / 17,2 %** |
| OD2-C | **0,85 / 1,61 dB** | **4,8 / 11,7 %** | **4,3 / 18,9 %** |

La calibración es común a los tres diseños: `mouth_radiation_scale=4.50`,
`mouth_radiation_area_factor=4.75`, `spl_offset_db=-5.75` y
`rear_radiation_scale=4.50` (factor de potencia).
