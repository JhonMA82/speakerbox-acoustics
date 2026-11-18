//! Esquema JSON de geometría acústica. Excitación y aire son entradas al solver.
use serde_json::{json, Value};

use super::{
    topology::NetworkLayout, AcousticBoundary, AcousticDuct, AcousticNetwork, AcousticSource,
    Boundary, Conduit, NetworkError,
};
use crate::{Chamber, EndCorrection, Flare, Port, Segment};

const FORMAT: NetworkError = NetworkError::Invalid("esquema JSON de red v1");

impl NetworkLayout {
    /// Serializa una red geométrica v1 en SI. Las closures arbitrarias, los
    /// revestimientos y las opciones termoviscosas no tienen representación v1.
    pub fn to_json(&self) -> Result<String, NetworkError> {
        self.validate_structure()?;
        let ducts = self
            .network
            .ducts
            .iter()
            .map(encode_duct)
            .collect::<Result<Vec<_>, _>>()?;
        let boundaries = self
            .network
            .boundaries
            .iter()
            .map(encode_boundary)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(
            json!({"version":1,"nodes":self.network.nodes,"front":self.front,
            "rear":self.rear,"ducts":ducts,"boundaries":boundaries})
            .to_string(),
        )
    }

    /// Lee geometría nominal de la red, verificando referencias y campos.
    pub fn from_json(text: &str) -> Result<Self, NetworkError> {
        let v: Value = serde_json::from_str(text).map_err(|_| FORMAT)?;
        if v["version"].as_u64() != Some(1) {
            return Err(FORMAT);
        }
        fields(
            &v,
            &["version", "nodes", "front", "rear", "ducts", "boundaries"],
        )?;
        let layout = Self {
            front: node(&v["front"])?,
            rear: node(&v["rear"])?,
            network: AcousticNetwork {
                nodes: node(&v["nodes"])?,
                ducts: v["ducts"]
                    .as_array()
                    .ok_or(FORMAT)?
                    .iter()
                    .map(decode_duct)
                    .collect::<Result<_, _>>()?,
                boundaries: v["boundaries"]
                    .as_array()
                    .ok_or(FORMAT)?
                    .iter()
                    .map(decode_boundary)
                    .collect::<Result<_, _>>()?,
            },
        };
        layout.validate_structure()?;
        Ok(layout)
    }

    fn validate_structure(&self) -> Result<(), NetworkError> {
        if self.front == self.rear {
            return Err(NetworkError::Invalid(
                "caras del diafragma en el mismo nodo",
            ));
        }
        self.network
            .check_connectivity(&[self.front, self.rear].map(|node| AcousticSource {
                node,
                flow_m3_s: 0.0.into(),
            }))?;
        for b in &self.network.boundaries {
            match b.boundary {
                Boundary::Chamber(ch) => ch.validate()?,
                Boundary::Opening {
                    area_m2,
                    load_scale,
                } => {
                    crate::error::require_positive(area_m2, "network.opening.area")?;
                    crate::error::require_positive(load_scale, "network.opening.scale")?;
                }
                Boundary::Resistance { pa_s_m3 } => {
                    crate::error::require_positive(pa_s_m3, "network.resistance")?
                }
                Boundary::PressureRelease => (),
            }
        }
        Ok(())
    }
}

fn encode_duct(duct: &AcousticDuct) -> Result<Value, NetworkError> {
    let value = match &duct.conduit {
        Conduit::Distributed(s) => {
            s.validate()?;
            if s.thermoviscous_properties().is_some() {
                return Err(NetworkError::Invalid(
                    "segmento termoviscoso sin serialización v1",
                ));
            }
            let (name, t) = match s.flare() {
                Flare::Cylindrical => ("cylindrical", None),
                Flare::Conical => ("conical", None),
                Flare::Exponential => ("exponential", None),
                Flare::Parabolic => ("linear_area", None),
                Flare::HyperbolicExponential { t } => ("hypex", Some(*t)),
                Flare::Custom(_) => {
                    return Err(NetworkError::Invalid(
                        "conserve la especificación del perfil Custom por separado",
                    ))
                }
            };
            json!({"kind":"distributed","flare":name,"t":t,"length_m":s.length(),"inlet_area_m2":s.s1(),
                "outlet_area_m2":s.s2(),"wall_loss":s.loss,"resistivity_pa_s_m2":s.flow_resistivity()})
        }
        Conduit::Inertance(p) => {
            p.validate()?;
            let r = (p.area / std::f64::consts::PI).sqrt();
            json!({"kind":"inertance","length_m":p.length,"area_m2":p.area,
                "end_correction_radii":p.end_correction.delta(p.area)/r})
        }
    };
    Ok(json!({"inlet":duct.inlet,"outlet":duct.outlet,"model":value}))
}

