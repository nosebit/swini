# MAD Goal Command

This set of instructions is used to generate a well structured `goal.md` spec
file containing the goal of a new feature the user wants to implements.

<!----->

## Command Rules

- Keep `goal.md` strictly product-focused. Its audience is non-technical
  stakeholders (PMs, etc.).
- If the user volunteers an implementation-level detail (a specific library,
  protocol, data structure, algorithm, etc.) with no product-facing shape, do
  not put it anywhere in the visible document — record it in the hidden
  `plan-notes` comment the template defines instead.
- If a remark has both a product-facing implication and an implementation detail
  (e.g. "must interoperate with our existing node protocol, which is
  gRPC-based"), put the product-facing part in Constraints and the specific
  implementation detail in `plan-notes`.

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
  `specs/*-<feature-slug>/` folder matching that slug. If it exists and has a
  `goal.md`, read it, present a summary to the user and ask them what needs to
  be changed. Remember the exact folder name so it can be reused in other steps,
  then skip to Step 4 with the existing content.
- If no feature slug can be inferred from context, or no folder matches the
  inferred slug, assume you are dealing with a completely new feature and move
  to Step 3.

<!----->

### Step 3 — Interview User

Silently read `.mad/templates/goal.md` to understand the required structure of
the final `goal.md`.

If the user has already described the feature — in the message that invoked this
command, or earlier in the conversation — do not ask the canned opening question
below. That description counts even if it mixes in requirements, constraints, or
implementation detail; it all gets sorted into the right place per the Command
Rules above. Just note what's already answered and skip straight to identifying
gaps.

Otherwise, your entire message — with nothing before or after it, no lead-in, no
justification — must be exactly:

> What's the goal of the feature you want to build? Describe it in your own
> words and I'll follow up with a few questions.

Either way, use the sections and comments defined in the template to guide the
rest of the interview. Identify the top 2–3 unanswered questions needed to
fulfill the template sections and ask them. Repeat until you have enough
information to draft the full document.

**Done when:** You have gathered enough context to accurately fill out every
section of the `goal.md` template without guessing.

<!----->

### Step 4 — Draft `goal.md`

Generate a draft using `.mad/templates/goal.md` as the structure, applying the
product-vs-implementation split from the Command Rules. Only append the hidden
`plan-notes` comment the template defines if there's implementation detail to
carry forward — omit it entirely otherwise, never leave an empty placeholder.

<!----->

### Step 5 — Review and Iterate

Show the draft to the user. If a `plan-notes` block was produced in Step 4,
explicitly call out what's in it before asking for approval — e.g. "I've kept
`goal.md` product-focused; these implementation details you mentioned will carry
forward to `/mad.plan` instead: {list}." Then ask: _"Does this capture what you
had in mind? What needs to change?"_

If the user requests changes, update the draft and show it again. Repeat until
the user explicitly approves.

**Done when:** User says the draft is approved.

<!----->

### Step 6 — Confirm Spec Path

If this is an existing feature being updated, skip this step and reuse the exact
folder name detected in Step 2.

If this is a new feature:

- If the user already gave a spec name or number for the feature (e.g. they want
  to use a GitHub issue id), propose `specs/{their-id}-{feature-slug}/` using
  it.
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
