"""Windows regression checks for native arguments, recipe replay and devserver exit status.

Uses a supplied compiler plus a small native argument-capture fixture. No emulator
or hardware checks are claimed. Run with Python -B to avoid cache files.
"""
from pathlib import Path
import argparse,base64,hashlib,json,os,shutil,subprocess,sys,tempfile

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--compiler',required=True,type=Path)
parser.add_argument('--target',required=True,choices=['gb','fc'])
parser.add_argument('--output-parent',type=Path,default=Path('.'))
parser.add_argument('--csc',type=Path,default=Path('C:/Program Files/dotnet/sdk/8.0.302/Roslyn/bincore/csc.dll'))
options=parser.parse_args()
options.output_parent.mkdir(parents=True,exist_ok=True)
runroot=Path(tempfile.mkdtemp(prefix='process-workflow-',dir=options.output_parent.resolve()))
CSC=options.csc.resolve()
phase='supplied-compiler'
FW=Path('C:/Windows/Microsoft.NET/Framework64/v4.0.30319')
CASES=['plain','','a b','C:\\space path\\','quote"inside','slash\\\\"quote','$literal',"a'b",'semi;colon','line\nbreak','tab\tvalue','Japanese-日本語']
cs=r'''using System; using System.IO; using System.Linq; using System.Text;
using System.Diagnostics; using System.Reflection;
class Capture {
    static int Main(string[] args) {
        if (args.Length==2 && args[0]=="--probe") {
            var assembly=Assembly.LoadFrom(args[1]);
            var method=assembly.GetType("Program").GetMethod("QuoteArg", BindingFlags.Static|BindingFlags.NonPublic);
            string[] cases=new string[] { CASES };
            var psi=new ProcessStartInfo(Assembly.GetExecutingAssembly().Location) {
                UseShellExecute=false,CreateNoWindow=true,
                Arguments=string.Join(" ",cases.Select(a=>(string)method.Invoke(null,new object[]{a})))
            };
            using(var child=Process.Start(psi)) { child.WaitForExit(); return child.ExitCode; }
        }
        string line=Convert.ToBase64String(Encoding.UTF8.GetBytes(Environment.CurrentDirectory))+"\t"+args.Length+"\t"+
            string.Join(",",args.Select(a=>Convert.ToBase64String(Encoding.UTF8.GetBytes(a))))+Environment.NewLine;
        File.AppendAllText(Environment.GetEnvironmentVariable("KITAQ_TEST_CAPTURE"),line,Encoding.UTF8);
        return args.Contains("--fail=7") ? 7 : 0;
    }
}'''.replace('CASES',','.join(json.dumps(s,ensure_ascii=True) for s in CASES))
(runroot/'capture.cs').write_text(cs,encoding='utf-8')

