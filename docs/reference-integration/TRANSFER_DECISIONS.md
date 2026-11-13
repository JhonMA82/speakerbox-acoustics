> Informe histórico de la integración en 3e9e15d. El estado posterior está en
> [COMPLETION_REPORT.md](COMPLETION_REPORT.md); las limitaciones de este informe
> no deben interpretarse como el estado actual de implementación.

# Decisiones
Implementar primero el modelo serie distribuido mediante ecuación de momento
con resistencia Rf, sin convertir densidad de relleno en resistividad.
Conservar shunt loss y activar Rf solo mediante configuración explícita.
Integrar su disipación Rf|U|²/S en las mismas etapas y balances existentes.
Implementar alineamiento sellado exacto, sin tablas empíricas ventiladas.
TL, semiinductancia integrada, optimizador y adaptador externo quedan pendientes;
no se consideran satisfechos por estas dos incorporaciones.
