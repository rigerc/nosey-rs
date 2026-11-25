# Opt-in skills

Skills here are **not** auto-discovered by Claude, Codex, OpenCode or Pi, so they
cost no context until you enable them. They are general-purpose planning and
skill-authoring tools that most sessions in this repo do not need.

| Skill | Use for |
|-------|---------|
| `goap-agent` | Multi-agent planning and coordination |
| `task-decomposition` | Breaking large tasks into atomic goals |
| `triz-analysis` / `triz-solver` | TRIZ contradiction audits and problem solving |
| `self-fix-loop` | Commit → CI → auto-fix loop until green |
| `skill-creator` / `skill-evaluator` | Authoring and evaluating skills |

Enable one for everyone (commit it):

```bash
git mv .agents/skills-optional/<name> .agents/skills/<name>
bash scripts/generate-skills-md.sh
```

Or just for yourself, ask the agent to read `.agents/skills-optional/<name>/SKILL.md`.
