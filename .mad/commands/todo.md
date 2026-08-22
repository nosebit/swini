# MAD Todo Command

This set of instructions is used to break a feature's approved `plan.md` into a
`todo.md` list of concrete, atomic tasks that a human engineer or an AI agent
can pick up and execute one at a time.

<!----->

## Command Rules

- Every task must be atomic: small enough to implement and verify in one
  sitting, with an unambiguous Definition of Done.
- Every task must reference the specific `plan.md` section/snippet it implements
  — never invent scope not present in the plan. If breaking the plan down
  reveals it needs scope the plan doesn't cover, flag that to the user rather
  than silently adding a task for it.
- Order tasks by dependency, and call out which tasks can run in parallel.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load context
- [ ] Step 2: Detect the feature
- [ ] Step 3: Read the plan
- [ ] Step 4: Draft `todo.md`
- [ ] Step 5: Review and iterate
- [ ] Step 6: Write file
- [ ] Step 7: Wrap up
```

<!----->

### Step 1 — Load Context

Read `.mad/rules.md` if it is not already in your context.

<!----->

### Step 2 — Detect the Feature

Determine which `specs/<nnnn>-<feature-slug>/` folder you are dealing with:

- If a feature slug can be inferred from context — the user's message names it
  explicitly, or it can be read off the current git branch name — look for a
  `specs/*-<feature-slug>/` folder matching that slug. If none exists, stop and
  ask the user to run `/mad.plan` first.
- If no feature slug can be inferred from context, stop and ask the user which
  feature they mean.

Remember the exact folder name found so it can be reused in other steps.

If a `todo.md` already exists in that folder, read it, present a summary to the
user, and ask what needs to change. Skip to Step 4 with the existing content.
Otherwise move to Step 3.

<!----->

### Step 3 — Read the Plan

Read `plan.md` in the detected folder. If it doesn't exist, stop and ask the
user to run `/mad.plan` first.

Identify every concrete component in its "Implementation Details" and "Data
Model / API Changes" sections — each will become one or more tasks.

<!----->

### Step 4 — Draft `todo.md`

Silently read `.mad/templates/todo.md` to understand the required structure.

For each component in the plan, create one task with:

- a stable id (`T1`, `T2`, ...),
- a link back to the `plan.md` section it implements,
- the concrete files it will add or modify,
- a description,
- a Definition of Done a reviewer could check mechanically,
- its dependencies on other task ids (or "none").

Order the list so dependencies come before dependents. Every task that touches
code must include passing the project's
[Post-Write Checklist](../memory/lore.md) as part of its Definition of Done, in
addition to task-specific criteria.

<!----->

### Step 5 — Review and Iterate

Show the draft to the user. Ask: _"Does this break the plan down correctly?
Anything to split, merge, or reorder?"_

If the user requests changes, update the draft and show it again. Repeat until
the user explicitly approves.

**Done when:** User says the draft is approved.

<!----->

### Step 6 — Write File

Write the approved `todo.md` to `todo.md` in the exact folder detected in
Step 2.

<!----->

### Step 7 — Wrap Up

Summarize what was created and suggest next steps:

- Open a PR with the new `todo.md` for review, OR
- Run `/mad.exec` to start implementing the tasks.
