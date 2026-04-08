# git-remote-agent

Git remote helper for Cocapn agents. Push your branch to the agent, pull commits back.

## Install
```bash
cargo install --path .
# Then:
git remote add agent https://cocapn.dev/your-repo
git push agent main
```

## How It Works
1. `git push agent main` sends commits to the Cocapn agent endpoint
2. Agent processes the diff, generates changes, commits them
3. `git pull agent main` retrieves the agent's commits
4. Works with every git client. Zero plugins needed.

## Protocol
Implements the git remote helper protocol:
- `capabilities` → push, fetch, option
- `push` → POST to agent /api/push
- `fetch` → GET from agent /api/fetch
- `list` → report HEAD and refs

## Architecture
- Rust binary (~200 lines)
- Zero dependencies beyond serde_json
- Uses curl for HTTP (portable)
- Supports agent:// and https:// URLs

Superinstance & Lucineer (DiGennaro et al.)

---

<i>Built with [Cocapn](https://github.com/Lucineer/cocapn-ai) — the open-source agent runtime.</i>
<i>Part of the [Lucineer fleet](https://github.com/Lucineer)</i>

