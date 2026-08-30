// Sessions expire after an hour so a stolen cookie has a bounded life,
// and the rotation below keeps a long-lived client alive without
// re-authenticating; the exact hour is a product choice that the spec
// owns rather than this file, see §FS-001-auth.1 and also
// §FS-001-auth.2 for the rotation side.
fn expire() {}
