# Segunda auditoría: energía y radiación

Base real: `5e4388363fa248f49abd2e4a164949dbfdaf7b0b`, rama
`fix/acoustic-physical-consistency`, árbol limpio. Nueva rama local:
`fix/acoustic-energy-consistency`. Se preservan todos los commits de la primera
auditoría; main no se mueve.

Se releen contratos y los cinco informes anteriores. Línea base recién
ejecutada, no tomada de esos informes: suite debug/release, producción,
comparación ampliada y cinco benchmarks. Registros en `energy-raw/base-*`.
Se confirman matriz exacta de área constante, matrices locales y PreparedDesign.

EC-001 confirmado: Horn resuelve una carga distinta de la utilizada por
System para reportar potencia. Matched_mouth tampoco define una función de
transferencia a un punto del campo lejano.

EC-002 confirmado: las potencias se deducían del campo estimado o multiplicaban
el flujo de boca por 2π/Ω. Ese escalado no pertenece a la carga de la red.
Las regresiones anteriores a cambios están en `energy-raw/regressions-before.log`.
No se cambia la calibración ni un archivo de referencia.
