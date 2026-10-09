"""MIT. Completed-display metrics in PPU dots, shared by DMG and CGB.

Input: an array of records with completion_ppu_dots and optional *_dots phases.
Normal library builds contain no markers or measurement overhead.
"""
import argparse,json,statistics
from pathlib import Path
DOTS_PER_FRAME=70224
DOTS_PER_SECOND=4194304

def summarize(records, deadline_frames=4):
    if len(records)<2: raise ValueError("At least two completed frames are required")
    times=[r["completion_ppu_dots"] for r in records]
    intervals=[b-a for a,b in zip(times,times[1:])]
    if deadline_frames<=0 or any(x<=0 for x in intervals):
        raise ValueError("Deadline must be positive and completions strictly increasing")
    phases=sorted(k for k in records[0] if k.endswith("_dots") and k!="completion_ppu_dots")
    phase_stats={}
    for key in phases:
        values=[r[key] for r in records]
        if any(x<0 for x in values):raise ValueError("Negative phase duration: "+key)
        phase_stats[key]={"mean":statistics.mean(values),"median":statistics.median(values),"worst":max(values)}
    return {"completed_frames":len(records),"interval_count":len(intervals),
      "completed_upload_fps":DOTS_PER_SECOND/statistics.mean(intervals),
      "interval_ppu_frames":{"mean":statistics.mean(intervals)/DOTS_PER_FRAME,
        "median":statistics.median(intervals)/DOTS_PER_FRAME,"worst":max(intervals)/DOTS_PER_FRAME},
      "deadline_ppu_frames":deadline_frames,"deadline_misses":sum(x>deadline_frames*DOTS_PER_FRAME for x in intervals),
      "phase_ppu_dots":phase_stats,"hardware_verified":False}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("samples",type=Path)
    p.add_argument("--deadline-frames",type=float,default=4);p.add_argument("--output",type=Path)
    a=p.parse_args();report=summarize(json.loads(a.samples.read_text(encoding="utf-8-sig")),a.deadline_frames)
    text=json.dumps(report,indent=2)
    if a.output:a.output.write_text(text+"\n",encoding="utf-8")
    else:print(text)
if __name__=="__main__":main()
