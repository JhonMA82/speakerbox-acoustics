use std::f64::consts::TAU;

use num_complex::Complex64 as C;
use serde_json::Value;
use speakerbox_acoustics::{
    energy::active_power, Air, Chamber, Driver, Flare, FrontAdapter, HornProfileSpec, Segment,
    System, ThermoviscousAir, TransmissionLine,
};

const FIXTURE: &str = include_str!("data/horn-simulation/webster.json");
const REVISION: &str = "7dd9852aec8f60e1b35ae637ad28bd614efc2c1b";
const GRID: [f64; 9] = [25., 41., 69., 98., 147., 222., 331., 479., 630.];
const CASES: [(&str, &str, &str, &str); 11] = [
    ("tube", "cylindrical", "flanged_piston", "lossless"),
    ("cone", "conical", "flanged_piston", "lossless"),
    ("exponential", "exponential", "flanged_piston", "lossless"),
    ("hypex", "hypex", "flanged_piston", "lossless"),
    ("tractrix", "tractrix", "flanged_piston", "lossless"),
    ("tractrix_full", "tractrix", "flanged_piston", "lossless"),
    (
        "scaled_tractrix",
        "scaled_tractrix",
        "flanged_piston",
        "lossless",
    ),
    ("os", "oblate_spheroidal", "flanged_piston", "lossless"),
    (
        "compound",
        "exponential_conical",
        "flanged_piston",
        "lossless",
    ),
    ("tube_matched", "cylindrical", "plane_wave", "lossless"),
    (
        "tube_thermoviscous",
        "cylindrical",
        "flanged_piston",
        "boundary_layer",
    ),
];

fn fixture() -> String {
    std::env::var_os("HORN_SIMULATION_FIXTURE")
        .map(|path| std::fs::read_to_string(path).unwrap())
        .unwrap_or_else(|| FIXTURE.to_owned())
}

fn number(v: &Value) -> f64 {
    v.as_f64().filter(|x| x.is_finite()).expect("finite scalar")
}

fn complex(v: &Value) -> C {
    assert_eq!(v.as_array().unwrap().len(), 2);
    C::new(number(&v[0]), number(&v[1]))
}

fn manifest(data: &Value) -> Result<(), &'static str> {
    if data["schema_version"] != 1
        || data["source_revision"] != REVISION
        || data["phasor"] != "exp(+iwt) RMS"
        || data["drive"] != "throat pressure 1 Pa RMS"
        || data["impedance_unit"] != "Pa s/m^3"
        || data["c_m_s"] != 343.0
        || data["rho_kg_m3"] != 1.225
        || data["p_ref_pa"] != 20e-6
        || data["slices"] != 4096
    {
        return Err("incompatible solver metadata");
    }
    let thermo = ThermoviscousAir::default();
    if data["thermophysical_air"]
        != serde_json::json!({
            "viscosity_pa_s": thermo.viscosity_pa_s,
            "thermal_conductivity_w_m_k": thermo.thermal_conductivity_w_m_k,
            "heat_capacity_j_kg_k": thermo.heat_capacity_j_kg_k,
            "heat_capacity_ratio": thermo.heat_capacity_ratio,
        })
    {
        return Err("incompatible thermophysical air");
    }
    if data["frequency_hz"] != serde_json::json!(GRID) {
        return Err("incompatible frequency grid");
    }
    let cases = data["cases"].as_array().ok_or("no cases")?;
    if cases.len() != CASES.len() {
        return Err("missing or additional cases");
    }
    for (case, (name, kind, boundary, losses)) in cases.iter().zip(CASES) {
        if case["name"] != name
            || case["profile"]["kind"] != kind
            || case["boundary"] != boundary
            || case["losses"] != losses
        {
            return Err("incompatible profile, loss model or boundary");
        }
        validate_columns(case, &["z_in", "p_mouth", "u_mouth"], &["net_loss_w"])?;
        if kind == "hypex" && case["profile"]["t"] != std::f64::consts::FRAC_1_SQRT_2 {
            return Err("incompatible Hypex parameter");
        }
        let profile = HornProfileSpec::from_json(&case["profile"].to_string())
            .map_err(|_| "invalid geometry")?;
        if profile.throat_radius_m != 0.025
            || profile.mouth_radius_m != if kind == "cylindrical" { 0.025 } else { 0.12 }
        {
            return Err("incompatible physical section areas");
        }
        let length = match name {
            "tube" | "tube_matched" | "tube_thermoviscous" => 0.27,
            "tractrix_full" => HornProfileSpec::natural_tractrix_length(0.025, 0.12)
                .map_err(|_| "incompatible tractrix length")?,
            "scaled_tractrix" => 0.19,
            _ => 0.30,
        };
        if (profile.length_m - length).abs() > 1e-15 {
            return Err("incompatible length");
        }
    }
    let coupling = data["coupling"].as_array().ok_or("no coupling cases")?;
    if coupling.len() != 3 {
        return Err("missing coupling cases");
    }
    for (case, kind) in coupling.iter().zip(["ideal", "lumped", "duct"]) {
        if case["kind"] != kind {
            return Err("incompatible front adapter");
        }
        validate_columns(
            case,
            &[
                "ze",
                "front_z",
                "velocity",
                "throat_flow",
                "throat_pressure",
                "mouth_flow",
            ],
            &[
                "spl_db",
                "phase_deg",
                "xd_mm",
                "p_electric_w",
                "p_mechanical_w",
                "p_acoustic_w",
                "p_radiated_w",
            ],
        )?;
    }
    validate_columns(
        &data["exact_kirchhoff_tube"],
        &["a", "b", "c"],
        &["attenuation_np"],
    )?;
    Ok(())
}

