# MAD Plan Command

This set of instructions is used to generate a well structured `plan.md` spec
file containing an technical implemenation proposal of a feature previously
defined in an approved `goal.md`.

<!----->

## Global Rules

- **Never write files** without explicit user confirmation.
- Ask clarifying questions in small batches (2–3 at a time), never as a long
  form.
- `plan.md` is implementation-focused. Do not restate product requirements from
  `goal.md` — reference them instead.
- **Every major implementation section must include a concrete code snippet**
  (function signatures, data structures, interfaces, or short pseudo-code) — not
  just prose. A software engineer unfamiliar with the conversation must be able
  to read `plan.md` and start implementing.
- Follow the project conventions in `.mad/memory/lore.md` (architecture, coding
  rules, testing rules) when proposing code.
- The final `plan.md` must be fully self-contained — someone reading it cold
  should understand both what to build and how, without needing the conversation
  context.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load context
- [ ] Step 2: Detect the feature
- [ ] Step 3: Read the goal
- [ ] Step 4: Explore the codebase
- [ ] Step 5: Interview user on open technical questions
- [ ] Step 6: Draft `plan.md`
- [ ] Step 7: Review and iterate
- [ ] Step 8: Write file
- [ ] Step 9: Wrap up
```

<!----->

### Step 1 — Load Context

Read the `.mad/memory/lore.md` file only if it is not in your context yet.

<!----->

### Step 2 — Detect the feature

Determine the feature stored in `specs/0000-<feature-slug>/` you are dealing
with from the context.

- If such a feature can be determined and a `specs/0000-<feature-slug>/plan.md`
  already exists, then read it, present a summary to the user and ask them what
  needs to be changed. Skip to Step 6 with the existing content. Remember the
  `<feature-slug>` so it can be used in other steps.
- If such a feature cannot be determined, then stop and ask the user to run
  `/mad.goal`.

<!----->

### Step 3 — Read the Goal

Read the `specs/0000-<feature-slug>/goal.md` in case it exists or otherwise stop
and ask the user to run `/mad.goal`.

<!----->

### Step 4 — Explore the Codebase

Identify the parts of the codebase this feature will touch:

- Read the READMEs relevant to the areas the goal implies.
- Look for existing patterns, types, or modules the implementation should reuse
  or follow the conventions of.
- Note the concrete files/modules that will likely be added or modified.

**Done when:** You understand enough of the existing architecture to propose a
concrete, idiomatic technical approach.

<!----->

### Step 5 — Interview User on Open Technical Questions

Read `.mad/templates/plan.md` to understand the required structure of the final
`plan.md` you need to generate.

Using the goal, the template, and what you learned exploring the codebase,
identify the top 2–3 technical decisions that aren't already answered by the
goal or the existing codebase conventions (e.g. data model shape, library
choice, concurrency model, API surface). Ask about those. Do not ask about
anything you can already answer by reading the code.

**Done when:** You have enough information to write concrete code snippets for
every implementation section, without guessing.

<!----->

### Step 6 — Draft `plan.md`

Generate a draft using `.mad/templates/plan.md` as the structure.

For every component being added or changed, include a short, concrete code
snippet — a function signature, struct/type definition, or interface — that
shows precisely what will be implemented. Snippets should be illustrative and
correct in shape (real types, real function names where known) even if not the
final production code.

<!----->

### Step 7 — Review and Iterate

Show the draft to the user. Ask: _"Does this technical approach look right? What
needs to change?"_

If the user requests changes, update the draft and show it again. Repeat until
the user explicitly approves.

**Done when:** User says the draft is approved.

<!----->

### Step 8 — Write File

Write the approved `plan.md` to `specs/<feature-slug>/plan.md`.

<!----->

### Step 9 — Wrap Up

Summarize what was created and suggest next steps:

- Open a PR with `specs/<feature-slug>/plan.md` for technical review, OR
- Run `/mad.todo` to break the plan into a task list.
