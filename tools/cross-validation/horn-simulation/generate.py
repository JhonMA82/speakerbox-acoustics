#!/usr/bin/env python3
"""Generate the integration fixtures from a clean horn-simulation checkout."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace

import numpy as np
import scipy
from scipy.special import jve

REFERENCE = "7dd9852aec8f60e1b35ae637ad28bd614efc2c1b"
FREQUENCIES = np.array([25., 41., 69., 98., 147., 222., 331., 479., 630.])
SLICES = 4096
RT, RM, LENGTH = .025, .12, .30
T = math.sqrt(.5)
DRIVER = dict(sd_m2=.008, cms_m_per_n=.00035, moving_mass_kg=.015,
              rms_kg_per_s=1.1, re_ohm=5.8, le_h=.0004, bl_tm=7.6,
              rear_chamber_volume_m3=.008, voltage_rms=2.83)
ADAPTER = dict(volume_m3=.00018, area_m2=.0045)


def pairs(values):
    return [[float(v.real), float(v.imag)] for v in values]


def transfer_metrics(result, boundary, air, radius, piston):
    z = (result["z_real"] + 1j * result["z_imag"]) / result["inlet_area_m2"]
    p = result["mouth_pressure_transfer"]
    u = result["mouth_volume_velocity_transfer"]
    pin = np.real(1 / z)
    pout = np.real(p * np.conj(u))
    zm = air.rho * air.c / result["mouth_area_m2"]
    if boundary == "flanged_piston":
        zm *= np.array([piston(2*np.pi*f/air.c, radius) for f in FREQUENCIES])
    assert np.allclose(p, zm * u, rtol=1e-12)
    return z, p, u, pin - pout, np.broadcast_to(zm, z.shape)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    reference = args.reference.resolve()
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=reference, text=True).strip()
    if commit != REFERENCE or subprocess.check_output(["git", "status", "--porcelain"], cwd=reference):
        parser.error(f"reference must be a clean checkout at {REFERENCE}")

    sys.path.insert(0, str(reference / "packages/horn-core/src"))
    from horn_core.driver_interface import driver_operating_point
    from horn_core.duct import DEFAULT_AIR as air
    from horn_core.profiles import get_radius_func, natural_tractrix_length
    from horn_core.webster import compute_horn_transfer_tmm, piston_radiation_impedance

    natural = natural_tractrix_length(RT, RM)
    cases = []
    configurations = [
        ("tube", "cylindrical", RT, .27, "flanged_piston", "lossless", {}),
        ("cone", "conical", RM, LENGTH, "flanged_piston", "lossless", {}),
        ("exponential", "exponential", RM, LENGTH, "flanged_piston", "lossless", {}),
        ("hypex", "hypex", RM, LENGTH, "flanged_piston", "lossless", {"t": T}),
        ("tractrix", "tractrix", RM, LENGTH, "flanged_piston", "lossless", {}),
        # Compute the natural length with the same expression as the independent
        # parameterization (the Rust constructor may differ by a few ulps).
        ("tractrix_full", "tractrix", RM, natural, "flanged_piston", "lossless", {}),
        ("scaled_tractrix", "scaled_tractrix", RM, .19, "flanged_piston", "lossless", {}),
        ("os", "oblate_spheroidal", RM, LENGTH, "flanged_piston", "lossless", {}),
        ("compound", "exponential_conical", RM, LENGTH, "flanged_piston", "lossless", {"join_fraction": .3}),
        ("tube_matched", "cylindrical", RT, .27, "plane_wave", "lossless", {}),
        ("tube_thermoviscous", "cylindrical", RT, .27, "flanged_piston", "boundary_layer", {}),
    ]
    profile_names = {"cylindrical": "conical", "hypex": "hyperbolic", "oblate_spheroidal": "os", "exponential_conical": "cd"}
    coupling = None
    for name, kind, rm, length, boundary, losses, extra in configurations:
        params = {"hypex_t": T} if kind == "hypex" else {}
        radius = get_radius_func(profile_names.get(kind, kind), RT, rm, length, **params)
        values = [
            transfer_metrics(
                compute_horn_transfer_tmm(FREQUENCIES, radius, length, RT, rm, n_segments=n,
                                          radiation_model=boundary, loss_model=losses, air=air),
                boundary, air, rm, piston_radiation_impedance,
            )
            for n in [SLICES // 2, SLICES]
        ]
        # Make the convergence of the inherited TMM an explicit fixture check.
        for coarse, fine in zip(values[0][:3], values[1][:3]):
            if np.max(np.abs(coarse / fine - 1)) > 5e-5:
                raise ValueError(f"{name}: reference slices have not converged")
        z, p, u, net_loss, zm = values[1]
        case = dict(name=name, boundary=boundary, losses=losses, profile=dict(
            version=1, kind=kind, throat_radius_m=RT, mouth_radius_m=rm, length_m=length, **extra),
            z_in=pairs(z), p_mouth=pairs(p), u_mouth=pairs(u), net_loss_w=list(net_loss))
        if name == "cone":
            operating = []
            for front_kind, adapter in [("ideal", dict(volume_m3=0., area_m2=None)),
                                        ("lumped", dict(volume_m3=.00018, area_m2=None)),
                                        ("duct", ADAPTER)]:
                driver = SimpleNamespace(**{k: v for k, v in DRIVER.items() if k != "voltage_rms"},
                    coupled_moving_mass_kg=DRIVER["moving_mass_kg"],
                    front_chamber_volume_m3=adapter["volume_m3"], front_chamber_area_m2=adapter["area_m2"])
                out = driver_operating_point(driver, FREQUENCIES, z * np.pi * RT**2,
                    np.pi * RT**2, DRIVER["voltage_rms"], rho=air.rho, c=air.c)
                ud = driver.sd_m2 * out["velocity_rms"]
                pt = out["throat_pressure"]
                ut = out["throat_volume_velocity"]
                observer_flow = u * pt
                # This observer is the Rust legacy power-equivalent SPL at 1 m in
                # 2π. The upstream on-axis Rayleigh observer is a different observable.
                power = np.abs(observer_flow)**2 * zm.real
                operating.append(dict(kind=front_kind, adapter=adapter,
                    ze=pairs(out["electrical_impedance"]),
                    front_z=pairs((z * ut if front_kind != "duct" else
                                  np.cos(2*np.pi*FREQUENCIES/air.c*adapter["volume_m3"]/adapter["area_m2"])*pt
                                  + 1j*air.rho*air.c/adapter["area_m2"]
                                  *np.sin(2*np.pi*FREQUENCIES/air.c*adapter["volume_m3"]/adapter["area_m2"])*ut) / ud),
                    velocity=pairs(out["velocity_rms"]), throat_flow=pairs(ut), throat_pressure=pairs(pt),
                    mouth_flow=pairs(observer_flow),
                    spl_db=list(10 * np.log10(power * (air.rho * air.c / (2*np.pi)) / (20e-6)**2)),
                    phase_deg=list(np.angle(observer_flow, deg=True)),
                    xd_mm=list(1000*out["displacement_peak_m"]),
                    p_electric_w=list(out["input_power_w"]),
                    p_mechanical_w=list(out["mechanical_loss_w"] + out["horn_power_w"]),
                    p_acoustic_w=list(out["horn_power_w"]),
                    p_radiated_w=list(power)))
            coupling = operating
        cases.append(case)

    # Independent circular-section Kirchhoff solution (scaled complex Bessel
    # functions). Compare the thin-layer approximation only in its stated domain.
    w = 2 * np.pi * FREQUENCIES
    xv = (1-1j) * RT / np.sqrt(2 * air.viscosity / (air.rho * w))
    xt = (1-1j) * RT / np.sqrt(2 * air.conductivity / (air.rho * air.heat_capacity * w))
    density = air.rho / (1 - 2*jve(1, xv)/(xv*jve(0, xv)))
    compressibility = (1 + (air.gamma-1)*2*jve(1, xt)/(xt*jve(0, xt))) / (air.rho*air.c**2)
    zc = np.sqrt(density / compressibility) / (np.pi*RT**2)
    kl = w * np.sqrt(density*compressibility) * .27
    exact = dict(a=pairs(np.cos(kl)), b=pairs(-1j*zc*np.sin(kl)), c=pairs(-1j*np.sin(kl)/zc),
                 attenuation_np=list(-kl.imag))
    data = dict(schema_version=1, source_revision=commit, numpy=np.__version__, scipy=scipy.__version__,
        c_m_s=air.c, rho_kg_m3=air.rho, frequency_hz=list(FREQUENCIES), slices=SLICES,
        thermophysical_air=dict(viscosity_pa_s=air.viscosity, thermal_conductivity_w_m_k=air.conductivity,
            heat_capacity_j_kg_k=air.heat_capacity, heat_capacity_ratio=air.gamma),
        phasor="exp(+iwt) RMS", drive="throat pressure 1 Pa RMS",
        impedance_unit="Pa s/m^3", p_ref_pa=20e-6, cases=cases,
        driver=DRIVER, coupling=coupling, exact_kirchhoff_tube=exact)
    args.output.write_text(json.dumps(data, indent=2, allow_nan=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
