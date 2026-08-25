# FS-pinned: Bare slash-prefixed tokens stay text in non-strict mode

A path-shaped token such as path/FS-pinned in prose must not be silently
promoted to a qualified citation. The marker-prefixed form would cross a
project boundary; the bare form is just text.

This file declares FS-pinned and cites itself again as §FS-pinned so the
check exits clean.
