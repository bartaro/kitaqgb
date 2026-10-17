# GBHUA

## 下载与启动

在[GUI下载](../GUI.md)中选择对应系统的压缩包并完整解压。Windows打开`GBHUA.exe`，保留旁边的`assets`文件夹。Mac将`GBHUA.app`复制到“应用程序”，注意区分Apple Silicon和Intel版本。运行GUI无需安装Rust或Python。程序尚未使用Developer ID签名或公证。请从官方仓库下载；若macOS拦截，确认来源后在“系统设置 > 隐私与安全性”中仅允许此应用，不要关闭系统安全保护。

GUI标签为英文，本页提供中文操作说明。Mac上的`Ctrl`同样指Control键。下方构建命令从仓库根目录开始。CLI默认仍不依赖窗口或GPU。

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA将PNG转换为Game Boy图块、调色板和地图。作者：DAISUKE OBA。原创代码采用MIT许可证。

## 构建与运行

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

启动后尚未编辑的文档不会显示未保存警告，实际编辑后的确认仍然保留。Windows GUI默认不打开控制台。需要诊断时运行 `cargo run -p gbhua --bin gbhua_gui --features gui-console`。

各栏标题和画布顶端已对齐。将鼠标悬停在打开、保存、撤销/重做图标上可查看名称。用Graphic上方的尺寸按钮选择大小，用Map上方的适应窗口、减号、加号调整缩放。右上角可切换DMG/CGB。宽度小于1180px时，左栏使用Tiles / Palettes标签切换。用滚轮或滚动条浏览调色板和诊断信息。Escape关闭菜单。

1. 在左侧选择图块，在Graphic > 8x8中选择7种尺寸之一。大图从所选ID开始使用连续图块。选择颜色并拖动绘制，右键拾色。

2. TOOLS > FILL填充连通区域；TILES > FILL TILE填充整幅图。FLIP X/Y、ROTATE用于变换，EDIT的COPY/PASTE GRAPHIC用于复制。UNDO/REDO最多64步。

3. TOOLS > MAP放置图块，右键拾取图块及调色板。RECT放置3x3，FILL填充区域，ATTR设置调色板并切换X翻转，COLL/EVT切换标记。VIEW选择Fit/1x/2x/4x、网格和标记，用滚轮或滚动条移动。

4. 点击BYTECODE，Ctrl+A后Ctrl+V粘贴16字节。IMPORT应用到首个8x8图块，EXPORT更新文本。方向键/Home/End移动光标，Shift选择，Ctrl+C/X复制/剪切，Backspace/Delete删除相邻字符。

5. FILE > SAVE及EXPORT均询问保存位置。未保存*时，打开/导入/关闭会确认：Yes保存、No丢弃、Cancel继续编辑。PNG导入替换项目，可以撤销。

6. SPRITES > NEW META新建元精灵，用ADD/DUP/CLEAR FRAME和PREV/NEXT FRAME管理帧。METASPRITE中点击添加部件、右键移除。DMG / CGB对比DMG/CGB，不改变已保存调色板。

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

完整项目使用.gbh保存。旧JSON项目只需将扩展名改为.gbh，无需转换内容；不提供旧扩展名加载器。GBTD/GBR/GBTB与GBMB/GBM用于交换，不保留所有项目字段。GBMB导出还会生成同名.gbr，请一起保存。

## PNG / Python / CLI

图片宽高必须为8的倍数（8..2040）。--size使用最近邻缩放。DMG使用4级明暗；CGB近似为RGB555，每图块4色、最多8个调色板。透明区域与白色合成。相同图块共享，超过256种图块会拒绝导入。请检查警告和预览。

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: 简体中文](../README.zh-CN.md)

## PNG导入与故障排查

使用`FILE > IMPORT PNG`或拖入PNG，先选择DMG/CGB。导入会替换当前项目，请先保存。CGB自动调色板按颜色误差选择，以保留少量高光和深色轮廓，而非只保留常见暗色。Game Boy颜色限制仍然存在。旧导入丢失的光泽需重新导入原PNG；已减色的`.gbh`不能恢复丢失的像素。`--size`仅为CLI选项。

选择目标图块，点击BYTECODE内部，输入恰好16字节并按IMPORT。EXPORT只更新文本。地图颜色或方向异常时检查调色板和翻转属性。使用`.gbh`保存完整编辑信息。错误显示于状态栏；取消对话框不会保存。

macOS/Linux依次运行`python3 -m venv .venv`、`source .venv/bin/activate`、`python -m pip install maturin`、`python -m maturin develop --features python`。构建绑定需要Python 3.10以上和Rust。
