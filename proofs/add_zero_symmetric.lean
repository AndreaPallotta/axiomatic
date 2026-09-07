-- ========================================================
-- AXIOMATIC AUTONOMOUS THEOREM DISCOVERY: add_zero_symmetric
-- Machine-certified proof synthesized via MCTS & Neurosymbolic Kernel
-- Formally verified by the Lean 4 proof assistant kernel
-- ========================================================

set_option linter.unusedVariables false

theorem add_zero_symmetric (a : Nat) : (a + 0) = (0 + a) := by
  try (conv => lhs; rw [Nat.add_comm])
  try (rw [Nat.add_comm])
  try rfl
  try omega
