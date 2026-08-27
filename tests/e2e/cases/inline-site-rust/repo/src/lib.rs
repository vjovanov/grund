//! Alpha keeps the basket total and the rounding rule that guards it.
//!
//! The rounding this module applies is the one §FS-001-alpha fixes, and this module doc runs past a hundred columns.
//! Every item below documents itself the same way.

/// Returns the basket total in minor units.
///
/// Rounding follows the rule §FS-001-alpha states, so a caller never sees a
/// half-cent, and this item doc comment runs four lines on purpose.
pub fn total(items: &[u32]) -> u32 {
    let mut running = 0;
    // The accumulator is widened before the loop because the overflow
    // rule that §FS-001-alpha fixes forbids an intermediate wrap, and
    // this note runs four lines, so the cap reports it at its opener.
    // The block sits among statements, which is what makes it a note.
    for item in items {
        running += *item;
    }
    running
}

// This is a plain slash run directly above a definition, which Rust does not
// call documentation, so the rule §FS-001-alpha fixes still measures it: the
// marker decides, not the position, and four lines is one over the default cap.
// Only a triple-slash or a module doc run is documentation here.
pub fn rounded(amount: u32) -> u32 {
    let step = 5;
    // Rounding to the nearest step is what §FS-001-alpha requires here.
    amount.div_ceil(step) * step
}

//// A four-slash run is a rule drawn across the file, not documentation, so
//// the rule that §FS-001-alpha fixes measures this block the way it measures
//// any other note: rustc renders none of it, and four lines is one line over
//// the cap that the default configuration sets.
pub const STEP: u32 = 5;
