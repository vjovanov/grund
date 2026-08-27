# Alpha keeps the basket total and the rounding rule that guards it.
#
# The rounding this file applies is the one §FS-001-alpha fixes, and this file header runs past a hundred columns.
# Every method below documents itself the same way.

class Basket
  # Returns the basket total in minor units.
  #
  # Rounding follows the rule §FS-001-alpha states, so a caller never sees a
  # half-cent, and this doc comment runs four lines on purpose.
  def total(items)
    running = 0
    # The accumulator is widened before the loop because the overflow
    # rule that §FS-001-alpha fixes forbids an intermediate wrap, and
    # this note runs four lines, so the cap reports it at its opener.
    # The block sits among statements, which is what makes it a note.
    items.each do |item|
      running += item
    end
    running
  end

  # This block is separated from the definition below it by one blank line, so
  # it documents nothing, and the rule §FS-001-alpha fixes still measures it:
  # adjacency is what decides here, and four lines is one line over the cap
  # that the default configuration sets.

  def rounded(amount)
    step = 5
    # Rounding to the nearest step is what §FS-001-alpha requires here.
    ((amount + step - 1) / step) * step
  end
end
