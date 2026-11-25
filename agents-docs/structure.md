# Project Structure

```
rust-2026-template/
├── .agents/
│   ├── SKILLS.md                  # Auto-generated skills index
│   ├── ORCHESTRATION.md           # Multi-agent orchestration rules
│   ├── ci/                        # CI health status artifacts
│   │   └── ci-summary.md
│   ├── context/                   # Cross-repo context for derived repositories
│   │   ├── external-repos.json
│   │   ├── shared-conventions.md
│   │   └── workflow-state.json
│   ├── events/                    # Agentic metrics event files (DORA)
│   ├── skills/                    # Active skills (index: .agents/SKILLS.md)
│   └── skills-optional/           # Opt-in skills, not auto-discovered
├── .cargo/
│   └── config.toml                # Linker (mold), dev profile, cargo aliases
├── .claude/                       # Claude Code: skills symlink + hooks
├── .config/
│   └── nextest.toml               # nextest profiles: default, ci
├── .githooks/                     # pre-commit (fast) and pre-push (clippy, deny)
├── .github/
│   ├── ISSUE_TEMPLATE/
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── actions/setup-rust/        # Reusable composite action
│   ├── dependabot.yml
│   ├── labeler.yml
│   └── workflows/                 # GitHub Actions workflows
│       ├── ci.yml                 # Format, clippy, test, audit, deny, MSRV
│       ├── commitlint.yml
│       ├── dependabot-auto-merge.yml
│       ├── docs-check.yml
│       ├── eval.yml
│       ├── hotfix.yml
│       ├── release.yml            # Tag-triggered release with cargo-dist
│       ├── secretlint.yml
│       ├── security-scan.yml
│       ├── skills-evaluation.yml
│       └── test-init-template.yml
├── .opencode/                     # OpenCode integration
├── .vscode/
│   └── settings.json              # rust-analyzer + WSL2 settings
├── agents-docs/                   # Agent reference documentation
│   ├── agent-doc-flow.md
│   ├── commands.md                # Command quick reference
│   ├── conventions.md             # Code conventions and invariants
│   ├── dora-metrics.md
│   ├── HANDOFF.md
│   ├── HOOKS.md
│   ├── LESSONS.md
│   ├── reference.md               # Tooling, tiers, setup and release details
│   ├── structure.md               # This file
│   ├── SUB-AGENTS.md
│   ├── tool-adapters.md
│   └── workflow.md                # Change workflow and skill/CLI pattern
├── config/
│   ├── ci/
│   ├── profiles/                  # Environment-specific JSON configs
│   ├── template-profiles/         # init-template profiles
│   └── xtask.json                 # Quality gate configuration
├── crates/                        # Workspace member crates
│   ├── example-crate/             # Placeholder library (rename for your project)
│   ├── sample-app/                # Reference binary application
│   └── xtask/                     # Cargo task runner
├── docs/
│   ├── adr/                       # Architecture Decision Records
│   ├── architecture/              # Architecture context for AI agents
│   ├── patterns/                  # Template pattern guides
│   └── src/                       # mdbook source files
├── examples/                      # Workspace members demonstrating patterns
│   ├── cli_dry_run/               # Safe --dry-run CLI pattern
│   ├── hello_world/               # Simple hello world example
│   ├── security_egress_and_path_validation/
│   └── security_secret_detector/
├── monitoring/                    # Monitoring configuration
├── plans/
│   ├── GOAP_STATE.md              # AI agent world state pointer (≤20 lines, ADR 0005)
│   ├── _status.json               # Machine anchor: active_plan, phases, handover_ref
│   └── adr/                       # Architecture Decision Records
├── reports/                       # Generated HTML reports (git-ignored)
├── schema/                        # JSON Schema definitions
├── scripts/                       # Automation scripts
│   ├── bootstrap.sh               # One-command first-time setup
│   ├── code-quality.sh            # fmt | clippy | audit | check | fix
│   ├── doctor.sh                  # Environment diagnostics
│   ├── generate-llms-txt.sh       # Regenerate LLM context files
│   ├── generate-skills-md.sh      # Regenerate skills index
│   ├── init-template.sh           # Initialize new project from template
│   ├── quality-gates.sh           # Local quality gate runner
│   ├── release-manager.sh         # validate | prepare | publish
│   ├── setup-beads.sh             # Optional: beads (bd) issue tracking
│   ├── setup-gpui-mcp.sh          # Optional: gpui-mcp server for GPUI automation
│   ├── validate-skills.sh         # Skill validation
│   └── ...                        # more automation scripts
├── templates/                     # Template files
├── tests/                         # Integration tests
├── .clippy.toml                   # Clippy lint configuration
├── .envrc                         # direnv environment setup
├── .gitignore
├── AGENTS.md                      # Canonical AI agent instructions
├── CHANGELOG.md                   # Keep-a-changelog format
├── CLAUDE.md                      # Claude Code adapter
├── CONTRIBUTING.md
├── Cargo.lock
├── Cargo.toml                     # Workspace manifest
├── HARNESS.md                     # Harness engineering guide
├── LICENSE                        # MIT
├── MIGRATION.md                   # Template adoption guide
├── QUICKSTART.md                  # Setup and onboarding guide
├── SECURITY.md
├── VERSION                        # Plain-text version (0.0.0 for adopter)
├── deny.toml                      # cargo-deny v2 supply chain config
├── flake.nix                      # Nix flake (optional)
├── llms.txt                       # LLM context file (machine-readable)
├── llms-full.txt                  # Full LLM source context (auto-generated)
├── opencode.json                  # OpenCode configuration
├── rust-toolchain.toml            # Pinned stable 1.99
└── rustfmt.toml                   # Rustfmt 2024 edition settings
```
