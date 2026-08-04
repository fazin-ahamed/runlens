package runlens

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project

class RecordingAction : AnAction() {
    private var recording = false

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val service = project.getService(RunLensProjectService::class.java)
        val notify = { msg: String ->
            NotificationGroupManager.getInstance()
                .getNotificationGroup("RunLens")
                .createNotification(msg, NotificationType.INFORMATION)
                .notify(project)
        }

        if (recording) {
            service.call("record.stop").thenAccept {
                recording = false
                e.presentation.text = "RunLens: Toggle Recording"
                notify("Recording stopped")
            }
        } else {
            service.call("record.start").thenAccept { result ->
                recording = true
                service.activeSessionId = result?.asJsonObject?.get("session_id")?.asString
                e.presentation.text = "RunLens: Stop Recording"
                notify("Recording started")
            }
        }
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null
    }
}