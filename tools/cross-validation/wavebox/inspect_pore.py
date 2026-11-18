#!/usr/bin/env python3
"""Evaluate the available calibration data and positive-real duct coefficients."""
from pathlib import Path
import argparse
import sys

import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wavebox", type=Path)
    root = parser.parse_args().wavebox
    sys.path.insert(0, str(root.resolve()))
    from wavebox.acoustics import Air, porous

    air = Air()
    for sigma in (250, 1000, 4000):
        file = root / f"tools/hornresp/exports/absorbent_{sigma}.npz"
        with np.load(file, allow_pickle=False) as data:
            w = 2*np.pi*data["f"]
            print(f"sigma={sigma}, n={len(w)}, Hz {data['f'][0]:g}–{data['f'][-1]:g}")
            for model in ("pore", "flow", "db"):
                k, z = porous(w, sigma, air, model)
                print(model, *(f"{metric}_relative_rms={np.sqrt(np.mean(abs(value / data[metric]-1)**2)):.5f}"
                              for metric, value in [("k", k), ("z", z)]))
        w = 2*np.pi*np.geomspace(.01, 2000, 300)
        k, z = porous(w, sigma, air, "pore")
        series = (1j*k*z).real
        shunt = (1j*k/z).real
        assert (series > 0).all() and (shunt > 0).all()
        print(f"low frequency relative resistance={series[0]/sigma:.6f}")


if __name__ == "__main__":
    main()
