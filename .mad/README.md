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
   Agent to go through the to do tasks and actually implement them.
