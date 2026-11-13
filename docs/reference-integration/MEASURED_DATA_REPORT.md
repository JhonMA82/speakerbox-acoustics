# M5 — FRD/ZMA independiente

`measured_data::Measurement::parse` importa dos/tres columnas con comentarios,
encabezados previos, espacio/tab/coma/punto y coma. Decimal: punto. Las unidades
se especifican explícitamente mediante escala de frecuencia y magnitud;
FRD dB SPL referencia 20 µPa; presión lineal y ZMA ohmios también admitidos.
No se interpretan encabezados ambiguos como una referencia de medición válida.
Frecuencias duplicadas/desordenadas, filas incompletas, no finitos, columnas
extra y dominios inválidos devuelven error con línea. Fase ausente queda None.
Procedencia, distancia, tensión y referencia temporal son metadatos opcionales.
No hay sustitución automática de la física simulada por mediciones.

Interpolación log-frecuencia: impedancia en plano complejo cartesiano si hay
fase; FRD magnitud/dB y fase local desenrollada, conservando fases continuas
explícitas fuera de ±180°. Si falta fase no se inventa. Cancelación compleja
exacta tiene fase None. Fuera del rango devuelve error, no extrapolación.

Tres pruebas con fixtures sintéticos propios: separadores, comentarios,
encabezados, fase envuelta/continua, datos faltantes, errores y unidades,
impedancia compleja interpolada. No se han copiado mediciones Wavecor ni se
presentan estos fixtures como experimentos de laboratorio.
