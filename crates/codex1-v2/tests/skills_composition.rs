//! Wave 4 acceptance: every repo-local skill file exists and contains the
//! minimum sections the parent needs to run them.

use std::fs;
use std::path::Path;

const SKILLS: &[&str] = &["clarify", "plan", "execute", "review-loop", "close"];

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
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        let required_commands: &[&str] = match *name {
            "clarify" => &["codex1 init", "codex1 validate"],
            "plan" => &["codex1 plan", "codex1 plan graph"],
            "execute" => &["codex1 task start", "codex1 task finish", "codex1 parent-loop"],
            "review-loop" => &[
                "codex1 review open",
                "codex1 review submit",
                "codex1 review close",
            ],
            "close" => &["codex1 parent-loop pause", "codex1 parent-loop resume"],
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
fn every_skill_lists_stop_boundaries() {
    for name in SKILLS {
        let content = fs::read_to_string(skill_path(name)).unwrap();
        assert!(
            content.contains("## Stop boundaries") || content.contains("Stop boundaries"),
            "{name}: SKILL.md must have a Stop boundaries section"
        );
    }
}
