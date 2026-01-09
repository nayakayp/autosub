# Autosub-RS Autonomous Agent Loop

> This file instructs AI agents to build the autosub CLI iteratively across multiple sessions.

---

## 🎯 Mission

Build the **autosub** CLI tool in Rust following `IMPLEMENTATION_PLAN.md`.

---

## 📋 Instructions

### 1. Read Context First
- Read `IMPLEMENTATION_PLAN.md` to understand the full architecture
- Read `TASKS.md` to see all tasks and their current status
- Read `CHANGELOG.md` to see what previous sessions accomplished and any issues encountered

### 2. Select Your Task
- Pick **ONE uncompleted task** from `TASKS.md` (marked with `[ ]`)
- Prioritize tasks in order: Phase 1 → Phase 2 → Phase 3 → etc.
- If a task depends on incomplete tasks, complete dependencies first
- If previous session failed a task, attempt to fix it before moving on

### 3. Implement the Task
- Write clean, idiomatic Rust code
- Follow the code patterns shown in `TASKS.md`
- Run `cargo build` and `cargo check` frequently to catch errors early
- Run `cargo clippy` for linting before marking complete

### 4. Test Your Work
- Run `cargo test` if tests exist for your module
- If the CLI is functional enough, test with real data:
  - Sample video: `sample.mp4` (Japanese language)
  - API keys are exported as environment variables:
    - `OPENAI_API_KEY`
    - `GEMINI_API_KEY`
- Test various scenarios when appropriate

### 5. Update Documentation
After completing work, update these files:

#### TASKS.md
- Mark completed tasks with `[x]`
- Update phase status if all tasks in phase are done:
  - 🔴 Not Started → 🟡 In Progress → 🟢 Complete

#### CHANGELOG.md
Append a session log at the bottom using this format:

```markdown
---

## Session {N} - {DATE} {TIME} {TIMEZONE}

### Status: {COMPLETED | PARTIAL | FAILED}

**Tasks Attempted:**
- {Task ID}: {Brief description} — ✅ Success | ⚠️ Partial | ❌ Failed

**Summary:**
{2-3 sentences describing what was accomplished}

### What Works Now
- {List of new functionality}

### Issues Encountered
- {Problem}: {How it was solved or why it remains unsolved}

### Next Steps for Next Agent
1. {Most important next task}
2. {Second priority}
3. {Third priority}

### Technical Notes
- {API quirks discovered}
- {Code patterns that worked well}
- {Gotchas for future sessions}
```

---

## 🧪 Testing Commands

```bash
# Build and check
cargo build
cargo check
cargo clippy

# Run tests
cargo test

# Run the CLI (when ready)
cargo run -- sample.mp4 -o output.srt --provider whisper
cargo run -- sample.mp4 -o output.srt --provider gemini --language ja

# Run with verbose logging
cargo run -- sample.mp4 -o output.srt -v
```

---

## ⚠️ Important Rules

1. **One task at a time** — Complete fully before starting another
2. **Build often** — Run `cargo build` after every significant change
3. **Don't skip dependencies** — If task 2.1 needs 1.3, do 1.3 first
4. **Log everything** — Both successes AND failures go in CHANGELOG.md
5. **Leave breadcrumbs** — Next agent should understand exactly where you left off
6. **Test with real APIs** — When possible, validate with actual API calls
7. **Clean up** — Remove debug code and commented-out experiments

---

## 📁 Project Files

| File | Purpose |
|------|---------|
| `IMPLEMENTATION_PLAN.md` | High-level architecture and API specs |
| `TASKS.md` | Detailed task checklist with code snippets |
| `CHANGELOG.md` | Session logs and learnings |
| `sample.mp4` | Test video file (Japanese audio) |
| `src/` | Rust source code |
| `Cargo.toml` | Rust dependencies |

---

## 🔑 Environment Variables

These are already exported in this directory:
- `OPENAI_API_KEY` — For Whisper API
- `GEMINI_API_KEY` — For Gemini Audio API

---

## 🚦 Session Checklist

Before ending your session, verify:

- [ ] Code compiles (`cargo build` succeeds)
- [ ] No clippy warnings (`cargo clippy` clean)
- [ ] Task marked complete in `TASKS.md`
- [ ] Session logged in `CHANGELOG.md`
- [ ] Next steps clearly documented
- [ ] Any new files are properly structured
- [ ] **Git commit with descriptive message**

---

## 📦 Git Commit

Always commit your work at the end of each session:

```bash
# Stage all changes
git add .

# Commit with descriptive message
git commit -m "feat(phase-X): {brief description}

- Completed: {task IDs}
- Status: {what works now}

Session {N}"
```

**Commit message prefixes:**
- `feat` — New feature/functionality
- `fix` — Bug fix
- `refactor` — Code restructure without behavior change
- `docs` — Documentation only
- `chore` — Build, deps, config changes
