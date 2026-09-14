"""Compile and execute original GB control-flow regression ROMs with KOKURA.

Pass --compiler and --emulator paths; --output-parent chooses where evidence is
retained. CPU memory watches prove these cases only, not physical hardware.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile


def digest(path):
    """Bind each result to its exact source, executable or runtime artifact."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, directory, log):
    """Bound each tool invocation and retain byte-safe diagnostics."""
    try:
        process = subprocess.run(command, cwd=directory, capture_output=True, timeout=45)
    except subprocess.TimeoutExpired as error:
        log.write_bytes((error.stdout or b'') + (error.stderr or b''))
        raise
    log.write_bytes(process.stdout + process.stderr)
    return process.returncode


# Result triples cover switch evaluation, nearest break scopes and induction on continue.
# Visit caps keep broken continue paths finite so the completion marker remains useful.
cases = [
('default_only', 'switch (input()) { default: result[0]=7; break; } result[1]=9;', [7,9,1]),
('default_only_fallthrough', 'switch (input()) { default: result[0]=7; } result[1]=9;', [7,9,1]),
('empty_switch', 'switch (input()) { } result[1]=9;', [0,9,1]),
('matched_case', 'switch (input()) { case 2: result[0]=8; break; case 3: result[0]=7; break; default: result[0]=6; } result[1]=9;', [7,9,1]),
('ordinary_continue', 'for (i=0; i<4 && visits<12; i++) { visits++; if(i<3) continue; sum++; } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,1]),
('unsafe_continue', 'for (i=0; i<4 && visits<12; i++) { visits++; __unsafe { if(i<3) continue; } sum++; } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,1]),
('switch_case_continue', 'for (i=0; i<4 && visits<12; i++) { visits++; switch(i) { case 0: continue; default: sum++; } } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,3]),
('switch_default_continue', 'for (i=0; i<4 && visits<12; i++) { visits++; switch(i) { case 3: sum++; break; default: continue; } } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,1]),
('while_break_in_switch', 'switch(input()) { case 3: while(j<3) { j++; break; } result[0]=j; result[1]=7; break; default: result[0]=9; }', [1,7,1]),
('do_break_in_switch', 'switch(input()) { case 3: do { j++; break; } while(j<3); result[0]=j; result[1]=7; break; default: result[0]=9; }', [1,7,1]),
('for_break_in_switch', 'switch(input()) { case 3: for(j=0;j<3;j++) { sum++; break; } result[0]=sum; result[1]=7; break; default: result[0]=9; }', [1,7,1]),
('switch_break_in_loop', 'for(i=0;i<3;i++) { switch(i) { case 1: sum+=2; break; default: sum++; } visits++; } result[0]=i; result[1]=visits; result[2]=sum;', [3,3,4]),

('countdown_break_in_switch', 'switch(input()) { case 3: for(j=3;j!=0;j--) { sum++; break; } result[0]=sum; result[1]=j; break; default: result[0]=9; }', [1,3,1]),
('nested_switch_loop_break', 'switch(input()) { case 3: for(i=0;i<2;i++) { switch(i) { case 0: sum++; break; default: sum+=2; } visits++; break; } result[0]=sum; result[1]=visits; break; default: result[0]=9; }', [1,1,1]),
('nested_for_continue', 'for(i=0;i<2;i++) { for(j=0;j<3 && visits<12;j++) { visits++; if(j<2) continue; sum++; } } result[0]=i; result[1]=visits; result[2]=sum;', [2,6,2]),
('nested_do_continue', 'for(i=0;i<2;i++) { j=0; do { j++; visits++; if(j<2) continue; sum++; } while(j<3); } result[0]=i; result[1]=visits; result[2]=sum;', [2,6,4]),
('unsafe_switch_continue', 'for(i=0;i<4 && visits<12;i++) { visits++; __unsafe { switch(i) { case 0: continue; default: sum++; } } } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,3]),
('default_loop_break', 'switch(input()) { default: while(j<3) { j++; break; } result[0]=j; result[1]=7; break; }', [1,7,1]),
('constant_expression_case_continue', 'for(i=0;i<4 && visits<12;i++) { visits++; switch(i) { case 1-1: continue; default: sum++; } } result[0]=i; result[1]=visits; result[2]=sum;', [4,4,3]),
('side_effect_condition_continue', 'for(i=0;condition();i++) { if(i<3) continue; sum++; } result[0]=i; result[1]=visits; result[2]=sum;', [4,5,1]),
]


def fixture(body):
    """Use explicit RAM observations and a final marker to distinguish incomplete runs."""
    return '''// Original bounded control-flow regression fixture.
__location(0xC700) u8 result[4];
u8 i; u8 j; u8 visits; u8 sum; u8 key;
u8 input() { result[2]++; return key; }
u8 condition() { visits++; return i<4; }
void main() {
result[0]=0; result[1]=0; result[2]=0; result[3]=0;
i=0; j=0; visits=0; sum=0; key=3;
''' + body + '\nresult[3]=0xA5; while(1) { }\n}\n'


def main():
    """Run each independent fixture and report failures without hiding later cases."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output-parent', type=Path)
    args = parser.parse_args()
    compiler = args.compiler.resolve(strict=True)
    emulator = args.emulator.resolve(strict=True)
    if args.output_parent:
        args.output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='gb-statement-flow-', dir=args.output_parent)).resolve()
    record = dict(compiler=str(compiler), compiler_sha256=digest(compiler), emulator=str(emulator),
        emulator_sha256=digest(emulator), cases=[], scope='Original ROMs and KOKURA DMG memory watches; no physical hardware.')
    for name, body, expected in cases:
        directory = output/name
        directory.mkdir()
        source = directory/'case.c'
        source.write_text(fixture(body), encoding='ascii')
        rom = directory/'case.gb'
        command = [str(compiler),str(source),'--cgb=dmg','--profile=dev','--rst-disable',
            '--stack-bank=fixed','--no-cache','--no-disasm','-o',str(rom)]
        rc = run(command,directory,directory/'compile.log')
        case = dict(name=name,expected=expected+[165],source_sha256=digest(source),command=command,compile_exit=rc,passed=False)
        if rc == 0:
            report_path = directory/'runtime.json'
            run_command = [str(emulator),str(rom),'--hardware','dmg','--run-frames','4',
                '--watch-window','result:0xC700:4','--dump-report',str(report_path)]
            run_exit = run(run_command,directory,directory/'run.log')
            case['run_exit'] = run_exit
            if run_exit == 0:
                state = json.loads(report_path.read_text(encoding='utf-8'))
                watches = [w for w in state['watched_memory'] if w['name']=='result']
                if len(watches)!=1 or watches[0]['addr']!=0xC700 or watches[0]['size']!=4:
                    raise RuntimeError('Invalid result watch: '+name)
                actual = watches[0]['preview_bytes']
                case.update(actual=actual,passed=actual==case['expected'],rom_sha256=digest(rom),
                    runtime_sha256=digest(report_path),run_command=run_command)
        record['cases'].append(case)
        # Persist after each case so completed evidence survives a later tool failure.
        (output/'report.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    failures = [case['name'] for case in record['cases'] if not case['passed']]
    print(json.dumps(dict(report=str(output/'report.json'),cases=len(cases),failures=failures)))
    if failures:
        raise SystemExit(1)


if __name__=='__main__':
    main()
