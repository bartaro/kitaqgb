"""Launch the packaged GUI from an unrelated directory and require its receipt."""
from pathlib import Path
import json, os, subprocess, sys, tempfile
folder=Path(sys.argv[1]).resolve()
exe=folder/('gbseq.exe' if os.name=='nt' else 'gbseq')
receipt=folder/'verification/gui-startup.json'
with tempfile.TemporaryDirectory() as cwd:
    result=subprocess.run([str(exe),'--startup-smoke',str(receipt)],cwd=cwd,capture_output=True,timeout=90)
(folder/'verification/gui-startup.log').write_bytes(result.stdout+result.stderr)
assert result.returncode==0,(result.returncode,receipt.read_text(encoding='utf-8') if receipt.exists() else 'no receipt',result.stderr.decode(errors='replace'))
data=json.loads(receipt.read_text(encoding='utf-8'))
assert data['passed'] and data['ticks']>=24 and data['draws']>=4
assert data['language']=='ja' and data['wave_graph'] and data['key_editor']
assert len(set(data['language_menu_draws']))==9
assert data['sram_free_player_export_menu']
assert data['gb_studio_export_menus']==2
assert data['monitor_path_count_solo0based_edit0based']==[[2,None,0],[2,1,0],[4,None,0],[4,3,0]]
assert (data['keyboard_width'],data['note_grid_left'])==(88,116)
assert (data['synthetic_resize_before'],data['synthetic_resize_after'])==(2,6)
assert Path(data['font']).resolve().is_relative_to(folder),'font loaded outside portable package'
print(json.dumps(data,indent=2))
