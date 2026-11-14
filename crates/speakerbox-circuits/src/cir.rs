//! Parser y serializador de netlists `.CIR`.
//!
//! Formato aceptado, una declaración por línea:
//!
//! ```text
//! * comentario
//! V1 1 0 1          ; fuente ideal de 1 V entre el nodo 1 y tierra
//! R1 1 2 10         ; 10 Ω
//! L1 2 3 1m 0.5     ; 1 mH con 0.5 Ω de resistencia serie (campo opcional)
//! C1 3 0 5u         ; 5 µF
//! Z1 1 0 6.4 2.1    ; carga de 6.4 + j2.1 Ω (parte imaginaria opcional)
//! ```
//!
//! Reglas:
//!
//! - Nodo `0` o `gnd` es tierra. Los nodos numéricos conservan su número.
//! - Los nodos con nombre (`$N_001`, `out`) reciben identificadores a partir
//!   del mayor número visto.
//! - Sufijos SI: `f`, `p`, `n`, `u`, `m`, `k`, `meg`, `g`, con unidad
//!   opcional (`ohm`, `h`, `f`, `v`). `1e-3` también es válido.
//! - Se ignoran las directivas que empiezan por `.`.
//! - Un error lleva el número de línea y no detiene el análisis de las demás.
//!
//! El serializador emite solo lo que el parser puede volver a leer; una curva
//! medida o una fuente compleja se rechazan con
//! [`CircuitError::NotSerializable`] en vez de perderse en silencio.

use crate::error::{CircuitError, CircuitResult};
use crate::model::{Circuit, Component, LoadImpedance, NodeId, VoltageSource};
use crate::solver::MAX_UNKNOWNS;
use num_complex::Complex64;
use std::collections::HashMap;
use std::fmt;

/// Error de una línea de netlist.
#[derive(Clone, Debug, PartialEq)]
pub struct CirError {
    /// Línea 1-based.
    pub line: usize,
    /// Descripción del problema.
    pub message: String,
}

impl fmt::Display for CirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "línea {}: {}", self.line, self.message)
    }
}

impl std::error::Error for CirError {}

/// Naturaleza de una declaración del netlist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Resistor,
    Inductor,
    Capacitor,
    Voltage,
    Load,
}

/// Declaración ya tokenizada, antes de asignar identificadores de nodo.
struct RawEntry {
    line: usize,
    kind: Kind,
    a: String,
    b: String,
    value: f64,
    extra: Option<f64>,
}

/// Analiza un netlist. Devuelve todos los errores encontrados, si hay alguno.
///
/// # Errores
///
/// Un `Vec<CirError>` con una entrada por línea inválida. Un netlist sin
/// declaraciones también es un error.
pub fn parse(text: &str) -> Result<Circuit, Vec<CirError>> {
    let mut errors = Vec::new();
    let mut entries = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        match parse_line(raw, line) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }
    if entries.is_empty() && errors.is_empty() {
        errors.push(CirError {
            line: 1,
            message: "netlist vacío".to_string(),
        });
    }
    let mut next_named = entries
        .iter()
        .flat_map(|entry| [entry.a.as_str(), entry.b.as_str()])
        .filter_map(|label| label.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut named: HashMap<String, u32> = HashMap::new();
    let mut resolve = |label: &str, line: usize| -> Result<NodeId, CirError> {
        if label == "0" || label.eq_ignore_ascii_case("gnd") {
            return Ok(NodeId::GROUND);
        }
        let numeric_like = label
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '+' || c == '-');
        if numeric_like {
            return match label.parse::<u32>() {
                Ok(number) if (number as usize) < MAX_UNKNOWNS => Ok(NodeId(number)),
                _ => Err(CirError {
                    line,
                    message: format!(
                        "nodo numérico fuera del límite del solver ({}): {}",
                        MAX_UNKNOWNS,
                        clean_token(label)
                    ),
                }),
            };
        }
        if let Some(id) = named.get(label) {
            return Ok(NodeId(*id));
        }
        let id = u32::try_from(next_named)
            .ok()
            .filter(|id| (*id as usize) < MAX_UNKNOWNS)
            .ok_or_else(|| CirError {
                line,
                message: format!("demasiados nodos: el límite es {MAX_UNKNOWNS}"),
            })?;
        next_named += 1;
        named.insert(label.to_string(), id);
        Ok(NodeId(id))
    };

    let mut resolved: Vec<Option<(NodeId, NodeId)>> = Vec::with_capacity(entries.len());
    let mut max_node = 0_u32;
    for entry in &entries {
        let a = resolve(&entry.a, entry.line);
        let b = resolve(&entry.b, entry.line);
        match (a, b) {
            (Ok(a), Ok(b)) => {
                max_node = max_node.max(a.0).max(b.0);
                resolved.push(Some((a, b)));
            }
            (Err(error), _) | (_, Err(error)) => {
                errors.push(error);
                resolved.push(None);
            }
        }
    }

    let mut circuit = Circuit::new(max_node as usize + 1);
    for (entry, nodes) in entries.iter().zip(resolved) {
        let Some((a, b)) = nodes else {
            continue;
        };
        let nodes = (a, b);
        let result = match entry.kind {
            Kind::Resistor => {
                circuit.push_component(Component::resistor(nodes.0, nodes.1, entry.value))
            }
            Kind::Inductor => circuit.push_component(Component::inductor(
                nodes.0,
                nodes.1,
                entry.value,
                entry.extra.unwrap_or(0.0),
            )),
            Kind::Capacitor => {
                circuit.push_component(Component::capacitor(nodes.0, nodes.1, entry.value))
            }
            Kind::Voltage => circuit.push_source(VoltageSource::new(
                nodes.0,
                nodes.1,
                Complex64::new(entry.value, 0.0),
            )),
            Kind::Load => {
                let impedance = Complex64::new(entry.value, entry.extra.unwrap_or(0.0));
                circuit.push_component(Component::load(nodes.0, nodes.1, impedance))
            }
        };
        if let Err(error) = result {
            errors.push(CirError {
                line: entry.line,
                message: error.to_string(),
            });
        }
    }
    if errors.is_empty() {
        Ok(circuit)
    } else {
        Err(errors)
    }
}

