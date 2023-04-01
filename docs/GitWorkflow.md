# Git - Workflow

## Branches
There are 5 types of branches:
- master
- dev
- release
- feature
- fix

`master` branch contains the newest version of the project.

`dev` branch contains content that is specific to the version that is currently under development.
There is one `dev` branch per version.
> Every `dev` branch must be named with the following syntax:
`dev-{{ MAJOR }}.{{ MINOR }}.{{ PATCH }}`. <br> For example: `dev-0.1.0`.

> When first full project version (one that is not a pre-release) hasn't been created yet, `dev` branch can't exist in repository and `master` should be used as `dev` branch for first project version.

`release` branch contains changes in version that is older than the newest version of the project (one that is currently on `master`).
This branch should be used for patching/developing older versions of the project, that version number change only in patch number.
> Every `release` branch must be named with the following syntax:
`release-{{ MAJOR }}.{{ MINOR }}`. <br> For example: `release-0.1`.

`feature` and `fix` branches contain changes related to the specific issue.
> Name of the `feature` or `fix` branch should be `feature/{{ ISSUE NUMBER }}/{{ NAME }}` or `fix/{{ ISSUE NUMBER }}/{{ NAME }}` where `NAME` part should be as close as possible to the title of the corresponding issue.
Branches must be named using kebab-case e.g.:<br>
`feature/34/different-terrain-version-blocks-matching`<br>
`fix/12/terrain-gaps`

## Commits
> A one-sentence commit message cannot end with a period,
> but a multi-sentence commit message must end with a period.

Commits must be in the imperative present tense and should be preceded by one of these words:
* `Add` - Create a capability e.g. feature, test, dependency.
* `Remove` - Remove a capability e.g. feature, test, dependency.
* `Edit` - Edit a capability e.g. feature, test.
* `Fix` - Fix an issue e.g. bug, typo, accident, misstatement.
* `Bump` - Increase the version of something e.g. dependency.
* `Make` - Change the build process, or tooling, or infra.
* `Refactor` - A code change that must be just a refactoring.
* `Reformat` - Refactor of formatting, e.g. omit whitespace.
* `Optimize` - Refactor of performance, e.g. speed up code.
* `Document` - Refactor of documentation, e.g. help files.

## Workflow
* You cannot commit directly to production branches such as `master`, `dev` or `release`. To make changes to the production branch, you create a `pull request`, which is then assessed and approved by another developer with the appropriate permissions.<br>

* Every feature or fix must follow order of the branches (`->` means pull request and merge):
  `feature` or `fix` -> `dev` -> `master` -> `release`<br>

* As a developer who is merging changes, you are responsible for resolving conflicts and are responsible for possible errors related to your changes.

* You cannot commit changes that contain changes unrelated to your task. For example, replacing the texture in the API development task.

* It is allowed to use commands such as `git rebase`, however, it is not allowed to overwrite commit dates.<br>
To follow that use `--committer-date-is-author-date` parameter.