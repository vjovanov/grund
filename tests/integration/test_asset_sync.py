"""§FS-init.5 — what the binary embeds is what the tree shows: the `grund-init`
skill under `skills/` is byte-identical to its copy under
`crates/grund-core/assets/`, so neither can drift from the other unnoticed."""

from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]


class SkillAssetSyncTests(unittest.TestCase):
    def test_grund_init_skill_matches_embedded_asset(self):
        repo_skill = REPO_ROOT / "skills" / "grund-init" / "SKILL.md"
        embedded_skill = (
            REPO_ROOT
            / "crates"
            / "grund-core"
            / "assets"
            / "skills"
            / "grund-init"
            / "SKILL.md"
        )

        self.assertEqual(
            repo_skill.read_bytes(),
            embedded_skill.read_bytes(),
            "skills/grund-init/SKILL.md must stay byte-identical to the embedded asset",
        )


if __name__ == "__main__":
    unittest.main()
