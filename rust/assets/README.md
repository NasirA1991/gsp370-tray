# Embedded font subset

`NotoSans-Medium-digits.ttf` is a digits-only subset of **Noto Sans Medium**
from the Noto Project's `latin-greek-cyrillic` family:

- Copyright 2022 The Noto Project Authors
- Source: https://github.com/notofonts/latin-greek-cyrillic
- Licence: SIL Open Font License 1.1 — see `NotoSans-OFL.txt`
- Subset command: `pyftsubset NotoSans-Medium.ttf --text=0123456789 --layout-features=''`
- The upstream OFL declares no Reserved Font Name.

The subset is embedded in the binary so the tray does not depend on a system
font at runtime.
