"""Inventory the locked, platform-filtered normal/build dependency closures."""
from pathlib import Path
import argparse, hashlib, json, shutil, subprocess

ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--offline',action='store_true')
parser.add_argument('--fallback',type=Path,help='Previously retained upstream notices')
args=parser.parse_args()
platforms={}; packages={}
for target in ['x86_64-pc-windows-msvc','x86_64-unknown-linux-gnu','aarch64-apple-darwin','x86_64-apple-darwin']:
    for mode,flags in [('gui',[]),('cli',['--no-default-features'])]:
        command=['cargo','metadata','--locked','--format-version','1','--filter-platform',target,*flags]
        if args.offline:command.append('--offline')
        data=json.loads(subprocess.check_output(command,cwd=ROOT))
        nodes={n['id']:n for n in data['resolve']['nodes']}; todo=[data['resolve']['root']]; seen=set()
        while todo:
            name=todo.pop()
            if name in seen:continue
            seen.add(name)
            todo.extend(d['pkg'] for d in nodes[name]['deps'] if any(k['kind']!='dev' for k in d['dep_kinds']))
        selected=[p for p in data['packages'] if p['id'] in seen]
        platforms[target+'/'+mode]=sorted(p['name']+'-'+p['version'] for p in selected)
        packages.update((p['id'],p) for p in selected)
rows=[];missing=[];unsupported=[]
allowed={'MIT','Apache-2.0','Apache-2.0 OR MIT','MIT OR Apache-2.0','MIT/Apache-2.0',
 'BSD-2-Clause','BSD-3-Clause','ISC','Zlib','CC0-1.0','Unlicense','0BSD',
 'MIT OR Unlicense','Unlicense OR MIT','MIT OR Apache-2.0 OR Zlib','Zlib OR Apache-2.0 OR MIT',
 'Apache-2.0 AND MIT','MIT AND Apache-2.0','MIT AND (MIT OR Apache-2.0)',
 'MIT OR Apache-2.0 OR LGPL-2.1-or-later','(MIT OR Apache-2.0) AND Unicode-3.0',
 'Unicode-3.0','Apache-2.0 WITH LLVM-exception','0BSD OR MIT OR Apache-2.0',
 'Apache-2.0/MIT','Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT',
 'BSD-2-Clause OR MIT OR Apache-2.0','MIT OR Zlib OR Apache-2.0',
 'BSD-2-Clause OR Apache-2.0 OR MIT'}
for package in sorted(packages.values(),key=lambda x:(x['name'],x['version'])):
    if package['name']=='gbseq':continue
    source=Path(package['manifest_path']).parent
    name=package['name']+'-'+package['version']
    files=[f for f in source.rglob('*') if f.is_file() and f.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE','COPYRIGHT','UNLICENSE'))]
    if package.get('license_file') and (source/package['license_file']).is_file():files.append(source/package['license_file'])
    if not files and (ROOT/'licenses/dependencies'/name).is_dir():
        source=ROOT/'licenses/dependencies'/name;files=[f for f in source.rglob('*') if f.is_file()]
    if not files and args.fallback and (args.fallback/name).is_dir():
        source=args.fallback/name;files=[f for f in source.rglob('*') if f.is_file()]
    notices=[]
    for file in sorted(set(files)):
        destination=ROOT/'licenses/dependencies'/name/file.relative_to(source)
        destination.parent.mkdir(parents=True,exist_ok=True)
        if destination.resolve()!=file.resolve():shutil.copy2(file,destination)
        notices.append(destination.relative_to(ROOT).as_posix())
    if not notices:missing.append({'name':name,'license':package['license'],'repository':package['repository']})
    if package['license'] not in allowed:unsupported.append({'name':name,'license':package['license']})
    rows.append({'name':package['name'],'version':package['version'],'license':package['license'],
                 'source':package['source'] or 'bundled source in vendor/',
                 'repository':package['repository'],'notices':notices})
sysroot=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())/'share/doc/rust'
for file in [sysroot/'COPYRIGHT-library.html',*(sysroot/'licenses').rglob('*')]:
    if file.is_file():
        dest=ROOT/'licenses/rust-standard-library'/file.relative_to(sysroot)
        dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(file,dest)