fn decode_duct(v: &Value) -> Result<AcousticDuct, NetworkError> {
    fields(v, &["inlet", "outlet", "model"])?;
    let m = &v["model"];
    let conduit = match m["kind"].as_str() {
        Some("distributed") => {
            fields(
                m,
                &[
                    "kind",
                    "flare",
                    "t",
                    "length_m",
                    "inlet_area_m2",
                    "outlet_area_m2",
                    "wall_loss",
                    "resistivity_pa_s_m2",
                ],
            )?;
            let flare = match m["flare"].as_str() {
                Some("cylindrical") if m["t"].is_null() => Flare::Cylindrical,
                Some("conical") if m["t"].is_null() => Flare::Conical,
                Some("exponential") if m["t"].is_null() => Flare::Exponential,
                Some("linear_area") if m["t"].is_null() => Flare::Parabolic,
                Some("hypex") => Flare::HyperbolicExponential { t: number(m, "t")? },
                _ => return Err(FORMAT),
            };
            let mut s = Segment::new(
                flare,
                number(m, "length_m")?,
                number(m, "inlet_area_m2")?,
                number(m, "outlet_area_m2")?,
            )
            .with_flow_resistivity(number(m, "resistivity_pa_s_m2")?);
            s.loss = number(m, "wall_loss")?;
            s.validate()?;
            Conduit::Distributed(s)
        }
        Some("inertance") => {
            fields(m, &["kind", "area_m2", "length_m", "end_correction_radii"])?;
            let p = Port::new(
                number(m, "area_m2")?,
                number(m, "length_m")?,
                EndCorrection::Inner(number(m, "end_correction_radii")?),
            );
            p.validate()?;
            Conduit::Inertance(p)
        }
        _ => return Err(FORMAT),
    };
    Ok(AcousticDuct {
        inlet: node(&v["inlet"])?,
        outlet: node(&v["outlet"])?,
        conduit,
    })
}

fn encode_boundary(b: &AcousticBoundary) -> Result<Value, NetworkError> {
    let value = match b.boundary {
        Boundary::Chamber(c) if c.lining.is_none() => {
            json!({"kind":"compliance","volume_m3":c.volume})
        }
        Boundary::Chamber(_) => {
            return Err(NetworkError::Invalid("recubrimiento sin serialización v1"))
        }
        Boundary::Opening {
            area_m2,
            load_scale,
        } => json!({"kind":"opening","area_m2":area_m2,"load_scale":load_scale}),
        Boundary::Resistance { pa_s_m3 } => json!({"kind":"resistance","pa_s_m3":pa_s_m3}),
        Boundary::PressureRelease => json!({"kind":"pressure_release"}),
    };
    Ok(json!({"node":b.node,"model":value}))
}

fn decode_boundary(v: &Value) -> Result<AcousticBoundary, NetworkError> {
    fields(v, &["node", "model"])?;
    let m = &v["model"];
    let boundary = match m["kind"].as_str() {
        Some("compliance") => {
            fields(m, &["kind", "volume_m3"])?;
            Boundary::Chamber(Chamber::new(number(m, "volume_m3")?))
        }
        Some("opening") => {
            fields(m, &["kind", "area_m2", "load_scale"])?;
            Boundary::Opening {
                area_m2: number(m, "area_m2")?,
                load_scale: number(m, "load_scale")?,
            }
        }
        Some("resistance") => {
            fields(m, &["kind", "pa_s_m3"])?;
            Boundary::Resistance {
                pa_s_m3: number(m, "pa_s_m3")?,
            }
        }
        Some("pressure_release") => {
            fields(m, &["kind"])?;
            Boundary::PressureRelease
        }
        _ => return Err(FORMAT),
    };
    Ok(AcousticBoundary {
        node: node(&v["node"])?,
        boundary,
    })
}

fn fields(v: &Value, keys: &[&str]) -> Result<(), NetworkError> {
    let data = v.as_object().ok_or(FORMAT)?;
    if data.len() != keys.len() || !keys.iter().all(|k| data.contains_key(*k)) {
        return Err(FORMAT);
    }
    Ok(())
}

fn number(v: &Value, key: &str) -> Result<f64, NetworkError> {
    v[key].as_f64().filter(|n| n.is_finite()).ok_or(FORMAT)
}

fn node(v: &Value) -> Result<usize, NetworkError> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(FORMAT)
}
