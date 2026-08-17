// §FS-001-alpha: the canonical form
pub fn alpha() {}

// §FS-001-alpha the note runs straight on with no colon
pub fn beta() {}

// the note comes first and the citation trails it (§FS-001-alpha).
pub fn gamma() {}

// §FS-001-alpha §FS-002-beta: two citations, separated by a space
pub fn delta() {}

// §FS-001-alpha
// the note sits on the next line of the same comment block
pub fn epsilon() {}

/// Walks the alpha table.
/// §FS-001-alpha: one finding per unresolved entry.
pub fn zeta() {}

// §FS-001-alpha, §FS-002-beta
pub fn eta() {}

/// - §FS-001-alpha: a bulleted grounded point
/// 1. §FS-002-beta: an ordered one
pub fn theta() {}

/* §FS-001-alpha: a note that runs past one line and
   still names §FS-002-beta on the way */
pub fn iota() {}

/* §FS-001-alpha: a note that runs past one line and
   §FS-002-beta opens the continuation */
pub fn kappa() {}
