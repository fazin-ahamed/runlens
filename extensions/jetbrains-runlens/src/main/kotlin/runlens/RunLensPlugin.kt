package runlens

import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory

@Service(Service.Level.PROJECT)
class RunLensProjectService(private val project: Project) {
    val daemon = DaemonClient()
    var activeSessionId: String? = null

    fun connect() = daemon.connect()
    fun disconnect() = daemon.disconnect()
    fun call(method: String, params: Any = mapOf<String, Any>()) = daemon.call(method, params)
}

class SessionToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = SessionToolWindow(project)
        val content = ContentFactory.getInstance().createContent(panel, "", false)
        toolWindow.contentManager.addContent(content)
    }
}

class RunLensStatusBarFactory : com.intellij.openapi.wm.StatusBarWidgetFactory {
    override fun getId() = "runlens.status"
    override fun getDisplayName() = "RunLens Status"
    override fun isAvailable(project: Project) = true
    override fun createWidget(project: Project) = StatusBarWidget(project)
}