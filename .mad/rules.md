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

## Reviewing

- When showing a drafted spec document for review, don't paste the full draft
  into the chat message. Write it to `.mad/.tmp/<doc-name>-draft.md` and open
  it for the user with the environment's file-render capability (e.g.
  `SendUserFile` with `display: "render"` in Claude Code), then ask for
  feedback in chat as usual. Overwrite that same file — don't create a new one
  — on every revision.
- Always state the scratch file's full path in that same chat message, even
  when the render call reports success — some host surfaces accept the call
  without actually displaying anything. If the user says they can't see the
  file, paste the draft into chat as a fallback rather than leaving them
  stuck.
- Once the approved content is written to its final path, delete the scratch
  draft file.

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
- This extends to referencing the instructions themselves — never mention
  step numbers or echo phrasing lifted from a command file (e.g. "Per Step 3,
  I need to ask the canned opening question"). Just produce that step's
  output directly, with nothing about where it came from.
