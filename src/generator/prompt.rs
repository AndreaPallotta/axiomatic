use crate::verifier::kernel::ProofState;

/// Serializes proof states for LLM consumption
pub fn format_proof_prompt(state: &ProofState) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are an Autonomous Neurosymbolic Prover. Current Proof State:\n");

    for goal in &state.open_goals {
        prompt.push_str(&format!("  [GOAL] #{}: {}\n", goal.id, goal.equality));
    }

    if !state.proof_history.is_empty() {
        prompt.push_str("Applied Steps so far:\n");
        for (i, (tactic, desc)) in state.proof_history.iter().enumerate() {
            prompt.push_str(&format!("  {}. {} ({})\n", i + 1, tactic, desc));
        }
    }

    prompt.push_str("Propose the best formal rewrite tactic or lemma application to simplify and prove the goal.\n");
    prompt
}
