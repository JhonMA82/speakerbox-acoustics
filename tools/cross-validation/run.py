#!/usr/bin/env python3
"""Build a disposable external harness; reference never becomes production dependency."""
import argparse,pathlib,tempfile,subprocess,shutil,json
p=argparse.ArgumentParser();p.add_argument('reference_repo');p.add_argument('--output',default='docs/reference-integration/completion-raw/cross');a=p.parse_args()
repo=pathlib.Path(__file__).resolve().parents[2];reference=pathlib.Path(a.reference_repo).resolve();out=pathlib.Path(a.output).resolve();out.mkdir(parents=True,exist_ok=True)
sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=reference,text=True).strip()
with tempfile.TemporaryDirectory(prefix='acoustic-cross-') as tmp:
    root=pathlib.Path(tmp);(root/'src').mkdir()
    (root/'Cargo.toml').write_text('[package]\nname="acoustic-cross-check"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nspeakerbox-acoustics={path='+json.dumps(str(repo))+'}\nloudspeaker-solver={path='+json.dumps(str(reference/'solver'))+'}\n')
    shutil.copyfile(repo/'tools/cross-validation/harness.rs',root/'src/main.rs')
    subprocess.run(['cargo','run','--release','--manifest-path',str(root/'Cargo.toml'),'--',str(out)],check=True)
(out/'reference_sha.txt').write_text(sha+'\n')
subprocess.run(['python3',str(repo/'tools/cross-validation/compare.py'),str(out/'speakerbox.csv'),str(out/'reference.csv'),'--output',str(out/'comparison.json')],check=True)