fn parse_line(raw: &str, line: usize) -> Result<Option<RawEntry>, CirError> {
    let content = raw.split([';', '#']).next().unwrap_or("").trim();
    if content.is_empty() || content.starts_with('*') || content.starts_with('.') {
        return Ok(None);
    }
    let tokens: Vec<&str> = content.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(CirError {
            line,
            message: "se esperaba: nombre, nodo A, nodo B y valor".to_string(),
        });
    }
    let name = tokens[0];
    let kind = match name.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('R') => Kind::Resistor,
        Some('L') => Kind::Inductor,
        Some('C') => Kind::Capacitor,
        Some('V') => Kind::Voltage,
        Some('Z') => Kind::Load,
        Some('I') => {
            return Err(CirError {
                line,
                message: "las fuentes de corriente no están soportadas".to_string(),
            })
        }
        _ => {
            return Err(CirError {
                line,
                message: format!("tipo de componente desconocido: {}", clean_token(name)),
            })
        }
    };
    let mut value_index = 3;
    if kind == Kind::Voltage
        && tokens.get(3).is_some_and(|token| {
            token.eq_ignore_ascii_case("ac") || token.eq_ignore_ascii_case("dc")
        })
    {
        value_index = 4;
    }
    let value_token = tokens.get(value_index).ok_or_else(|| CirError {
        line,
        message: "falta el valor".to_string(),
    })?;
    let value = parse_si_value(value_token).map_err(|message| CirError { line, message })?;
    let expected = value_index + 1;
    let extra = match tokens.get(expected) {
        Some(token) if matches!(kind, Kind::Inductor | Kind::Load) => {
            Some(parse_si_value(token).map_err(|message| CirError { line, message })?)
        }
        Some(token) => {
            return Err(CirError {
                line,
                message: format!("token inesperado: {}", clean_token(token)),
            })
        }
        None => None,
    };
    if tokens.len() > expected + usize::from(extra.is_some()) {
        return Err(CirError {
            line,
            message: format!(
                "tokens sobrantes: {}",
                clean_token(&tokens[expected + 1..].join(" "))
            ),
        });
    }
    Ok(Some(RawEntry {
        line,
        kind,
        a: tokens[1].to_string(),
        b: tokens[2].to_string(),
        value,
        extra,
    }))
}

