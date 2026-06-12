const MANAGED_GUIDANCE_START: &str = "<!-- codex1-managed setup guidance start -->";
const MANAGED_GUIDANCE_END: &str = "<!-- codex1-managed setup guidance end -->";

pub(super) fn body() -> &'static str {
    r#"# Codex1 Setup Guidance

codex1-managed

Codex1 is enabled in this repository as a local artifact workflow convention. Use `$clarify` for product discovery and `$create-prd` to synthesize known context into `PRD.md` for PRD-backed product missions. Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` for the repo-local workflow, domain, ADR, and artifact rules. Use `codex1 setup` for repo-local guidance and `codex1 init` for path-safe mission scaffolding. Use native `/goal` for persistent objectives and continuation. For execution, work directly from the PRD and current repo evidence, choosing lane skills such as `$tdd`, `$diagnose`, or `$improve-codebase-architecture` only when they fit the task.

Codex remains the semantic judge. Codex1 setup status and init output are not readiness, completion, review, proof, closeout, or native goal state.
"#
}

pub(super) fn managed_block() -> String {
    format!(
        "{MANAGED_GUIDANCE_START}\n{}{MANAGED_GUIDANCE_END}\n",
        body()
    )
}

pub(super) fn has_managed_block(text: &str) -> bool {
    text.contains(MANAGED_GUIDANCE_START) && text.contains(MANAGED_GUIDANCE_END)
}

pub(super) fn replace_block(text: &str, replacement: &str) -> Option<String> {
    let start = text.find(MANAGED_GUIDANCE_START)?;
    let after_start = start + MANAGED_GUIDANCE_START.len();
    let relative_end = text[after_start..].find(MANAGED_GUIDANCE_END)?;
    let mut end = after_start + relative_end + MANAGED_GUIDANCE_END.len();
    if text[end..].starts_with('\n') {
        end += 1;
    }
    let mut next = String::new();
    next.push_str(&text[..start]);
    next.push_str(replacement);
    next.push_str(&text[end..]);
    Some(next)
}

pub(super) fn remove_block(text: &str) -> Option<String> {
    let start = text.find(MANAGED_GUIDANCE_START)?;
    let after_start = start + MANAGED_GUIDANCE_START.len();
    let relative_end = text[after_start..].find(MANAGED_GUIDANCE_END)?;
    let mut end = after_start + relative_end + MANAGED_GUIDANCE_END.len();
    if text[end..].starts_with('\n') {
        end += 1;
    }
    let mut edited = String::new();
    edited.push_str(&text[..start]);
    edited.push_str(&text[end..]);
    while edited.contains("\n\n\n") {
        edited = edited.replace("\n\n\n", "\n\n");
    }
    Some(edited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guidance_block_round_trips_with_neighboring_text() {
        let text = "before\n\n<!-- codex1-managed setup guidance start -->\nold\n<!-- codex1-managed setup guidance end -->\nafter\n";
        let replacement = managed_block();
        let replaced = replace_block(text, &replacement).unwrap();
        assert!(replaced.contains("before"));
        assert!(replaced.contains("after"));
        assert!(replaced.contains("native `/goal`"));
        let removed = remove_block(&replaced).unwrap();
        assert!(removed.contains("before"));
        assert!(removed.contains("after"));
        assert!(!removed.contains("codex1-managed setup guidance start"));
    }
}
