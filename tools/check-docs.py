"""Verifica enlaces Markdown locales y ejecuta los ejemplos de la guía con Cargo."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def body(path):
    return re.sub(r"^```.*?^```[^\n]*", "", path.read_text(), flags=re.M | re.S)


def anchors(path):
    titles = re.findall(r"^#{1,6} (.+)$", body(path), re.M)
    return {re.sub(r"[^\w -]", "", t.lower()).replace(" ", "-") for t in titles}


def check_links():
    files = sorted((ROOT / "docs").rglob("*.md")) + list(ROOT.glob("*.md"))
    failures = []
    for page in files:
        for link in re.findall(r"\]\(([^()\s]+)\)", body(page)):
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            dest = (page.parent / unquote(url.path)).resolve() if url.path else page
            if not dest.exists() or (url.fragment and dest.suffix == ".md"
                                     and unquote(url.fragment) not in anchors(dest)):
                failures.append(f"{page.relative_to(ROOT)} → {link}")
    if failures:
        sys.exit("\n".join(failures))
    print(f"Enlaces relativos verificados en {len(files)} páginas.")


if __name__ == "__main__":
    check_links()
    with tempfile.TemporaryDirectory() as directory:
        project = Path(directory)
        (project / "Cargo.toml").write_text(
            '[package]\nname="speakerbox-guide"\nversion="0.0.0"\nedition="2021"\n'
            '[lib]\npath="guide.rs"\n[dependencies]\nspeakerbox-acoustics={path='
            + json.dumps(str(ROOT)) + '}\n'
        )
        (project / "guide.rs").write_text(
            '#![doc = include_str!(' + json.dumps(str(ROOT / "docs/INTEGRATION.md")) + ')]\n'
        )
        shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target/doc-guide"))
        subprocess.run(["cargo", "test", "--offline", "--doc"], cwd=project, env=env, check=True)
