// Local, independent compatibility check using an unmodified upstream checkout.
// node script path/to/typescript.js path/to/gb-studio path/to/gbseq-cli
const fs=require("node:fs"),path=require("node:path"),Module=require("node:module");
const assert=require("node:assert/strict"),cp=require("node:child_process"),os=require("node:os");
const ts=require(path.resolve(process.argv[2])),src=path.resolve(process.argv[3],"src"),cli=path.resolve(process.argv[4]);
const resolve=Module._resolveFilename;
Module._resolveFilename=function(id,parent,...rest){
    if(id==="consts"||id.startsWith("shared/")) id=path.join(src,id);
    return resolve.call(this,id,parent,...rest);
};
require.extensions[".ts"]=(m,file)=>m._compile(ts.transpileModule(fs.readFileSync(file,"utf8"),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,esModuleInterop:true}}).outputText,file);
const {loadUGESong,exportToC}=require(path.join(src,"shared/lib/uge/ugeHelper.ts"));
const {parseMod}=require(path.join(src,"shared/lib/uge/mod2uge/parseMod.ts"));
const {convertMODDataToUGESong}=require(path.join(src,"shared/lib/uge/mod2uge/import.ts"));
const dir=fs.mkdtempSync(path.join(os.tmpdir(),"gbseq-studio-proof-"));
const run=(...args)=>cp.execFileSync(cli,args,{encoding:"utf8"});
let count=0;
for(const players of [1,2,4]){
  const tmp=path.join(dir,"case"+players);fs.mkdirSync(tmp);const song=path.join(tmp,"fixture.gbseq");
  run("new",song,"--players",String(players),"--bars","5","--bpm","150","--no-loop","--title","音色テスト");
  const p=JSON.parse(fs.readFileSync(song,"utf8"));
  p.wave_slots[7].source=128;
  p.user_waves=[{name:"Test ramp",samples:Array.from({length:32},(_,i)=>(i*3)%16)}];
  fs.writeFileSync(song,JSON.stringify(p));
  for(let i=0;i<players*4;i++){
    run("add-note",song,"--track",String(i+1),"--start","63","--length","4","--pitch",String(12+i),"--out",song);
  }
  for(const ext of ["mod","uge"]){
    const report=JSON.parse(run("export-"+ext,song,path.join(tmp,"fixture."+ext)));
    assert.equal(report.files.length,players);
    report.files.forEach((file,i)=>{
      const b=fs.readFileSync(file),u=ext==="uge"?loadUGESong(b):convertMODDataToUGESong(b);
      assert.equal(u.sequence.length,3);
      if(ext==="mod"){
        const raw=parseMod(b);assert.equal(raw.mkMagic,"M.K.");assert.equal(raw.songLen,3);
        assert.ok(raw.samples[1].sampleLength>0);
      }else{
        assert.equal(u.name,"音色テスト"+(players>1?" P"+(i+1):""));
        assert.deepEqual(Array.from(u.waves[0]),p.user_waves[0].samples);
        assert.equal(u.ticksPerRow,6);
      }
      for(let ch=0;ch<4;ch++){
        const start=u.patterns[u.sequence[0].channels[ch]][63];
        if(ch<3) assert.equal(start.note,12+i*4+ch);else assert.notEqual(start.note,null);
        assert.notEqual(start.instrument,null);
        assert.equal(u.patterns[u.sequence[1].channels[ch]][3].effectCode,14);
      }
      assert.ok(exportToC(u,"gbseq_fixture").includes("gbseq_fixture"));count++;
    });
  }
}
console.log(JSON.stringify({passed:true,files:count,upstream:cp.execFileSync("git",["-C",process.argv[3],"rev-parse","HEAD"],{encoding:"utf8"}).trim(),dir}));
