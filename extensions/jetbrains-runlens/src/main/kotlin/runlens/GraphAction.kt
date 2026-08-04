package runlens

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent

class GraphAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val service = project.getService(RunLensProjectService::class.java)
        val sessionId = service.activeSessionId ?: run {
            showError(project, "No active session. Start a recording first.")
            return
        }
        service.call("graph.critical", mapOf("session_id" to sessionId))
            .thenAccept { result ->
                val summary = result?.asJsonObject
                val path = summary?.get("critical_path")?.asJsonArray
                val count = path?.size() ?: 0
                showInfo(project, "Critical path: $count nodes")
            }
            .exceptionally { t ->
                showError(project, t.message ?: "Failed to get critical path")
                null
            }
    }

    private fun showInfo(project: com.intellij.openapi.project.Project, msg: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("RunLens")
            .createNotification(msg, NotificationType.INFORMATION)
            .notify(project)
    }

    private fun showError(project: com.intellij.openapi.project.Project, msg: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("RunLens")
            .createNotification(msg, NotificationType.ERROR)
            .notify(project)
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null
    }
}