#!/usr/bin/env python3
"""Normalize published Wavebox Hornresp exports and retain independent PH1 flow."""
from argparse import ArgumentParser
from pathlib import Path
import csv
import io
import subprocess
import sys

REVISION = "4195c93f757a99c4bb2af456ff8dd8421a948510"


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    root = args.reference.resolve()
    rev = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if rev != REVISION or subprocess.check_output(["git", "status", "--porcelain"], cwd=root):
        parser.error(f"Expected a clean checkout at {REVISION}")
    sys.path.insert(0, str(root))
    from tools.calibrate import CASES
    from wavebox.system import solve
    import numpy as np

    def read(filename):
        file = root / "tools/hornresp/exports" / filename
        tracked = subprocess.check_output(["git", "show", f"HEAD:{file.relative_to(root)}"], cwd=root)
        with io.StringIO(tracked.decode("ascii")) as stream:
            rows = list(csv.reader(stream, delimiter="\t"))[1:]
        return [[float(x) for x in row] for row in rows if row]

    args.output.mkdir(parents=True, exist_ok=True)
    for case in ("ph1a", "bp8sx", "bpbx", "dbrx", "abcx", "bpcx"):
        a, b, c = (read(f"{case}_{field}.txt") for field in ("spl", "ze", "xd"))
        assert len(a) == len(b) == len(c) and len(a) > 500
        rows = []
        for (f, spl, phase), (fz, ze, zphase), (fx, xd) in zip(a, b, c):
            assert f == fz == fx
            if 10 <= f <= 2000:
                rows.append([f, spl, phase, ze, zphase, xd])
        columns = ["frequency_hz", "spl_db", "wphase_deg", "ze_ohm", "ze_phase_deg", "xd_peak_mm"]
        if case == "ph1a":
            result = solve(CASES[case][0], np.array([r[0] for r in rows]))
            columns += ["wavebox_u_re_m3_s", "wavebox_u_im_m3_s"]
            for row, u in zip(rows, result.sources["mouth"]):
                row.extend([f"{u.real:.12e}", f"{u.imag:.12e}"])
        with (args.output / f"{case}.csv").open("w", newline="") as out:
            writer = csv.writer(out, lineterminator="\n")
            writer.writerow(columns)
            writer.writerows(rows)
    for case in ("ref_semia", "ref_fdda"):
        rows = read(f"{case}_all.txt")
        with (args.output / f"{case}.csv").open("w", newline="") as out:
            writer = csv.writer(out, lineterminator="\n")
            writer.writerow(["frequency_hz", "spl_db", "wphase_deg", "ze_ohm", "ze_phase_deg", "xd_peak_mm"])
            for row in rows:
                if 10 <= row[0] <= 2000:
                    writer.writerow([row[i] for i in [0, 4, 7, 5, 15, 6]])


if __name__ == "__main__":
    main()