fn validate_columns(data: &Value, vector: &[&str], scalar: &[&str]) -> Result<(), &'static str> {
    let finite = |v: &Value| v.as_f64().is_some_and(f64::is_finite);
    for (fields, complex) in [(vector, true), (scalar, false)] {
        for &name in fields {
            let rows = data[name].as_array().ok_or("missing value column")?;
            if rows.len() != GRID.len()
                || rows.iter().any(|v| {
                    if complex {
                        !v.as_array()
                            .is_some_and(|p| p.len() == 2 && p.iter().all(finite))
                    } else {
                        !finite(v)
                    }
                })
            {
                return Err("incomplete or invalid column");
            }
        }
    }
    Ok(())
}

#[derive(Default, Debug)]
struct Errors {
    max_db: f64,
    max_deg: f64,
}

impl Errors {
    fn check(&mut self, rust: C, reference: C) -> Result<(), &'static str> {
        let ratio = rust / reference;
        let db = (20.0 * ratio.norm().log10()).abs();
        let deg = ratio.arg().to_degrees().abs();
        if !db.is_finite() || !deg.is_finite() || db > 0.015 || deg > 0.1 {
            eprintln!("rust={rust} reference={reference} db={db} deg={deg}");
            return Err("incompatible magnitude or phase");
        }
        self.max_db = self.max_db.max(db);
        self.max_deg = self.max_deg.max(deg);
        Ok(())
    }
}

fn check_scalar(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}, tolerance={tolerance:.12e}"
    );
}

fn compare_webster(data: &Value) -> Result<(), &'static str> {
    manifest(data)?;
    let air = Air::new(number(&data["c_m_s"]), number(&data["rho_kg_m3"]));
    for case in data["cases"].as_array().unwrap() {
        let spec = HornProfileSpec::from_json(&case["profile"].to_string()).unwrap();
        let mut horn = spec.resolve().unwrap().into_horn();
        horn.matched_mouth = case["boundary"] == "plane_wave";
        if case["losses"] == "boundary_layer" {
            horn.segments[0] = horn.segments[0]
                .clone()
                .with_thermoviscous_losses(ThermoviscousAir::default());
        }
        let mut impedance = Errors::default();
        let mut pressure = Errors::default();
        let mut flow = Errors::default();
        let mut loss_error: f64 = 0.0;
        for (i, f) in GRID.into_iter().enumerate() {
            let z = horn.input_impedance(&air, f, TAU);
            let p_in = C::from(1.0);
            let u_in = p_in / z;
            let (p, u, loss) =
                horn.segments
                    .iter()
                    .try_fold((p_in, u_in, 0.0), |(p, u, loss), s| {
                        let loss = loss
                            + s.dissipated_power_w(&air, f, p, u)
                                .map_err(|_| "Rust dissipated power")?;
                        let (p, u) = s
                            .try_matrix(&air, f)
                            .map_err(|_| "Rust transfer matrix")?
                            .apply(p, u);
                        Ok((p, u, loss))
                    })?;
            impedance.check(z, complex(&case["z_in"][i]))?;
            pressure.check(p, complex(&case["p_mouth"][i]))?;
            flow.check(u, complex(&case["u_mouth"][i]))?;
            let input_power = active_power(p_in, u_in);
            let power_residual = input_power - active_power(p, u) - loss;
            assert!(input_power > 0.0 && loss >= 0.0);
            check_scalar(power_residual, 0.0, input_power * 1e-5);
            let expected_loss = number(&case["net_loss_w"][i]);
            let relative = (loss - expected_loss).abs() / input_power;
            assert!(relative < 1e-5);
            loss_error = loss_error.max(relative);
        }
        println!(
            "{} | Z {impedance:?} | p {pressure:?} | U {flow:?} | loss/input {loss_error:.3e}",
            case["name"]
        );
    }
    Ok(())
}

