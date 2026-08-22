# The MAD Framework

This folder implements the Machine Assisted Development (a.k.a MAD) Framework to
enable spec driven development of features.

The MAD Framework installs a `mad.spec` agentic workflow in the IDE which can be
used to generate a new MAD spec describing a new feature we want to develop.

The feature specification follows a 4 step process:

1. **Goal**: In the first step the MAD Framework instructs the AI Agent to
   iterate with the user until it clearly understand what is the goal of the
   feature we want to implement. This step is meant to capture the product
   aspect of the feature, not the implementation aspect of it. The output of
   this step is a `goal.md` file which is carefully reviewed and refined by a
   product person.

2. **Plan**: After the goal is completely clear and the `goal.md` file is
   approved, the MAD Framework instructs the AI Agent to outline an execution
   plan for the proposed feature. This step is meant to capture the technical
   aspect of the feature we want to deliver. The output of this step is a
   `plan.md` file which is carefully reviewed and refined by a technical person.

3. **To Do**: After the execution plan is approved by the user, the MAD
   Framework instructs the AI Agent to break the `plan.md` into a set of tasks
   an agent will need to do in order to actually implement the plan. The output
   of this step is a `todo.md` file.

4. **Exec**: After the to do list is created, the MAD Framework instructs the AI
   Agent to go through the to do tasks and actually implement them. If
   implementation departs from what `plan.md`/`todo.md` describe, the AI Agent
   reports the deviation and offers to reconcile the spec docs with what was
   actually built.

## Commands

Each phase above is driven by a command in `.mad/commands/`, invoked as
`/mad.<name>`:

| Command       | Produces / does                                                        |
| ------------- | ------------------------------------------------------------------------ |
| `/mad.init`   | Bootstraps a project onto MAD by creating the first `lore.md`.          |
| `/mad.lore`   | Creates or updates `.mad/memory/lore.md`, the project's living "constitution" (purpose, architecture, conventions) that every other command reads for context. |
| `/mad.goal`   | Interviews the user to produce `specs/<slug>/goal.md`.                  |
| `/mad.plan`   | Produces `specs/<slug>/plan.md` from an approved `goal.md`.             |
| `/mad.todo`   | Breaks an approved `plan.md` into `specs/<slug>/todo.md` tasks.         |
| `/mad.spec`   | Runs `/mad.goal` → `/mad.plan` → `/mad.todo` back to back, pausing for approval between each. |
| `/mad.exec`   | Implements the tasks in `todo.md`, one at a time.                       |

Templates for the generated documents live in `.mad/templates/`.

## Rules vs. Lore

Two files are loaded as shared context by (almost) every command, but they
hold different kinds of things:

- **`.mad/rules.md`** — how the AI Agent behaves while running any MAD
  command, in any project (when to ask for confirmation, how to interview,
  never narrating internal steps, etc). Framework-level and static. Every
  command's Step 1 reads it first.
- **`.mad/memory/lore.md`** — what *this* project is (purpose, architecture,
  coding/testing conventions, the post-write checklist). Project-specific and
  grows over time via `/mad.lore`.

Rules that are true for every step of one specific command (e.g. "every
`plan.md` section needs a code snippet") live in that command's own
`## Command Rules` section instead of `.mad/rules.md`.
