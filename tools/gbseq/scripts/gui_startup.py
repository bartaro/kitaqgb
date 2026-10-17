"""Launch the packaged GUI from an unrelated directory and require its receipt."""
from pathlib import Path
import json, os, subprocess, sys, tempfile
folder=Path(sys.argv[1]).resolve()
exe=folder/('gbseq.exe' if os.name=='nt' else 'gbseq')
receipt=folder/'verification/gui-startup.json'
with tempfile.TemporaryDirectory() as cwd:
    result=subprocess.run([str(exe),'--startup-smoke',str(receipt)],cwd=cwd,capture_output=True,timeout=90)
(folder/'verification/gui-startup.log').write_bytes(result.stdout+result.stderr)
assert result.returncode==0,result.stderr.decode(errors='replace')
data=json.loads(receipt.read_text(encoding='utf-8'))
assert data['passed'] and data['ticks']>=16 and data['draw_lists']>=2
assert Path(data['font_source']).resolve().is_relative_to(folder),'font loaded outside portable package'
print(json.dumps(data,indent=2))
