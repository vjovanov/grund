/**
 * Alpha keeps the basket total and the rounding rule that guards it.
 *
 * <p>The rounding this file applies is the one §FS-001-alpha fixes, and this file Javadoc runs past a hundred columns.
 * Every method below documents itself the same way.
 */
public final class App {

  /**
   * Returns the basket total in minor units.
   *
   * <p>Rounding follows the rule §FS-001-alpha states, so a caller never
   * sees a half-cent, and this item Javadoc runs four lines on purpose.
   */
  public static int total(int[] items) {
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

  /* This is a plain block comment directly above a definition, which Java
     does not call documentation, so the rule §FS-001-alpha fixes still
     measures it: the opener decides, not the position, and four lines is
     one line over the cap that the default configuration sets. */
  public static int rounded(int amount) {
    int step = 5;
    // Rounding to the nearest step is what §FS-001-alpha requires here.
    return ((amount + step - 1) / step) * step;
  }
}
