extends Node
# RunLens autoload singleton (stub).
#
# Wrap calls to the runlens CLI from inside a Godot project. Heavy
# work (PTY capture, hash chain) lives in the runlens binary; this
# script only orchestrates.
class_name RunLens

const RUNLENS_BIN := "runlens"

func is_available() -> bool:
    var output := []
    var rc := OS.execute("which", [RUNLENS_BIN], output, true, false)
    return rc == 0 and (output.size() == 0 or output[0].strip_edges() != "")

func init_workspace(dir: String) -> int:
    return OS.execute(RUNLENS_BIN, ["init"], [], true, false)

func record(label: String, args: PackedStringArray) -> int:
    var argv := ["record", "--label", label]
    argv.append_array(args)
    return OS.execute(RUNLENS_BIN, argv, [], true, false)

func list_sessions(limit: int) -> String:
    var out := []
    OS.execute(RUNLENS_BIN, ["list", "--limit", str(limit), "--json"], out, true, false)
    return out[0] if out.size() > 0 else ""

func verify_session(session_id: String) -> int:
    return OS.execute(RUNLENS_BIN, ["verify", session_id], [], true, false)
