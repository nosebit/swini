# MAD Goal Command

This set of instructions is used to generate a well structured `goal.md` spec
file containing the goal of a new feature the user wants to implements.

<!----->

## Command Rules

- Keep `goal.md` product-focused. If the user mentions implementation details,
  capture them in `## Constraints`, not `## Requirements`.

<!----->

## Workflow

Copy this checklist and track your progress:

```
- [ ] Step 1: Load context
- [ ] Step 2: Detect the feature
- [ ] Step 3: Interview user
- [ ] Step 4: Draft `goal.md`
- [ ] Step 5: Review and iterate
- [ ] Step 6: Confirm spec path
- [ ] Step 7: Write file
- [ ] Step 8: Wrap up
```

<!----->

### Step 1 — Load Context

Read `.mad/rules.md` if it is not already in your context.

<!----->

### Step 2 — Detect the feature

Determine which `specs/<nnnn>-<feature-slug>/` folder (if any) you're dealing
with. **This detection is internal** - do not announce which mode you picked or
why. Just proceed silently to the step it points to.

- If a feature slug can be inferred from context — the user's message names it
  explicitly, or it can be read off the current git branch name — look for a
  `specs/*-<feature-slug>/` folder matching that slug. If it exists and has
  a `goal.md`, read it, present a summary to the user and ask them what needs to
  be changed. Remember the exact folder name so it can be reused in other
  steps, then skip to Step 4 with the existing content.
- If no feature slug can be inferred from context, or no folder matches the
  inferred slug, assume you are dealing with a completely new feature and move
  to Step 3.

<!----->

### Step 3 — Interview User

Silently read `.mad/templates/goal.md` to understand the required structure of
the final `goal.md`.

Your entire message — with nothing before or after it, no lead-in, no
justification — must be exactly:

> What's the goal of the feature you want to build? Describe it in your own
> words and I'll follow up with a few questions.

Then use the sections and comments defined in the template to guide the rest of
the interview. Identify the top 2–3 unanswered questions needed to fulfill the
template sections and ask them. Repeat until you have enough information to
draft the full document.

**Done when:** You have gathered enough context to accurately fill out every
section of the `goal.md` template without guessing.

<!----->

### Step 4 — Draft `goal.md`

Generate a draft using `.mad/templates/goal.md` as the structure.

<!----->

### Step 5 — Review and Iterate

Show the draft to the user. Ask: _"Does this capture what you had in mind? What
needs to change?"_

If the user requests changes, update the draft and show it again. Repeat until
the user explicitly approves.

**Done when:** User says the draft is approved.

<!----->

### Step 6 — Confirm Spec Path

If this is an existing feature being updated, skip this step and reuse the
exact folder name detected in Step 2.

If this is a new feature:

- If the user already gave a spec name or number for the feature (e.g. they
  want to use a GitHub issue id), propose `specs/{their-id}-{feature-slug}/`
  using it.
- Otherwise, look at the existing `specs/*/` folders, pick the next unused
  sequential number (zero-padded to 4 digits, e.g. `0001`, `0002`, ...), and
  propose `specs/{next-number}-{feature-slug}/`.

Either way, ask the user to confirm the proposed path or replace it entirely
(e.g. with a GitHub issue id) — whatever is confirmed here is permanent, there
is no renaming step later.

**Done when:** The spec folder path is settled (confirmed by the user for a new
feature, or carried over from Step 2 for an existing one).

<!----->

### Step 7 — Write File

Write the approved `goal.md` to the confirmed path.

<!----->

### Step 8 — Wrap Up

Summarize what was created. Suggest next steps:

- Open a PR with the new `goal.md` for community review, OR
- Run `/mad.plan` to generate the technical plan.
