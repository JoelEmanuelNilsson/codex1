---
name: handoff
description: Compact the current conversation into a handoff document for another agent to pick up.
argument-hint: "What will the next session be used for?"
---

Write a handoff document summarising the current conversation so a fresh agent can continue the work. Save to the temporary directory of the user's OS - not the current workspace.

Include a "suggested skills" section in the document, which suggests skills that the agent should invoke.

Do not duplicate content already captured in other artifacts (PRDs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.

Redact any sensitive information, such as API keys, passwords, or personally identifiable information.

If the user passed arguments, treat them as a description of what the next session will focus on and tailor the doc accordingly.

Before writing, gather enough context to make the handoff useful. Prefer brief `rg`, `git status`, `ls`, and targeted file reads over memory alone when the conversation references local artifacts, source files, docs, skills, plugins, branches, or generated outputs. Verify that important paths exist when practical.

The handoff must preserve the shape of the work, not just the headline. Include:

- **Purpose for the next session**: what the next agent is supposed to do and what kind of output the user wants.
- **Latest user intent**: the most recent steering from the user, especially corrections, constraints, or "do not do X" instructions.
- **Non-decisions and locked decisions**: clearly distinguish choices that are still open from facts or constraints that are already settled.
- **Primary artifacts**: absolute paths or URLs the next agent should read first. Reference existing artifacts instead of copying their contents.
- **Key facts already established**: short evidence-backed bullets with source paths or URLs when available.
- **Open questions / research branches**: what the next agent still needs to inspect or prove.
- **Suggested skills and plugins**: name the likely skills/plugins and why they matter.
- **Suggested first steps**: concrete, ordered startup actions for the next agent.
- **Prompt for the next session**: when the user asks for a new session handoff, include a ready-to-paste prompt that references the key artifacts, skills/plugins, constraints, and expected discussion/output.
- **Sensitive data note**: state that secrets were redacted or that no secrets are included.

Do not flatten a complex exploration into a vague summary. If the next agent needs low-level understanding, explicitly tell it which source files, docs, or APIs to inspect before discussing conclusions. If the user says not to lock decisions, repeat that in the handoff and in the new-session prompt.

If an existing artifact already contains a detailed map, plan, PRD, ADR, or notes file, the handoff should point to it and explain how to use it. It should not paste large duplicate diagrams or plans into the handoff.
