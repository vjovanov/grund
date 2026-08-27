"""Alpha keeps the basket total and the rounding rule that guards it.

The rounding this module applies is the one §FS-001-alpha fixes, and this module docstring runs past a hundred columns.
Every function below documents itself the same way.
"""


def total(items):
    """Return the basket total in minor units.

    Rounding follows the rule §FS-001-alpha states, so a caller never sees a
    half-cent, and this docstring runs four lines on purpose.
    """
    running = 0
    # The accumulator is widened before the loop because the overflow
    # rule that §FS-001-alpha fixes forbids an intermediate wrap, and
    # this note runs four lines, so the cap reports it at its opener.
    # The block sits among statements, which is what makes it a note.
    for item in items:
        running += item
    return running


# This is a plain hash run directly above a def, which PEP 257 does not call
# documentation, so the rule §FS-001-alpha fixes still measures it: the marker
# decides, not the position, and four lines is one line over the default cap.
# Only a docstring is documentation in Python.
def rounded(amount):
    step = 5
    # Rounding to the nearest step is what §FS-001-alpha requires here.
    return ((amount + step - 1) // step) * step
