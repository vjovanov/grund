//! §FS-001-alpha
// grund:fmt off
/// Protected: $$FS-001-alpha, bare FS-001-alpha, shorthand §FS-001.
fn diagram() {}
// grund:fmt on
/// Ordinary: §FS-001-alpha, bare §FS-001-alpha, shorthand §FS-001-alpha.
fn prose() {}
/// A fence drawn in a doc comment is live text, not fence state, so the
/// directive it illustrates toggles like any other:
/// ```text
/// grund:fmt off
/// ```
/// Suppressed by that illustration: $$FS-001-alpha and §FS-001.
// grund:fmt on
/// On again after the region closes: §FS-001-alpha and §FS-001-alpha.
fn illustrated() {}
/* grund:fmt off */
/// Runs to the end of the file: $$FS-001-alpha and §FS-001.
fn tail() {}
