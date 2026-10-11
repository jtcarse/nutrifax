# Issue tracker: GitHub

Issues and specs for this repo live in GitHub Issues at `jtcarse/nutrifax`. Use `gh` for issue operations from this repo.

## Conventions

- Create: `gh issue create --title "..." --body-file <file>`.
- Read: `gh issue view <number> --json number,title,body,labels,comments`.
- List: `gh issue list --state open --json number,title,body,labels,comments`; add label or state filters as needed.
- Comment: `gh issue comment <number> --body-file <file>`.
- Label: `gh issue edit <number> --add-label "..."` or `--remove-label "..."`.
- Close: `gh issue close <number> --comment "..."`.
- Link child issues with GitHub sub-issues when available. Otherwise, put `Part of #<parent>` at the top of the child issue.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Skill terminology

- "Publish to the issue tracker" means create a GitHub issue.
- "Fetch the relevant ticket" means read its GitHub issue.