/// Convierte un valor con sufijo SI (`10k`, `5uF`, `1e-3`, `2.2mH`).
///
/// Acepta valores negativos finitos: el formato los necesita para fuentes con
/// polaridad invertida y para la parte imaginaria capacitiva de una carga. Son
/// los constructores de componentes quienes rechazan un R, L o C negativos.
///
/// # Errores
///
/// Texto que no empieza por un número válido, sufijo no reconocido o valor no
/// finito.
pub fn parse_si_value(token: &str) -> Result<f64, String> {
    let token = token.trim();
    let (number, suffix) =
        split_number(token).ok_or_else(|| format!("valor no numérico: {}", clean_token(token)))?;
    let suffix = suffix.trim().to_ascii_lowercase().replace('µ', "u");
    let (factor, unit) = split_multiplier(&suffix)
        .ok_or_else(|| format!("sufijo no reconocido en {}", clean_token(token)))?;
    if !matches!(unit, "" | "ohm" | "r" | "h" | "f" | "v" | "a" | "hz" | "s") {
        return Err(format!(
            "unidad no reconocida en {}: {}",
            clean_token(token),
            clean_token(unit)
        ));
    }
    let value = number * factor;
    if !value.is_finite() {
        return Err(format!("valor fuera de rango: {}", clean_token(token)));
    }
    Ok(value)
}

/// Sanea un token de entrada para un mensaje: sin caracteres de control y con
/// longitud acotada, para que un fichero hostil no inyecte secuencias ANSI en
/// el terminal o el log de quien muestre el error.
fn clean_token(token: &str) -> String {
    token
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .take(80)
        .collect()
}

