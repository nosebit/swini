# MAD Rules

These rules apply to every MAD command (`/mad.*`), at every step, in every
project. Every command's Step 1 points here — command files only state what's
different for that specific command, they don't repeat what's below.

<!----->

## Context

- Read `.mad/memory/lore.md` if it is not already in your context. It carries
  project-specific facts and conventions (architecture, coding rules, testing
  rules, the post-write checklist) that most commands need.

## Writing

- **Never write or overwrite a spec file** (`goal.md`, `plan.md`, `todo.md`,
  `lore.md`) **without showing the user the exact content and getting explicit
  confirmation first.** The one exception is routine progress bookkeeping —
  e.g. flipping a `todo.md` checkbox right after the user already approved
  that task's execution — which doesn't need a second, separate confirmation.
- Never commit or push changes unless the user explicitly asks, regardless of
  how confident you are the work is ready.

## Interviewing

- When asking the user questions to fill out a document, ask in small batches
  (2–3 at a time), never as a long form. Only ask what you can't already
  determine yourself from the codebase, `lore.md`, or the conversation so far.

## Authoring

- Every generated spec document must be fully self-contained: someone reading
  it cold, without the conversation that produced it, must be able to
  understand it — and, for `todo.md`, act on any single task in it — without
  guessing at missing context.

## Communication

- Never narrate internal steps, checklist progress, or routing decisions to
  the user (e.g. "no existing goal.md was found, so I'll start fresh"). The
  user should see only the outputs each step defines — questions, drafts,
  confirmations — never the mechanics that produced them.
