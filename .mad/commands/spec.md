# MAD Spec Command

This set of instructions runs the **goal → plan → todo** phases back to back
for a single feature, end-to-end, in one sitting. Each phase still requires
the user to review and approve its document (as each underlying command
already does) before moving on to the next — but there is no separate
"continue or stop?" checkpoint between phases; a document's approval is what
moves the command forward. A user who wants to stop after a single phase
should run that phase's command individually instead of `/mad.spec`. The
**exec** phase is intentionally not included — it writes real code and is
always run separately via `/mad.exec`.

<!----->

## Command Rules

- This command does not duplicate the goal/plan/todo logic — it runs each of
  `.mad/commands/goal.md`, `.mad/commands/plan.md`, and `.mad/commands/todo.md`
  in full, in order, obeying every rule and confirmation gate defined there
  (including their own Step 1, which loads `.mad/rules.md`).
- Once the user approves a phase's document (its own "Review and Iterate"
  step) and it's written, move straight to the next phase. Do not add an
  extra "want to continue?" prompt on top of that approval.
- Carry the detected `<feature-slug>` and spec folder forward across phases so
  the user isn't asked to confirm the same path three times.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Run goal phase
- [ ] Step 2: Run plan phase
- [ ] Step 3: Run todo phase
- [ ] Step 4: Wrap up
```

<!----->

### Step 1 — Run Goal Phase

Follow `.mad/commands/goal.md` in full, except skip its final "Wrap Up" step —
this command's own wrap-up happens at the very end instead.

<!----->

### Step 2 — Run Plan Phase

Follow `.mad/commands/plan.md` in full, except skip its "Detect the feature"
step — the feature folder is already known from the goal phase — and skip its
final "Wrap Up" step.

<!----->

### Step 3 — Run Todo Phase

Follow `.mad/commands/todo.md` in full, except skip its "Detect the Feature"
step — the feature folder is already known from the goal phase — and skip its
final "Wrap Up" step.

<!----->

### Step 4 — Wrap Up

Summarize all three files created (`goal.md`, `plan.md`, `todo.md`) and
suggest running `/mad.exec` to implement the tasks.
