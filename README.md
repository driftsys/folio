# folio

Reference CLI implementation of the **Repofolio standard** — a tool-agnostic
spec for laying out, governing, and releasing polyglot repositories.

Part of the [driftsys](https://github.com/driftsys) org, alongside `git-std`,
`prim`, `upskill`, `dock`, and `schemas`.

Status: pre-alpha, v0.1 in progress ("check" + "init").

See `docs/planning/folio-roadmap.md` for the full milestone plan and
`docs/planning/folio-plan.md` for the upstream gap analysis and integration
architecture. See `CLAUDE.md` if you're an agent working in this repo.

## Building

```
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

## License

Apache-2.0
