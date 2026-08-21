# MAD Lore Command

This set of instructions is used to create or update the project's lore — the
persistent "constitution" at `.mad/memory/lore.md` that captures the project's
purpose, architecture, conventions, and rules.

<!----->

## Command Rules

- `lore.md` is a **living document** — prefer editing/extending the existing
  content over rewriting it from scratch, unless the user asks for a rewrite.
- Capture conventions as rules an AI agent can follow literally (RFC-2119 style:
  MUST / MUST NOT / SHOULD / MAY), with short code examples for anything about
  code shape.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load rules
- [ ] Step 2: Check existing lore
- [ ] Step 3: Interview user
- [ ] Step 4: Draft `lore.md`
- [ ] Step 5: Review and iterate
- [ ] Step 6: Write file
- [ ] Step 7: Wrap up
```

<!----->

### Step 1 — Load Rules

Read `.mad/rules.md` if it is not already in your context.

<!----->

### Step 2 — Check Existing Lore

Check whether `.mad/memory/lore.md` already exists and has content.

- If it exists and is non-empty, read it and present a short summary to the
  user. Ask what they want to add, change, or remove. Skip to Step 4 once they
  answer, treating the existing content as the base to edit.
- If it does not exist or is empty, this is first-time setup — move to Step 3.

<!----->

### Step 3 — Interview User

Silently read `.mad/templates/lore.md` to understand the required structure.

If the repo already has READMEs, linter configs, or CI files, read them first
and only ask about what you can't determine yourself. Then ask the user, in
small batches:

- What does the project do, and who/what is it for? (feeds the intro)
- Where does the codebase map/architecture already live (a README, a docs
  folder), or should it be summarized here directly?
- What hard rules must code changes follow (language/runtime version, banned
  patterns, import conventions, distribution constraints)?
- What are the comment and testing conventions?
- What commands MUST be run after writing code, and in what order (format, lint,
  build, unit tests, e2e tests)?

**Done when:** You have enough to fill out every section of the template without
guessing.

<!----->

### Step 4 — Draft `lore.md`

Generate a draft using `.mad/templates/lore.md` as the structure. If updating
existing lore, merge the changes into the existing content rather than
discarding untouched sections.

<!----->

### Step 5 — Review and Iterate

Show the draft to the user. Ask: _"Does this accurately capture how the project
works and what future agents should follow? What needs to change?"_

If the user requests changes, update the draft and show it again. Repeat until
the user explicitly approves.

**Done when:** User says the draft is approved.

<!----->

### Step 6 — Write File

Write the approved content to `.mad/memory/lore.md`.

<!----->

### Step 7 — Wrap Up

Summarize what was created or changed. Remind the user that every MAD command
(`/mad.goal`, `/mad.plan`, `/mad.todo`, `/mad.exec`) reads this file for
context, so keep it updated as project conventions evolve.
