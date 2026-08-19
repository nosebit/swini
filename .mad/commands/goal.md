# MAD Goal Command

This set of instructions is used to generate a well structured `goal.md` spec
file containing the goal of a new feature the user wants to implements.

<!----->

## Global Rules

- **Never write files** without explicit user confirmation.
- Ask clarifying questions in small batches (2–3 at a time), never as a long
  form.
- Keep `goal.md` product-focused. If the user mentions implementation details,
  capture them in `## Constraints`, not `## Requirements`.
- The final `goal.md` must be fully self-contained — someone reading it cold
  should understand the feature without needing the conversation context.
- **Never narrate internal steps or decisions.** Do not tell the user which
  step, mode, or checklist item you're on, and do not explain _why_ you're about
  to do something — including paraphrased reasoning like "No existing goal.md
  was found, so I'll start fresh" or "Since you mentioned X, I'll do Y." This
  applies regardless of wording. The user should see only the output each step
  defines, never the routing logic that produced it.

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

Read the `.mad/memory/lore.md` file only if it is not in your context yet:

<!----->

### Step 2 — Detect the feature

Determine the feature stored in `specs/0000-<feature-slug>/` you are dealing
with from the context. **This detection is internal** - do not announce which
mode you picked or why. Just proceed silently to the step it points to.

- If such a feature can be determined and a `specs/0000-<feature-slug>/goal.md`
  already exists, then read it, present a summary to the user and ask them what
  needs to be changed. Remember the `<feature-slug>` so it can be used in other
  steps and skip to Step 4 with the existing content.
- If no feature can be determined, assume you are dealing with a completely new
  feature and move to Step 3.

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

Propose a spec folder path derived from the feature name:
`specs/0000-{feature-slug}/goal.md`

Ask the user to confirm or rename the slug.

Remind the user: _"The `0000` prefix is a placeholder. Rename the folder to
match the PR number once it is opened."_

**Done when:** User confirms the path.

<!----->

### Step 7 — Write File

Write the approved `goal.md` to the confirmed path.

<!----->

### Step 8 — Wrap Up

Summarize what was created. Suggest next steps:

- Open a PR with `specs/0000-{feature-slug}/goal.md` for community review, OR
- Run `/mad.plan` to generate the technical plan.
