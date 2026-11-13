#!/usr/bin/env python3
"""Compare normalized exports (not an oracle). Blank cells mean unavailable.
Schema: frequency_hz plus any of spl_db,ze_ohm,ze_phase_deg,excursion_peak_mm,
phase_deg,delay_ms,mouth_u_m3_s,power_w. Phase errors are circular degrees.
Power may only be supplied for matching definitions established by exporter.
"""
import argparse,csv,json,math,pathlib
FIELDS=('spl_db','ze_ohm','ze_phase_deg','excursion_peak_mm','phase_deg','delay_ms','mouth_u_m3_s','power_w')
def read(path):
    with open(path) as f: rows=list(csv.DictReader(f))
    out=[]
    for row in rows:
        values={k:float(v) for k,v in row.items() if v and k in ('frequency_hz',)+FIELDS}
        if 'frequency_hz' not in values or any(not math.isfinite(v) for v in values.values()): raise ValueError('missing or nonfinite data')
        if values['frequency_hz']<=0 or out and values['frequency_hz']<=out[-1]['frequency_hz']: raise ValueError('frequencies must strictly increase')
        out.append(values)
    if not out:raise ValueError('empty export')
    return out

def compare(a,b,tolerances=None):
    if not a or not b:raise ValueError('empty comparison')
    if tolerances is not None and (not isinstance(tolerances,dict) or any(not isinstance(v,(float,int)) or not math.isfinite(v) or v<0 for v in tolerances.values())):raise ValueError('invalid tolerance')
    if len(a)!=len(b) or any(x['frequency_hz']!=y['frequency_hz'] for x,y in zip(a,b)):raise ValueError('frequency grids differ')
    result={'category':'comparison of implementations, not independent physical validation','metrics':{}}
    for field in FIELDS:
        errors=[(x['frequency_hz'],(x[field]-y[field]+180)%360-180 if field.endswith('phase_deg') else x[field]-y[field]) for x,y in zip(a,b) if field in x and field in y]
        if not errors:continue
        tolerance=(tolerances or {}).get(field)
        def stats(rows):
            v=[e for _,e in rows]
            return {'n':len(v),'rms':math.sqrt(sum(e*e for e in v)/len(v)),'max_abs':max(map(abs,v)),'bias':sum(v)/len(v),'out_of_tolerance':None if tolerance is None else sum(abs(e)>tolerance for e in v)}
        result['metrics'][field]=stats(errors)
        result['metrics'][field]['bands']={f'{lo:g}-{hi:g}Hz':stats(v) for lo,hi in [(0,20),(20,100),(100,1000),(1000,float('inf'))] if (v:=[(f,e) for f,e in errors if lo<=f<hi])}
    if not result['metrics']:raise ValueError('no comparable quantities')
    return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('candidate');p.add_argument('reference');p.add_argument('--tolerances',help='JSON dictionary, absolute native units');p.add_argument('--output');a=p.parse_args()
    r=compare(read(a.candidate),read(a.reference),json.loads(a.tolerances) if a.tolerances else None)
    text=json.dumps(r,indent=2,allow_nan=False);print(text)
    if a.output:pathlib.Path(a.output).write_text(text+'\n')