#[test]
fn webster_matches_independent_tmm() {
    compare_webster(&serde_json::from_str(&fixture()).unwrap()).unwrap();
}

#[test]
fn fixture_gate_reports_incompatible_and_incorrect_data() {
    let valid: Value = serde_json::from_str(FIXTURE).unwrap();
    for (field, value) in [
        ("cases", serde_json::json!([])),
        ("frequency_hz", serde_json::json!([25., 41.])),
        ("impedance_unit", serde_json::json!("Pa s/m")),
        ("phasor", serde_json::json!("exp(-iwt) peak")),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert!(compare_webster(&bad).is_err(), "{field}");
    }
    let mut bad = valid.clone();
    bad["cases"][0]["boundary"] = "closed".into();
    assert!(compare_webster(&bad).is_err());
    let mut bad = valid.clone();
    bad["cases"][0]["z_in"][0] = serde_json::json!([0.0, 0.0]);
    assert_eq!(
        compare_webster(&bad),
        Err("incompatible magnitude or phase")
    );
    let mut bad = valid;
    bad["cases"][0]["p_mouth"][0] = Value::Null;
    assert!(compare_webster(&bad).is_err());
}

fn motor(d: &Value) -> Driver {
    let value = |name| number(&d[name]);
    Driver::new(
        value("sd_m2"),
        value("cms_m_per_n"),
        value("moving_mass_kg"),
        value("re_ohm"),
        value("bl_tm"),
        value("rms_kg_per_s"),
        value("le_h"),
    )
}

#[test]
fn coupled_horn_matches_motor_and_adapter_oracle() {
    let data: Value = serde_json::from_str(&fixture()).unwrap();
    manifest(&data).unwrap();
    let air = Air::new(number(&data["c_m_s"]), number(&data["rho_kg_m3"]));
    let d = &data["driver"];
    let mut base = System::new(air, motor(d));
    base.horn = Some(
        HornProfileSpec::from_json(&data["cases"][1]["profile"].to_string())
            .unwrap()
            .resolve()
            .unwrap()
            .into_horn(),
    );
    base.rear_chamber = Some(Chamber::new(number(&d["rear_chamber_volume_m3"])));
    base.eg = number(&d["voltage_rms"]);
    let adapter = FrontAdapter {
        area_m2: 0.0045,
        volume_m3: 0.00018,
    };
    let mut conflict = base.clone();
    conflict.throat_chamber = Some(Chamber::new(adapter.volume_m3));
    assert!(conflict.with_front_adapter(adapter).is_err());
    assert!(base
        .clone()
        .with_front_adapter(FrontAdapter {
            volume_m3: f64::NAN,
            ..adapter
        })
        .is_err());

    for case in data["coupling"].as_array().unwrap() {
        let configuration = case["adapter"].clone();
        let mut system = base.clone();
        if case["kind"] == "duct" {
            system = system
                .with_front_adapter(FrontAdapter {
                    area_m2: number(&configuration["area_m2"]),
                    volume_m3: number(&configuration["volume_m3"]),
                })
                .unwrap();
        } else if case["kind"] == "lumped" {
            system.throat_chamber = Some(Chamber::new(number(&configuration["volume_m3"])));
        } else {
            system = system
                .with_front_adapter(FrontAdapter {
                    volume_m3: 0.0,
                    ..adapter
                })
                .unwrap();
        }
        let mut error = Errors::default();
        let mut max_spl: f64 = 0.0;
        let mut max_balance: f64 = 0.0;
        for (i, f) in GRID.into_iter().enumerate() {
            let point = system.try_horn_operating_point_at(f).unwrap();
            let energy = system.try_energy_at(f).unwrap();
            let response = system.try_response_at(f).unwrap();
            let (pt, ut) = if case["kind"] == "duct" {
                system.horn.as_ref().unwrap().segments[0]
                    .try_matrix(&air, f)
                    .unwrap()
                    .apply(point.inlet_pressure_pa, point.inlet_flow_m3_s)
            } else {
                (point.inlet_pressure_pa, point.inlet_flow_m3_s)
            };
            for (name, actual) in [
                ("ze", point.electrical_impedance_ohm),
                ("front_z", point.front_impedance),
                ("velocity", point.diaphragm_velocity_m_s),
                ("throat_flow", ut),
                ("throat_pressure", pt),
                ("mouth_flow", point.mouth_flow_m3_s),
            ] {
                error.check(actual, complex(&case[name][i])).unwrap();
            }
            let spl = number(&case["spl_db"][i]);
            check_scalar(response.spl_db, spl, 0.015);
            max_spl = max_spl.max((response.spl_db - spl).abs());
            let dphase = (response.phase_deg - number(&case["phase_deg"][i]) + 180.0)
                .rem_euclid(360.0)
                - 180.0;
            check_scalar(dphase, 0.0, 0.1);
            for (name, value) in [
                ("xd_mm", response.displacement_mm),
                ("p_electric_w", energy.driver_terminal_active_w),
                (
                    "p_mechanical_w",
                    energy.suspension_loss_w + energy.acoustic_input_w,
                ),
                ("p_acoustic_w", energy.acoustic_input_w),
                ("p_radiated_w", energy.radiated_power.watts.unwrap()),
            ] {
                let expected = number(&case[name][i]);
                check_scalar(value, expected, 2e-4 * expected);
            }
            let balance = energy.balance_error_w() / energy.source_active_w;
            check_scalar(balance, 0.0, 1e-6);
            max_balance = max_balance.max(balance.abs());
        }
        println!(
            "coupling {} | {error:?} | SPL {max_spl:.3e} dB | balance/input {max_balance:.3e}",
            case["kind"]
        );
    }
}

