-- Alpha keeps the basket total and the rounding rule that guards it.
--
-- The rounding this schema applies is the one §FS-001-alpha fixes, and this file header runs past a hundred columns.
-- Every table below documents itself the same way.

-- The basket table holds one row per open cart.
--
-- Its money columns follow the rule §FS-001-alpha states, so no half-cent is
-- ever stored, and this doc comment runs four lines on purpose.
CREATE TABLE basket (
  id INTEGER PRIMARY KEY,
  -- The total is stored in minor units because the rounding rule
  -- that §FS-001-alpha fixes forbids a fractional cent, and this
  -- note runs four lines, so the cap reports it at its opener.
  -- The block sits among column definitions, so it is a note.
  total_minor INTEGER NOT NULL
);

-- This block is separated from the definition below it by one blank line, so
-- it documents nothing, and the rule §FS-001-alpha fixes still measures it:
-- adjacency is what decides here, and four lines is one line over the cap
-- that the default configuration sets.

CREATE TABLE basket_line (
  basket_id INTEGER NOT NULL,
  -- Line amounts round the way §FS-001-alpha requires before an insert.
  amount_minor INTEGER NOT NULL
);