fn split_number(token: &str) -> Option<(f64, &str)> {
    let bytes = token.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }
    let mut digits = false;
    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' => {
                digits = true;
                index += 1;
            }
            b'.' => index += 1,
            b'e' | b'E' if digits => {
                let mut lookahead = index + 1;
                if matches!(bytes.get(lookahead), Some(b'+' | b'-')) {
                    lookahead += 1;
                }
                if bytes.get(lookahead).is_some_and(u8::is_ascii_digit) {
                    index = lookahead;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if !digits {
        return None;
    }
    let number = token[..index].parse().ok()?;
    Some((number, &token[index..]))
}

fn split_multiplier(suffix: &str) -> Option<(f64, &str)> {
    for (prefix, factor) in [
        ("meg", 1e6),
        ("g", 1e9),
        ("k", 1e3),
        ("m", 1e-3),
        ("u", 1e-6),
        ("n", 1e-9),
        ("p", 1e-12),
        ("f", 1e-15),
    ] {
        if let Some(rest) = suffix.strip_prefix(prefix) {
            return Some((factor, rest));
        }
    }
    Some((1.0, suffix))
}

/// Serializa el circuito como netlist que [`parse`] puede volver a leer.
///
/// # Errores
///
/// [`CircuitError::NotSerializable`] para fuentes con parte imaginaria o
/// cargas medidas, que el formato no representa.
pub fn to_cir(circuit: &Circuit) -> CircuitResult<String> {
    let mut out = String::from("* speakerbox-circuits netlist (SI)\n");
    for (index, source) in circuit.sources().iter().enumerate() {
        if source.volts.im != 0.0 {
            return Err(CircuitError::NotSerializable {
                index,
                reason: "la fuente tiene parte imaginaria",
            });
        }
        out.push_str(&format!(
            "V{} {} {} {}\n",
            index + 1,
            source.positive.0,
            source.negative.0,
            source.volts.re
        ));
    }
    for (index, component) in circuit.components().iter().enumerate() {
        let (a, b) = (component.a.0, component.b.0);
        match &component.kind {
            crate::model::ComponentKind::Resistor { ohm } => {
                out.push_str(&format!("R{} {a} {b} {ohm}\n", index + 1));
            }
            crate::model::ComponentKind::Inductor { henry, dcr_ohm } => {
                out.push_str(&format!("L{} {a} {b} {henry} {dcr_ohm}\n", index + 1));
            }
            crate::model::ComponentKind::Capacitor { farad } => {
                out.push_str(&format!("C{} {a} {b} {farad}\n", index + 1));
            }
            crate::model::ComponentKind::Load(LoadImpedance::Constant(z)) => {
                if z.im == 0.0 {
                    out.push_str(&format!("Z{} {a} {b} {}\n", index + 1, z.re));
                } else {
                    out.push_str(&format!("Z{} {a} {b} {} {}\n", index + 1, z.re, z.im));
                }
            }
            crate::model::ComponentKind::Load(LoadImpedance::Curve(_)) => {
                return Err(CircuitError::NotSerializable {
                    index,
                    reason: "una carga medida no se serializa",
                })
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CircuitError;

    const RLC: &str = "\
* red serie del fixture
V1 1 0 1
R1 1 2 10
L1 2 3 1m 0
C1 3 0 5u
";

    #[test]
    fn parses_si_suffixes() {
        assert!((parse_si_value("10k").unwrap() - 10_000.0).abs() < 1e-12);
        assert!((parse_si_value("5uF").unwrap() - 5e-6).abs() < 1e-18);
        assert!((parse_si_value("2.2mH").unwrap() - 2.2e-3).abs() < 1e-15);
        assert!((parse_si_value("1meg").unwrap() - 1e6).abs() < 1e-6);
        assert!((parse_si_value("1e-3").unwrap() - 1e-3).abs() < 1e-18);
        assert!((parse_si_value("100ohm").unwrap() - 100.0).abs() < 1e-12);
        assert!((parse_si_value("4.7µF").unwrap() - 4.7e-6).abs() < 1e-18);
        assert!(parse_si_value("abc").is_err());
        assert!(parse_si_value("10x").is_err());
        assert!((parse_si_value("-5").unwrap() + 5.0).abs() < 1e-12);
        assert!(parse_si_value("1e").is_err());
    }

    #[test]
    fn parses_the_series_fixture() {
        let circuit = parse(RLC).unwrap();
        assert_eq!(circuit.nodes(), 4);
        assert_eq!(circuit.sources().len(), 1);
        assert_eq!(circuit.components().len(), 3);
        assert_eq!(circuit.sources()[0].volts, Complex64::new(1.0, 0.0));
    }

    #[test]
    fn understands_ac_sources_and_comments() {
        let circuit = parse("V1 1 0 AC 2 ; 2 V\nR1 1 0 4 # carga\n").unwrap();
        assert_eq!(circuit.sources()[0].volts, Complex64::new(2.0, 0.0));
    }

    #[test]
    fn maps_named_nodes_above_numeric_ones() {
        let circuit = parse("V1 in 0 1\nR1 in out 10\nR2 out 0 10\n").unwrap();
        assert!(circuit.nodes() >= 3);
    }

    #[test]
    fn reports_every_bad_line_with_its_number() {
        let errors = parse("V1 1 0 1\nX1 1 0 1\nR1 1 0 -5\n").unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[1].line, 3);
    }

    #[test]
    fn rejects_current_sources_explicitly() {
        let errors = parse("I1 1 0 1\n").unwrap_err();
        assert!(errors[0].message.contains("corriente"));
    }

    #[test]
    fn keeps_solver_errors_at_the_right_line() {
        let errors = parse("V1 1 0 1\nR1 1 1 10\n").unwrap_err();
        assert_eq!(errors[0].line, 2);
    }

    #[test]
    fn negative_source_and_reactive_load_round_trip() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(-1.0, 0.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::load(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(8.0, -4.0),
            ))
            .unwrap();
        let text = to_cir(&circuit).unwrap();
        assert_eq!(parse(text.as_str()).unwrap(), circuit);
    }

    #[test]
    fn node_ids_are_capped_with_a_clear_error() {
        for label in ["512", "4096", "4294967295", "-1", "18446744073709551616"] {
            let text = format!("V1 1 0 1\nR1 {label} 0 10\n");
            let errors = parse(&text).unwrap_err();
            assert!(
                errors[0].message.contains("nodo numérico fuera"),
                "{label}: {}",
                errors[0].message
            );
        }
        let ok = parse("V1 1 0 1\nR1 511 0 10\n").unwrap();
        assert_eq!(ok.nodes(), 512);
    }

    #[test]
    fn control_characters_never_reach_the_error_text() {
        let errors = parse("R1 1 0 \u{1b}[2J\n").unwrap_err();
        assert!(!errors[0].message.chars().any(char::is_control));
        assert!(errors[0].message.contains("?[2J"));
    }

    #[test]
    fn round_trip_preserves_the_circuit() {
        let original = parse(RLC).unwrap();
        let text = to_cir(&original).unwrap();
        let reparsed = parse(&text).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn refuses_to_serialize_what_the_format_cannot_read() {
        let mut circuit = Circuit::new(2);
        circuit
            .push_source(VoltageSource::new(
                NodeId(1),
                NodeId::GROUND,
                Complex64::new(0.0, 1.0),
            ))
            .unwrap();
        circuit
            .push_component(Component::resistor(NodeId(1), NodeId::GROUND, 10.0))
            .unwrap();
        assert_eq!(
            to_cir(&circuit).unwrap_err(),
            CircuitError::NotSerializable {
                index: 0,
                reason: "la fuente tiene parte imaginaria"
            }
        );
    }
}