#[test]
fn thermoviscous_cylinder_matches_kirchhoff_and_conserves_cut_losses() {
    let data: Value = serde_json::from_str(&fixture()).unwrap();
    manifest(&data).unwrap();
    let exact = &data["exact_kirchhoff_tube"];
    let air = Air::new(number(&data["c_m_s"]), number(&data["rho_kg_m3"]));
    let area = std::f64::consts::PI * 0.025_f64.powi(2);
    let segment = |length| {
        Segment::new(Flare::Cylindrical, length, area, area)
            .with_thermoviscous_losses(ThermoviscousAir::default())
    };
    let mut max_attenuation: f64 = 0.0;
    let mut error = Errors::default();
    for (i, f) in GRID.into_iter().enumerate() {
        let matrix = segment(0.27).try_matrix(&air, f).unwrap();
        for (name, value) in [("a", matrix.a), ("b", matrix.b), ("c", matrix.c)] {
            error.check(value, complex(&exact[name][i])).unwrap();
        }
        let attenuation = number(&exact["attenuation_np"][i]);
        let actual = matrix.a.acosh().re;
        max_attenuation = max_attenuation.max((actual / attenuation - 1.0).abs());
        check_scalar(actual, attenuation, 0.03 * attenuation);
        let tl = TransmissionLine::at_position(motor(&data["driver"]), vec![segment(0.27)], 0.05)
            .unwrap();
        let parts = &tl.segments;
        let cut = parts[0]
            .try_matrix(&air, f)
            .unwrap()
            .then(parts[1].try_matrix(&air, f).unwrap());
        for (full, divided) in [
            (matrix.a, cut.a),
            (matrix.b, cut.b),
            (matrix.c, cut.c),
            (matrix.d, cut.d),
        ] {
            check_scalar((full - divided).norm(), 0.0, 3e-12 * full.norm());
        }
    }
    let err = segment(0.27)
        .try_matrix(&air, 0.01)
        .unwrap_err()
        .to_string();
    assert!(err.contains("layer depth / radius"));
    let err = segment(0.27)
        .try_matrix(&air, 10_000.0)
        .unwrap_err()
        .to_string();
    assert!(err.contains("ka exceeds"));
    assert!(segment(0.27)
        .with_flow_resistivity(1000.0)
        .try_matrix(&air, 125.0)
        .is_err());
    println!("Kirchhoff cylinder {error:?} | relative attenuation {max_attenuation:.3e}");
}
