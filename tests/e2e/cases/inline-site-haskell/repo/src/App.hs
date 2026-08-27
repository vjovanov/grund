-- | Alpha keeps the basket total and the rounding rule that guards it.
--
-- The rounding this module applies is the one §FS-001-alpha fixes, and this module header runs past a hundred columns.
-- Every binding below documents itself the same way.
module App (total, rounded) where

-- | Return the basket total in minor units.
--
-- Rounding follows the rule §FS-001-alpha states, so a caller never sees a
-- half-cent, and this doc comment runs four lines on purpose.
total :: [Int] -> Int
total items = sum guarded
  where
    -- The negatives are dropped before the fold because the total rule
    -- that §FS-001-alpha fixes forbids a credit line here, and this note
    -- runs four lines, so the cap reports it at its opening line.
    -- The block sits among bindings, which is what makes it a note.
    guarded = filter (>= 0) items

-- This is a plain dash run directly above a type signature, which Haddock does
-- not call documentation, so the rule §FS-001-alpha fixes still measures it:
-- the marker decides, not the position, and four lines is one over the cap.
-- Only a run opening with a bar or a caret is documentation here.
rounded :: Int -> Int
rounded amount = ((amount + step - 1) `div` step) * step
  where
    -- Rounding to the nearest step is what §FS-001-alpha requires here.
    step = 5
