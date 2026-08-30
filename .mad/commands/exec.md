# MAD Exec Command

This set of instructions is used to implement the tasks listed in an approved
`todo.md`, one at a time, and to keep `plan.md`/`todo.md` honest about what was
actually built.

<!----->

## Command Rules

- By default, implement all eligible tasks back to back, in dependency order,
  in one run — no per-task check-in. Only pause when a task turns out to be
  blocked, ambiguous, or needs scope `plan.md` doesn't cover, or when Step 6
  needs approval for a `plan.md` deviation diff. If the user asks for tighter
  oversight instead (e.g. stop after every task, or after a specific one),
  follow that.
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

- If the user already asked for a specific task, or for tighter oversight
  (see Command Rules), follow that instead of the default below.
- Otherwise, pick the first eligible task in `todo.md`'s list order — it
  already reflects the intended sequence. The first time this step runs in
  a session, briefly state the full run before starting: which tasks are
  eligible and the order you'll work through them. On every task, name which
  one you're starting, then proceed — don't stop to ask.

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

<!----->

### Step 6 — Reconcile Spec Docs

Flip the task's checkbox to `[x]` and write the update to `todo.md`. This is
routine progress tracking and doesn't need its own confirmation beyond the
task approval already given in Step 4.

If Step 5 recorded deviations for this task, append them to
`specs/<feature-slug>/wrap.md` as a new section, using
`.mad/templates/wrap.md`'s structure (create the file from that template if
it doesn't exist yet). This is also routine bookkeeping — no confirmation
needed, and the file is never rewritten, only appended to.

Then update the relevant section of `plan.md` to match what was actually
built (e.g. swap in the real snippet, note the added file) — treat this like
any other draft change: show the diff and get approval before writing. If the
user rejects the change, leave `plan.md` as-is — the deviation is preserved
in `wrap.md` either way, so nothing is lost. Record which way it went in the
`wrap.md` entry (see the template).

<!----->

### Step 7 — Repeat or Wrap Up

If tasks remain, go back to Step 3 automatically — don't stop to ask, unless
the user has asked for tighter oversight (see Command Rules).

Otherwise, summarize:

- Tasks completed this session and what changed.
- Any deviations recorded this session — see `specs/<feature-slug>/wrap.md`
  for the full log — and whether `plan.md` was updated to match or left
  as-is.
- Remaining unchecked tasks, if any.
- Suggested next step: open a PR.
