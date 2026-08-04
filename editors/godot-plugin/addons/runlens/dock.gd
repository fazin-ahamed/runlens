tool
extends VBoxContainer

var recording := false

func _ready() -> void:
    $RecordBtn.connect("pressed", self, "_on_record")
    $StopBtn.connect("pressed", self, "_on_stop")
    $ListBtn.connect("pressed", self, "_on_list")
    $StopBtn.disabled = true

func _on_record() -> void:
    var cmd = "echo recording"
    $Output.text = runlens("record -- " + cmd)
    recording = true
    $RecordBtn.disabled = true
    $StopBtn.disabled = false

func _on_stop() -> void:
    $Output.text = "stopped"
    recording = false
    $RecordBtn.disabled = false
    $StopBtn.disabled = true

func _on_list() -> void:
    $Output.text = runlens("list")

func runlens(args: String) -> String:
    var output = []
    OS.execute("runlens", args.split(" ", false), output, true)
    return output[0] if output.size() > 0 else "no output"