def run(command,folder,label,env=None):
    """Run controlled arguments without a shell and retain both output streams."""
    process=subprocess.run([str(s) for s in command],cwd=folder,capture_output=True,timeout=90,
        env=env,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    (folder/(label+'.stdout.log')).write_bytes(process.stdout)
    (folder/(label+'.stderr.log')).write_bytes(process.stderr)
    return process.returncode

def build(sources,exe,refs,folder,label):
    """Build only the native capture fixture with explicit framework references."""
    command=['dotnet',CSC,'/nologo','/noconfig','/nostdlib+','/target:exe','/optimize+','/define:TRACE','/utf8output',
             '/out:'+str(exe)]+['/reference:'+str(FW/(r+'.dll')) for r in refs]+[str(p) for p in sources]
    code=run(command,folder,label);assert code==0,str(folder/(label+'.stdout.log'))
    return command

capture=runroot/'capture.exe'
build([runroot/'capture.cs'],capture,['mscorlib','System','System.Core'],runroot,'capture-build')

def read_capture(path):
    """Decode the fixture's counted base64 arguments, preserving empty values."""
    if not path.exists():return []
    rows=[]
    for line in path.read_text(encoding='utf-8-sig').splitlines():
        cwd,count,raw=line.split('\t',2);count=int(count)
        args=[] if count==0 else [base64.b64decode(s).decode('utf-8') for s in raw.split(',')]
        assert len(args)==count
        rows.append(dict(cwd=base64.b64decode(cwd).decode('utf-8'),args=args))
    return rows

records=[]
for target in ['kitaq'+options.target]:
    folder=runroot/target;folder.mkdir()
    compiler=options.compiler.resolve()
    checks=[]
    def check(name,passed,**details):
        """Keep failures in the report instead of skipping unsuccessful cases."""
        checks.append(dict(name=name,passed=bool(passed),**details))
    env=os.environ.copy();capture_log=folder/'quoted-args.txt';env['KITAQ_TEST_CAPTURE']=str(capture_log)
    exit_code=run([capture,'--probe',compiler],folder,'quote-probe',env)
    actual=read_capture(capture_log)
    check('windows-argument-roundtrip',exit_code==0 and len(actual)==1 and actual[0]['args']==CASES,actual=actual,expected=CASES)

    # The legacy replay receives only benign controlled arguments. Its two
    # commands invoke capture fixtures, not a real build or arbitrary shell text.
    cwd_a=folder/'compile path';cwd_b=folder/"test path's $folder"
    cwd_a.mkdir();cwd_b.mkdir()
    for cwd in [cwd_a,cwd_b]:shutil.copy2(capture,cwd/(target+'.exe'))
    args_a=['source $value.c','-o','result path.bin',"--label=a'b",'--path=C:\\folder with space\\']
    args_b=['test','--case=second']
    history=folder/'history.log'
    history.write_text('2026-09-14T00:00:00Z\t'+str(cwd_a)+'\t'+subprocess.list2cmdline(args_a)+'\n'+
                       '2026-09-14T00:00:01Z\t'+str(cwd_b)+'\t'+subprocess.list2cmdline(args_b)+'\n',encoding='utf-8')
    recipe=folder/'recipe.ps1'
    code=run([compiler,'recipe','--history='+str(history),'--out='+str(folder/'recipe.md'),'--script='+str(recipe)],folder,'recipe-generate')
    check('recipe-generation',code==0)
    env['KITAQ_TEST_CAPTURE']=str(folder/'recipe-args.txt')
    code=run(['powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',recipe],folder,'recipe-run',env)
    actual=read_capture(folder/'recipe-args.txt')
    check('recipe-literal-arguments',code==0 and len(actual)==2 and [r['args'] for r in actual]==[args_a,args_b],actual=actual)
    check('recipe-per-command-directory',len(actual)==2 and [r['cwd'] for r in actual]==[str(cwd_a),str(cwd_b)])

    # A failed first command must prevent later replay steps and propagate its status.
    history=folder/'failed-history.log'
    history.write_text('2026-09-14T00:00:00Z\t'+str(cwd_a)+'\tsource.c --fail=7\n'+
                       '2026-09-14T00:00:01Z\t'+str(cwd_b)+'\ttest\n',encoding='utf-8')
    code=run([compiler,'recipe','--history='+str(history),'--out='+str(folder/'failed-recipe.md'),
              '--script='+str(folder/'failed-recipe.ps1')],folder,'failed-recipe-generate')
    assert code==0
    env['KITAQ_TEST_CAPTURE']=str(folder/'failed-recipe-args.txt')
    code=run(['powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',folder/'failed-recipe.ps1'],folder,'failed-recipe-run',env)
    check('recipe-stops-on-failure',code==7 and len(read_capture(folder/'failed-recipe-args.txt'))==1,exit_code=code)

    # Real compiler invocations prove bounded devserver exit status separately
    # from the harmless argument-recording fixture used for generated replay.
    (folder/'broken.c').write_text('void main( {\n',encoding='utf-8')
    (folder/'valid.c').write_text('void main() { while(1) { } }\n',encoding='utf-8')
    suffix='.gb' if target=='kitaqgb' else '.nes'
    code=run([compiler,'valid.c','-o','direct'+suffix,'--fast-build','--no-cache'],folder,'direct-valid-build')
    check('direct-valid-build',code==0 and (folder/('direct'+suffix)).is_file(),exit_code=code)
    for label,option,source,expect_success in [('once-fails','--once','broken.c',False),
            ('max-builds-fails','--max-builds=1','broken.c',False),('once-succeeds','--once','valid.c',True)]:
        code=run([compiler,'devserver',option,source,'-o','out'+suffix,'--no-cache'],folder,label)
        check(label,(code==0)==expect_success,exit_code=code)
    records.append(dict(target=target,compiler=str(compiler),compiler_sha256=hashlib.sha256(compiler.read_bytes()).hexdigest(),checks=checks))

artifacts={str(p.relative_to(runroot)):hashlib.sha256(p.read_bytes()).hexdigest() for p in runroot.rglob('*') if p.is_file()}
report=dict(phase=phase,directory=str(runroot),targets=records,artifacts=artifacts,
    test_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    scope='Supplied real compiler for recipe generation and bounded compile/devserver calls; controlled native argument-capture fixture for replay and QuoteArg transport. No emulator/hardware claim.')
(runroot/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps(dict(report=str(runroot/'report.json'),targets=[dict(target=r['target'],checks=len(r['checks']),
    failures=[c['name'] for c in r['checks'] if not c['passed']]) for r in records])))

sys.exit(1 if any(not c['passed'] for r in records for c in r['checks']) else 0)
