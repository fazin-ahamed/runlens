# RunLens shell helpers -- source this from ~/.bashrc or ~/.zshrc

RL_BIN="${RL_BIN:-runlens}"

rl()    { "$RL_BIN" "$@"; }
rl:()   { "$RL_BIN" daemon status 2>/dev/null && echo "daemon running" || echo "daemon not running (start with: runlens daemon)"; }

rl-status()   { "$RL_BIN" daemon status; }
rl-start()    { "$RL_BIN" record start --label "${1:-nano}"; }
rl-stop()     { "$RL_BIN" record stop; }
rl-list()     { "$RL_BIN" list --limit "${1:-10}"; }
rl-critical() { "$RL_BIN" graph critical "$1"; }
rl-last()     { local id=$("$RL_BIN" list --limit 1 --json 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['session_id'] if isinstance(d,list) and d else d.get('sessions',[{}])[0].get('session_id',''))" 2>/dev/null); [ -n "$id" ] && "$RL_BIN" graph critical "$id" || echo "no sessions found"; }