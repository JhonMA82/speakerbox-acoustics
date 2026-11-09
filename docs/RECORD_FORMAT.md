# Formato de registro de 734 bytes

La librería puede leer el registro binario de 734 bytes usado por la aplicación
de referencia. Esta es una función de compatibilidad para importar diseños
existentes; no implica que el motor use ese formato internamente.

## Registro por defecto

La aplicación de referencia guarda sus diseños en un registro binario de
734 bytes. El registro por defecto, volcado a texto, empieza así:

```text
0.5 x Pi    2.83   55.30    1.00   80.00  350.00   60.00    0.00
350.00 2400.00   75.00   70.27    0.00 ...
```

Los campos del registro son:

| Índice | Campo | Unidad | Valor por defecto |
|---:|---|---|---:|
| 0 | `Ang` | texto (`0.5 x Pi`) | π/2 |
| 1 | `Eg` | V | 2.83 |
| 2 | `Fta` | ° (ángulo tangente de boca) | 55.30 |
| 3 | `Cir` | — | 1.00 |
| 4 | `S1` | cm² | 80.00 |
| 5 | `S2` | cm² | 350.00 |
| 6 | `L12` | cm | 60.00 |
| 7 | `F12` | Hz | 0.00 |
| 8 | `S2` | cm² | 350.00 |
| 9 | `S3` | cm² | 2400.00 |
| 10 | `L23` | cm | 75.00 |
| 11 | `F23` | Hz | 70.27 |
| 12 | `S3` | cm² | 0.00 |
| 13 | `S4` | cm² | 0.00 |
| 14 | `L34` | cm | 0.00 |
| 15 | `F34` | Hz | 0.00 |
| 16 | `S4` | cm² | 0.00 |
| 17 | `S5` | cm² | 0.00 |
| 18 | `L45` | cm | 0.00 |
| 19 | `F45` | Hz | 0.00 |
| 20 | `Sd` | cm² | 350.00 |
| 21 | `Cms` | m/N | 4.00e-4 |
| 22 | `Mmd` | g | 20.00 |
| 23 | `Re` | Ω | 6.00 |
| 24 | `Bl` | T·m | 18.00 |
| 25 | `Rms` | kg/s | 4.00 |
| 26 | `Le` | mH | 1.00 |
| 27 | `Nd` | — | 1 |
| 28 | `Vrc` | L | 14.00 |
| 29 | `Fr` | mks rayls/m | 40000.00 |
| 30 | `Vtc` | cc | 900.00 |
| 31 | `Lrc` | cm | 16.00 |
| 32 | `Tal` | cm | 4.00 |
| 33 | `Atc` | cm² | 350.00 |
| 34 | `Pmax` | W | 100 |
| 35 | `Xmax` | mm | 5.0 |

* Los campos 0–391 son 49 campos de 8 caracteres.
* El comentario ocupa 40 bytes en el offset **642**.
* Los tipos de segmento están en los últimos 16 bytes: `ConExpL34L45111A`
  (segmento 1 cónico, segmento 2 exponencial, segmentos 3–4 vacíos).
* Los campos 392–641 contienen banderas y parámetros auxiliares empaquetados.

**Comprobación observada**: para el segmento exponencial,
`fc = c·ln(S3/S2)/(4π·L23) = 70.27 Hz`, exactamente el `F23` del registro. Esto fija la convención de longitudes en
cm y áreas en cm².

---
