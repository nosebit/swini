# MAD Exec Command

This set of instructions is used to implement the tasks listed in an approved
`todo.md`, one at a time, and to keep `plan.md`/`todo.md` honest about what was
actually built.

<!----->

## Command Rules

- Implement **one task at a time**, in dependency order, and stop after each
  for the user to review — unless the user explicitly asks to run several (or
  all) tasks unattended. Even then, stop immediately if a task turns out to be
  blocked, ambiguous, or needs scope `plan.md` doesn't cover.
- Treat `plan.md`'s code snippets as the intended shape, not a copy/paste
  script — adapt them to what you actually find in the codebase, but track
  every place you depart from them (Step 5).
- After writing or modifying code for a task, run the project's
  [Post-Write Checklist](../memory/lore.md) and fix any failure before marking
  the task done.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load context
- [ ] Step 2: Detect the feature
- [ ] Step 3: Select next task
- [ ] Step 4: Implement task
- [ ] Step 5: Track deviations
- [ ] Step 6: Reconcile spec docs
- [ ] Step 7: Repeat or wrap up
```

<!----->

### Step 1 — Load Context

Read `.mad/rules.md` if it is not already in your context.

<!----->

### Step 2 — Detect the Feature

Determine which `specs/<nnnn>-<feature-slug>/` folder you are dealing with:

- If a feature slug can be inferred from context — the user's message names it
  explicitly, or it can be read off the current git branch name — look for a
  `specs/*-<feature-slug>/` folder matching that slug. If none exists, stop
  and ask the user to run `/mad.todo` first.
- If no feature slug can be inferred from context, stop and ask the user which
  feature they mean.

Read `goal.md`, `plan.md`, and `todo.md` from that folder.

<!----->

### Step 3 — Select Next Task

List the tasks in `todo.md` that are unchecked (`[ ]`) and whose dependencies
are all checked (`[x]`).

- If the user already asked for a specific task, use it.
- Otherwise present the eligible tasks and ask which to run next, or whether
  to run all of them back to back with per-task stops only on trouble (see
  Command Rules).

If every task is checked, skip to Step 7.

<!----->

### Step 4 — Implement Task

Implement the selected task's Definition of Done, using its `plan.md`
reference for the intended shape and the codebase's existing conventions for
everything the plan didn't specify.

Run the project's [Post-Write Checklist](../memory/lore.md) and fix any
failure before continuing. Do not mark the task done if any step of it fails.

<!----->

### Step 5 — Track Deviations

Compare what was actually implemented against the task's `plan.md` reference.
Note it as a deviation if any of the following happened:

- A file was added, modified, or removed that wasn't listed in the task.
- A function/type/API ended up with a different shape than the snippet in
  `plan.md`.
- The task's scope was split, merged, or changed during implementation.
- New scope was discovered that isn't covered by any task in `todo.md`.

Keep a running list of deviations across tasks in this session rather than
reporting them one by one — batch them for Step 6.

<!----->

### Step 6 — Reconcile Spec Docs

Flip the task's checkbox to `[x]` and write the update to `todo.md`. This is
routine progress tracking and doesn't need its own confirmation beyond the
task approval already given in Step 4.

If Step 5 recorded deviations for this task, tell the user concretely what
differed from the plan and ask: _"Should I update `plan.md` (and/or
`todo.md`) to reflect what was actually built, leave the docs as-is, or
something else?"_

- If the user wants the docs updated, edit the relevant section of `plan.md`
  (e.g. swap in the real snippet, note the added file) — treat this like any
  other draft change: show the diff, get approval, then write.
- If the user wants to note it without rewriting the original content, append
  a short bullet under a `## Implementation Notes` section at the end of
  `plan.md` describing the deviation instead.

<!----->

### Step 7 — Repeat or Wrap Up

If tasks remain and the user wants to continue, go back to Step 3.

Otherwise, summarize:

- Tasks completed this session and what changed.
- Any deviations reported and how they were resolved (doc updated / noted /
  left as-is).
- Remaining unchecked tasks, if any.
- Suggested next step: open a PR, or — if `todo.md` is now fully checked —
  update the spec's `status` to `done` in its frontmatter.
