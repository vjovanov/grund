/**
 * Alpha keeps the basket total and the rounding rule that guards it.
 *
 * The rounding this file applies is the one §FS-001-alpha fixes, and this file doc comment runs past a hundred columns.
 * Every function below documents itself the same way.
 */

#include <vector>

/// Returns the basket total in minor units.
///
/// Rounding follows the rule §FS-001-alpha states, so a caller never sees a
/// half-cent, and this item doc comment runs four lines on purpose.
int total(const std::vector<int> &items) {
  int running = 0;
  // The accumulator is widened before the loop because the overflow
  // rule that §FS-001-alpha fixes forbids an intermediate wrap, and
  // this note runs four lines, so the cap reports it at its opener.
  // The block sits among statements, which is what makes it a note.
  for (int item : items) {
    running += item;
  }
  return running;
}

// This is a plain slash run directly above a definition, which C++ does not
// call documentation, so the rule §FS-001-alpha fixes still measures it: the
// marker decides, not the position, and four lines is one over the default cap.
// Only a triple-slash run or a doc block comment is documentation here.
int rounded(int amount) {
  const int step = 5;
  // Rounding to the nearest step is what §FS-001-alpha requires here.
  return ((amount + step - 1) / step) * step;
}

/*! The Qt-style doc opener is a doc comment too, so the rule §FS-001-alpha
    fixes never measures this block: it documents the constant below it, and
    four lines of documentation is documentation rather than a note that ran
    one line long. Nothing here is reported. */
const int kStep = 5;
