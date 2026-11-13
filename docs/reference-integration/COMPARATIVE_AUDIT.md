# Comparación inicial (antes de implementar)
Base principal 72f4e61, dos auditorías, árbol limpio. Fuente inspeccionada:
a0e128dfb6c21f68599d74584d4d6af2ec418820 (HEAD del clon).

| Capacidad | Referencia | Principal | Decisión |
|---|---|---|---|
| TMM | entrada=M salida, B/C positivos j | salida=M entrada, B/C negativos j | REFERENCIA: invertir orientación, no copiar signos |
| Radiación | serie reactiva empieza 2x/(3π) | 4x/(3π), validado | DESCARTAR: referencia tiene mitad del límite correcto |
| Pérdidas | densidad efectiva ρ+Rf/(jω), k/Z complejos | conductancia shunt y balance RMS | ADAPTAR: resistencia distribuida serie, derivación independiente |
| Semiinductancia | Ke sustituye Le, √s | Le, balances con Re explícita | REFERENCIA: añadir Ke sin contabilizar sus pérdidas rompería balance; no transferir parcialmente |
| Alineamientos sellados | Bessel f3=1.55fc orientativo | sin generador | ADAPTAR: raíz exacta del filtro de segundo orden |
| TL | stub cerrado paralelo, tubo segmentado, cierre 1e12 | OffsetHorn ya comparte esa red | ADAPTAR pendiente: no duplicar TMM, cierre exacto y validación experimental necesarios |
| Optimización | filtros/gain/delay de crossover | diseño de cajas | REFERENCIA: no equivale a optimización geométrica |
| Fixtures FRD/ZMA | encabezado “measured”, procedencia no acreditada | oráculos propios | NO IMPORTAR sin permisos/procedencia |
| Geometrías | aproximaciones de área | perfiles ya comprobados | DESCARTAR sustitución |

README.md declara MIT en su sección License. No hay LICENSE/COPYING/NOTICE
ni licencia en solver/Cargo.toml. No se acreditó procedencia específica de
fixtures; no se copió código ni datos, y se conserva esta distinción.
Los comentarios que citan publicaciones no constituyen licencia del software.
La reactancia se verifica algebraicamente: con x=2ka, H1(x)~2x²/(3π),
por tanto 2H1(x)/x~4x/(3π). La masa pura de pliegue no es disipación.
