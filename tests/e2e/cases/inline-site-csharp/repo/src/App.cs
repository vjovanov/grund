/**
 * Alpha keeps the basket total and the rounding rule that guards it.
 *
 * The rounding this file applies is the one §FS-001-alpha fixes, and this file doc comment runs past a hundred columns.
 * Every member below documents itself the same way.
 */
namespace Alpha;

public static class App
{
    /// Returns the basket total in minor units.
    ///
    /// Rounding follows the rule §FS-001-alpha states, so a caller never sees
    /// a half-cent, and this item doc comment runs four lines on purpose.
    public static int Total(int[] items)
    {
        var running = 0;
        // The accumulator is widened before the loop because the overflow
        // rule that §FS-001-alpha fixes forbids an intermediate wrap, and
        // this note runs four lines, so the cap reports it at its opener.
        // The block sits among statements, which is what makes it a note.
        foreach (var item in items)
        {
            running += item;
        }
        return running;
    }

    // This is a plain slash run directly above a definition, which C# does not
    // call documentation, so the rule §FS-001-alpha fixes still measures it:
    // the marker decides, not the position, and four lines is one over the cap.
    // Only a triple-slash run or a doc block comment is documentation here.
    public static int Rounded(int amount)
    {
        var step = 5;
        // Rounding to the nearest step is what §FS-001-alpha requires here.
        return ((amount + step - 1) / step) * step;
    }
}
