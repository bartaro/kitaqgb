# Third-party notices and license scope

KITAQGB is derived from NORCAL by Keith Holman. Preserve the original
`Copyright 2019 Keith Holman` notice and the MIT permission and disclaimer
in LICENSE when distributing copies or substantial portions of this code.
Upstream: https://github.com/holmak/norcal/blob/master/LICENSE.txt

The MIT declaration covers project-owned code and MIT-licensed inherited code.
It does not grant rights to third-party trademarks, console logo data, ROMs,
BIOS images, fonts, or extracted game assets.

`RomHeaderPatcher.cs` contains the 48-byte Nintendo logo data used in
compatible Game Boy cartridge headers for boot-time validation. As clarified
in the additional scope and trademark notice in LICENSE, including this data
does not assert ownership of Nintendo's logo or trademarks and does not grant
an independent license to any rights Nintendo may hold in them. KITAQGB is
an independent project, not affiliated with or approved by Nintendo.
The scope notice adds no restrictions to the MIT-licensed project code.

The separate support library in `../lib` carries its own LICENSE. Do not
publish the parent development workspace wholesale: it also holds unrelated
applications, private research inputs, caches, and local build outputs.
