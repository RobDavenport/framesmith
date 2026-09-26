"""Build the same binary-only static consumer locally and on Pages."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


def run(*args):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=ROOT, check=True, env=dict(os.environ))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packs-only', action='store_true')
    args = parser.parse_args()
    # Native Cargo/GCM need a real host path, not MSYS /tmp.
    temp = Path(os.environ['LOCALAPPDATA']) / 'Temp' if os.name == 'nt' else Path(tempfile.gettempdir())
    if not temp.is_dir():
        raise RuntimeError(f'Native temporary directory is unavailable: {temp}')
    for key in ('TMP', 'TEMP', 'TMPDIR'):
        os.environ[key] = str(temp)
    site = HERE / 'dist'
    (site / 'packs').mkdir(parents=True, exist_ok=True)
    run('cargo', 'build', '--manifest-path', ROOT / 'src-tauri/Cargo.toml', '--bin', 'framesmith-cli', '--locked')
    cli = ROOT / 'src-tauri/target/debug' / ('framesmith-cli.exe' if os.name == 'nt' else 'framesmith-cli')
    source = HERE / 'lab-authoring'
    project = {p.relative_to(source).as_posix(): json.loads(p.read_text(encoding='utf-8'))
               for p in sorted(source.rglob('*.json'))}
    (site / 'lab-source.json').write_text(json.dumps(project, sort_keys=True), encoding='utf-8')
    run(cli, 'export', '--project', source, '--character', 'relay',
        '--adapter', 'fspk', '--out', site / 'packs' / 'lab.fspk')
    assert (site / 'packs' / 'lab.fspk').read_bytes()[:4] == b'FSPK'
    # Only remove artifacts of the retired roster build; user source is untouched.
    for old in ('relay.fspk', 'bulwark.fspk', 'sable.fspk', 'zip.fspk'):
        (site / 'packs' / old).unlink(missing_ok=True)
    if args.packs_only:
        return
    run('cargo', 'test', '--manifest-path', HERE / 'Cargo.toml', '--locked')
    run('wasm-pack', 'build', HERE, '--target', 'web', '--release', '--out-dir', 'pkg')
    (site / 'pkg').mkdir(exist_ok=True)
    for name in ('index.html', 'style.css', 'main.js'):
        shutil.copyfile(HERE / 'www' / name, site / name)
    shutil.copytree(HERE / 'www' / 'assets', site / 'assets', dirs_exist_ok=True)
    for name in ('framesmith_arena.js', 'framesmith_arena_bg.wasm'):
        shutil.copyfile(HERE / 'pkg' / name, site / 'pkg' / name)
    (site / '.nojekyll').write_text('')
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    files = {p.relative_to(site).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in sorted(site.rglob('*')) if p.is_file() and p.name != 'build-info.json'}
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain', '--', 'demo-wasm', 'crates/framesmith-authoring', 'crates/framesmith-runtime', 'crates/framesmith-fspack', 'src-tauri'], cwd=ROOT, text=True).strip())
    (site / 'build-info.json').write_text(json.dumps({'commit': commit, 'dirty': dirty, 'files': files}, indent=2))
    print(f'Static game built: {site}', flush=True)


if __name__ == '__main__':
    main()
