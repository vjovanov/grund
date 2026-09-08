# grund LSP for LSP4IJ

Import this directory as a custom LSP4IJ template:

1. Install the LSP4IJ plugin if IntelliJ prompts for it.
2. Open **Settings | Languages & Frameworks | Language Servers**.
3. Choose **+ | New Language Server**.
4. Select **Import from custom template...**.
5. Choose this generated directory.
6. Create the server and apply it to the project.

Open a file containing a grund citation to verify that diagnostics, hover, and
navigation are available. Regenerate this directory after changing
`[scan].extensions` in `grund.toml`.
