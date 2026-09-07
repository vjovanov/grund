import json
from pathlib import Path

# The comment binding is checked too: `1.2e3` (§CONST-field-price.1)
values = json.loads((Path(__file__).parent.parent / "values/runtime.json").read_text())
discount = values["CONST-discount"][0]
