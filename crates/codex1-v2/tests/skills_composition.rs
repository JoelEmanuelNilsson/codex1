//! Wave 4 acceptance: every repo-local skill file exists and contains the
//! minimum sections the parent needs to run them.

use std::fs;
use std::path::Path;

const SKILLS: &[&str] = &[
    "clarify",
    "plan",
    "execute",
    "review-loop",
    "close",
    "autopilot",
];

fn skill_path(name: &str) -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    Path::new(&manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".claude/skills")
        .join(name)
        .join("SKILL.md")
}

#[test]
fn every_wave4_skill_exists() {
    for name in SKILLS {
        let p = skill_path(name);
        assert!(p.is_file(), "missing skill file: {}", p.display());
    }
}

#[test]
fn every_skill_has_frontmatter_with_name_and_description() {
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        assert!(content.starts_with("---\n"), "{name}: frontmatter missing");
        let fm_end = content[4..].find("---\n").expect("frontmatter close");
        let fm = &content[4..4 + fm_end];
        assert!(
            fm.contains(&format!("name: {name}")),
            "{name}: frontmatter lacks `name: {name}`"
        );
        assert!(
            fm.contains("description:"),
            "{name}: frontmatter lacks description"
        );
    }
}

#[test]
fn every_skill_documents_cli_commands_it_drives() {
    // Round 3 Fix 1: skills must invoke the V2 binary via the resolved
    // `$CODEX1` environment variable, not bare `codex1` (which collides
    // with the pre-existing V1 support CLI on many machines). The test
    // greps for `"$CODEX1" <subcommand>` so any regression to a bare
    // `codex1` invocation fails here.
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        let required_commands: &[&str] = match *name {
            "clarify" => &[r#""$CODEX1" init"#, r#""$CODEX1" validate"#],
            "plan" => &[r#""$CODEX1" plan"#, r#""$CODEX1" plan graph"#],
            "execute" => &[
                r#""$CODEX1" task start"#,
                r#""$CODEX1" task finish"#,
                r#""$CODEX1" parent-loop"#,
            ],
            "review-loop" => &[
                r#""$CODEX1" review open"#,
                r#""$CODEX1" review submit"#,
                r#""$CODEX1" review close"#,
            ],
            "close" => &[
                r#""$CODEX1" parent-loop pause"#,
                r#""$CODEX1" parent-loop resume"#,
            ],
            "autopilot" => &[
                r#""$CODEX1" parent-loop activate"#,
                r#""$CODEX1" mission-close check"#,
                r#""$CODEX1" mission-close complete"#,
                r#""$CODEX1" status"#,
            ],
            _ => unreachable!(),
        };
        for cmd in required_commands {
            assert!(
                content.contains(cmd),
                "{name}: SKILL.md must document {cmd}"
            );
        }
    }
}

#[test]
fn every_skill_has_the_binary_resolver_preamble() {
    // Round 3 Fix 1: each skill must set $CODEX1 via the callable
    // resolver before invoking any subcommand. This guards against a
    // skill slipping through with bare `codex1` commands.
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        assert!(
            content.contains("scripts/resolve-codex1-bin"),
            "{name}: SKILL.md must reference scripts/resolve-codex1-bin"
        );
        assert!(
            content.contains(r#"CODEX1="$("#),
            "{name}: SKILL.md must set CODEX1 via command substitution"
        );
    }
}

#[test]
fn every_skill_lists_stop_boundaries() {
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        assert!(
            content.contains("## Stop boundaries") || content.contains("Stop boundaries"),
            "{name}: SKILL.md must have a Stop boundaries section"
        );
    }
}
