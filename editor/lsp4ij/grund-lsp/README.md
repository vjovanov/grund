# grund LSP for LSP4IJ

Import this directory as a custom LSP4IJ template:

1. Install the LSP4IJ plugin if IntelliJ prompts for it.
2. Open **Settings | Languages & Frameworks | Language Servers**.
3. Choose **+ | New Language Server**.
4. In the **Template** dropdown, choose **Import from custom template...**.
5. Select this directory: `editor/lsp4ij/grund-lsp`.
6. Create the server.

The template starts the workspace copy of `grund-lsp` with:

```sh
cargo run -q -p grund-lsp --
```

That keeps IntelliJ on the current branch while this PR is under review.
