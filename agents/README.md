# agents/

The rule pack for AI coding agents that work on a Bylazora migration. Give an
agent these rules and it knows the invariants; the gate still disposes of
whatever it drafts. The agent proposes; the gate disposes.

## What is here

`CLAUDE.md` is the canonical pack. Deliberately one file of plain markdown, not
one file per tool.

## Installing it into your migration repository

Every agent tool reads its instructions from a different path, and those paths
move: `.cursorrules` was Cursor's legacy format and is now `.cursor/rules/*.mdc`,
Copilot reads `.github/copilot-instructions.md`, and `AGENTS.md` has become the
cross-tool convention. Shipping a copy per tool would mean shipping files that
are wrong within a year and disagree with each other in the meantime.

So copy the contents of `CLAUDE.md` into whichever file your tool reads:

| Tool | Commonly read | Notes |
|---|---|---|
| Claude Code | `CLAUDE.md` | at the repository root |
| Most agents | `AGENTS.md` | the emerging cross-tool convention |
| Cursor | `.cursor/rules/*.mdc` | rules can be scoped to a glob; `.cursorrules` is legacy |
| GitHub Copilot | `.github/copilot-instructions.md` | repository-wide, with path-specific instructions also supported |

Check your tool's current documentation rather than trusting this table: it is
the part of this repository most likely to age badly, which is exactly why the
pack is not split per tool.

**If your repository already has one of those files, merge the invariants into
it. Do not replace it.** Your file knows things about your estate that this pack
does not.

## If you are writing your own

The five invariants are the non-negotiable part. Copy them verbatim, because
each one is enforced by the gate rather than being a matter of taste:

1. the validator is the only authority that marks a run proven;
2. money is integer cents, end to end, never float;
3. byte-exact means headers, field widths, ordering, totals rows and line endings;
4. reproducible, or rejected;
5. never weaken the gate to make a run pass.

Everything after that - how the agent works, which tools it calls, the licence
boundary - is yours to fit to the estate. Keep the rule about run records: a
performance number without a recorded run is not a number.

## Keeping it current

The pack names the engine's CLI and MCP tools. If you pin a version, check that
list against `bylazora-core --help` when you upgrade.
