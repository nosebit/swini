# MAD Init Command

This set of instructions bootstraps the MAD Framework for a project that
hasn't used it yet. It creates the first version of `.mad/memory/lore.md` by
running the `/mad.lore` workflow, and makes sure the supporting folders exist.

<!----->

## Command Rules

- This command is a thin wrapper around `/mad.lore` — do not duplicate its
  interview logic here, follow it.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load rules
- [ ] Step 2: Check current state
- [ ] Step 3: Ensure folders exist
- [ ] Step 4: Run the lore workflow
- [ ] Step 5: Wrap up
```

<!----->

### Step 1 — Load Rules

Read `.mad/rules.md` if it is not already in your context.

<!----->

### Step 2 — Check Current State

Check whether `.mad/memory/lore.md` already exists and has content.

- If it does, tell the user the project is already initialized and ask if
  they want to update the lore instead (in which case, follow
  `.mad/commands/lore.md` starting at its "Check Existing Lore" step).
  Otherwise stop.
- If it doesn't, continue to Step 3.

<!----->

### Step 3 — Ensure Folders Exist

Confirm `.mad/memory/` and `specs/` exist at the project root, creating them
if missing. Creating empty directories does not need confirmation.

<!----->

### Step 4 — Run the Lore Workflow

Follow `.mad/commands/lore.md` starting at its "Interview User" step — its
earlier "Load Rules" and "Check Existing Lore" steps are already satisfied:
rules are loaded, and you already know `lore.md` doesn't exist yet.

<!----->

### Step 5 — Wrap Up

Summarize what was created. Suggest next steps:

- Run `/mad.goal` (or `/mad.spec`) to start specifying the first feature.