report={'application':'GBSEQ 0.1.0','author':'DAISUKE OBA','license':'MIT',
 'scope':'Conservative normal/build dependency closure for GUI and headless CLI on four native targets; Cargo.lock fixes versions. Not every build-time crate is linked into binaries.',
 'cargo_lock_sha256':hashlib.sha256((ROOT/'Cargo.lock').read_bytes()).hexdigest(),
 'platforms':platforms,'packages':rows,'missing_notices':missing,'review_required':unsupported}
(ROOT/'LICENSE_AUDIT.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
header='''# Third-party notices — GBSEQ

GBSEQ application: Copyright (c) 2026 DAISUKE OBA, under the [MIT License](LICENSE).
Third-party software, fonts and icons retain their original authors and terms.
GBSEQ's MIT license does not relicense third-party material or imported music.

## Bundled components and assets

The required GBDAW model/export/audio libraries and KOKURA core are included
as MIT source in `vendor/`. No KOKURA GUI, game ROM, boot ROM, commercial
music, sound sample or user project is included. Hardware names identify
compatibility and do not imply endorsement.

The five plita foundation crates retain their upstream MIT notice. This
snapshot makes Cargo manifests self-contained, enables Metal on macOS,
and adds executable-relative font discovery. Public API and environment
variable names (for example `PlitaProgram`, `PLITA_UI_FONT`) remain unchanged.

Unmodified Roboto and Roboto Mono fonts are distributed under SIL OFL 1.1,
with their original copyrights in `assets/fonts/*-OFL.txt` and `licenses/fonts/`.
Fonts must retain those terms when redistributed; they are not under MIT.
See the [official OFL text](https://openfontlicense.org/open-font-license-official-text/).
Lucide icons and Feather-derived paths retain ISC and MIT notices in
`licenses/assets/Lucide-Feather.txt` and `vendor/plita_ui/assets/lucide/LICENSE`.
See [Lucide's upstream license](https://github.com/lucide-icons/lucide/blob/main/LICENSE).
Operating-system fonts are only loaded at runtime and are not redistributed.
The two bundled fonts match official Google Fonts binaries byte for byte;
all 52 SVG geometries match official Lucide sources (current or 0.468.0).
Hashes and source URLs are in [the asset provenance record](licenses/ASSET_PROVENANCE.json).

## Dependency choices and binary redistribution

`LICENSE_AUDIT.json` records the GUI/CLI dependency graphs for Windows x86_64,
Linux x86_64, macOS ARM64 and macOS x86_64. Registry versions are pinned by
`Cargo.lock`. Full upstream notices are retained under `licenses/dependencies/`;
Rust standard-library notices are under `licenses/rust-standard-library/`.
For OR expressions, the MIT option is chosen where available (otherwise the
permissive Apache/BSD/Zlib option). AND expressions are cumulative: in particular,
`dpi` retains both Apache and MIT notices, and `unicode-ident` retains Unicode
terms too. The optional LGPL alternative of `r-efi` is not selected. Audio
decoder features are disabled: GBSEQ synthesizes PCM and does not require them.

Keep `LICENSE`, this file, `LICENSE_AUDIT.json`, `licenses/` and font notices
with binary redistributions. OS libraries/frameworks (Windows, Apple, Linux
ALSA/X11/Wayland and graphics drivers) are runtime dependencies, not bundled.
Each native package additionally retains its build toolchain's Rust notices.
This review covers the files and resolved versions in this distribution;
future dependency changes require another audit.

Some older macOS binding crates omit a standalone license in the registry
archive. `licenses/UPSTREAM_NOTICE_PROVENANCE.json` records the official source
revisions, fetched files and hashes used to supplement them. For objc2 versions
that declare MIT but provide only a licensing discussion, the exact discussion
is retained alongside the same project's later full MIT text, clearly labeled
as supplemental. The retained upstream discussion mentions Apple SDK-derived
declarations; Apple SDKs/frameworks are not redistributed in this package.
`dispatch` has an explicit MIT declaration and named author but no upstream
license file; its supplemental NOTICE records that distinction. This inventory
does not claim legal clearance of arbitrary future inputs or dependencies.

## Inventory

| Component | Version | Declared license | Full notice |
|---|---|---|---|
'''
for row in rows:
    header+=f"| {row['name']} | {row['version']} | {row['license']} | "+(f"[notice]({row['notices'][0]})" if row['notices'] else 'MISSING')+" |\n"
(ROOT/'THIRD_PARTY_NOTICES.md').write_text(header,encoding='utf-8')
print(json.dumps({'packages':len(rows),'missing':missing,'review_required':unsupported},indent=2))
if missing or unsupported:raise SystemExit(1)
