# Blueprint Standard

## Context

Architectural documents must be easy for humans to write and read, and simultaneously optimized for an LLM to process accurately. Without a standard the knowledge base becomes inconsistent over time. A fixed section structure ensures every blueprint answers the same questions in the same order.

## Decision

All blueprints use plain Markdown with no frontmatter. The filename (without extension) is the ID. The first `# heading` is the title. Blueprints are atomic: one decision per document. Cross-reference related blueprints via `[[slug]]` rather than duplicating content.

**Body sections in order:**
- `## Context` — the problem, constraints, and reasoning. Why this decision makes sense given the specific situation.
- `## Decision` — what we do. One or two declarative sentences plus any reference table or list of rules.
- `## Do` — bullet list of patterns to follow.
- `## Don't` — bullet list of anti-patterns to avoid.
- `## Examples` — code, copy, or other concrete examples. No prose.

## Do

- Use `[[slug]]` double-bracket syntax to reference other blueprints — links are tracked automatically
- Keep blueprints atomic — one decision per document
- Omit sections that genuinely do not apply — a blueprint with no Don'ts needs no Don't section
- Write the Decision section in present tense: "We use X", "X is the only approved Y"
- Name the file after the decision: `ecto-context.md`, not `data-access-patterns.md`

## Don't

- Don't add YAML frontmatter — ID comes from the filename, title from the `# heading`
- Don't use `[slug]` single brackets — they are plain Markdown links, not tracked references
- Don't duplicate content from another blueprint — cross-reference instead
- Don't mix multiple decisions into one blueprint — split them

## Examples

```
# Your Blueprint Title

## Context

Why this decision exists and what problem it solves.

## Decision

What we do. Present tense.

## Do

- Rule one
- Rule two

## Don't

- Anti-pattern one

## Examples

```elixir
# concrete example
```

See also: [[related-blueprint]]
```

See also: [[collaboration-model]]
