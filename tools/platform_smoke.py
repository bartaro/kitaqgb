"""Native CLI checks, runnable on Windows, Linux and macOS without C#."""
from pathlib import Path
import argparse, datetime, hashlib, json, os, platform, subprocess, tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--compiler', required=True, type=Path)
parser.add_argument('--receipt', required=True, type=Path)
options = parser.parse_args()
compiler = options.compiler.resolve()
repository = Path(__file__).resolve().parents[1]
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
records = []
environment = dict(os.environ)
environment['PATH'] = str(compiler.parent) + os.pathsep + environment.get('PATH', '')

with tempfile.TemporaryDirectory(prefix='kitaqgb-platform-') as temporary:
    root = Path(temporary)
    source = repository/'tests/emission-fixtures/runtime.c'
    common = ['-O0', '--cgb=cgb', '--cart=romonly', '--romsize=32k', '--no-cache', '--fast-build']
    expected_rom = '8219bba20fa9d34b282b05a35ec842bef4139be8fc0b5369c0f1e98b0eaa0ab7'

    def invoke(case, arguments, expected_exit=0, program=None):
        result = subprocess.run([str(program or compiler), *map(str, arguments)], cwd=root, env=environment, capture_output=True, timeout=60)
        record = {'case': case, 'exit_code': result.returncode, 'passed': result.returncode == expected_exit, 'stdout': result.stdout.decode('utf-8', errors='replace'), 'stderr': result.stderr.decode('utf-8', errors='replace')}
        records.append(record)
        assert record['passed'], record
        return record

    invoke('version', ['--version'])
    rom = root/'runtime.gb'
    r = invoke('native-compile', [source, '-o', rom, *common])
    r['rom_sha256'] = sha(rom)
    r['passed'] &= r['rom_sha256'] == expected_rom
    assert r['passed'], r
    watched = root/'watched.gb'
    r = invoke('devserver-once', ['devserver', '--once', source, '-o', watched, *common])
    r['rom_sha256'] = sha(watched)
    r['passed'] &= r['rom_sha256'] == expected_rom
    assert r['passed'], r
    broken = root/'broken.c'
    broken.write_text('#error deliberate\nvoid main(){}\n', encoding='utf-8')
    invoke('failure-status', [broken, '-o', root/'broken.gb', '--no-cache', '--fast-build'], expected_exit=1)
    template = root/"demo 'literal' $x.c"
    invoke('template', ['template', template])
    history = root/'history.tsv'
    recorded = subprocess.list2cmdline([str(source), '-o', str(root/'replayed.gb'), *common])
    history.write_text('2026-10-03T00:00:00Z\t'+str(root)+'\t'+recorded+'\n', encoding='utf-8')
    replay = root/'replay.ps1'
    invoke('recipe', ['recipe', '--history='+str(history), '--out='+str(root/'recipe.md'), '--script='+str(replay)])
    if os.name == 'nt':
        invoke('recipe-replay', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', replay, '-Compiler', compiler], program='powershell')
    else:
        invoke('template-shell-build', [str(template.with_name(template.stem+'_build.sh'))], program='sh')
        invoke('recipe-replay', [str(replay.with_suffix('.sh')), compiler], program='sh')
    r = records[-1]
    r['rom_sha256'] = sha(root/'replayed.gb')
    r['passed'] &= r['rom_sha256'] == expected_rom
    assert r['passed'], r

receipt = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'system': platform.system(), 'machine': platform.machine(), 'platform': platform.platform(), 'compiler_sha256': sha(compiler), 'source_sha256': sha(source), 'scope': 'Native compiler process, golden ROM bytes, devserver, failure status, templates and safe recipe replay; no emulated Game Boy execution in this script.', 'passed': all(r['passed'] for r in records), 'records': records}
options.receipt.parent.mkdir(parents=True, exist_ok=True)
options.receipt.write_text(json.dumps(receipt, indent=2), encoding='utf-8')
print(json.dumps({k: v for k, v in receipt.items() if k != 'records'}, indent=2))
