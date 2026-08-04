tool
extends EditorPlugin

var dock: Control = null

func _enter_tree() -> void:
    dock = preload("res://addons/runlens/dock.tscn").instantiate()
    add_control_to_dock(EditorPlugin.DOCK_SLOT_LEFT_UL, dock)

func _exit_tree() -> void:
    if dock:
        remove_control_from_dock(dock)
        dock.queue_free()

func runlens(args: String) -> String:
    var output = []
    var exit_code = OS.execute("runlens", args.split(" ", false), output, true)
    return output[0] if output.size() > 0 else "exit: " + str(exit_code)

func mark_event(kind: String, data: Dictionary) -> void:
    var json = JSON.stringify(data)
    runlens("mark --kind " + kind + " --data " + json)