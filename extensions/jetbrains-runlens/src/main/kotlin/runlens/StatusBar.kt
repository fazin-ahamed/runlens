package runlens

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.impl.status.EditorBasedWidget
import java.awt.*
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import javax.swing.JComponent
import javax.swing.Timer

class StatusBarWidget(project: Project) : EditorBasedWidget(project), StatusBarWidget.Multiframe {
    private var connected = false
    private var recording = false
    private val refreshTimer = Timer(5000) { updateStatus() }

    init {
        refreshTimer.start()
        val service = project.getService(RunLensProjectService::class.java)
        service.daemon.addListener(object : DaemonClient.DaemonListener {
            override fun onConnected() { connected = true; myStatusBar?.updateWidget(ID) }
            override fun onDisconnected() { connected = false; myStatusBar?.updateWidget(ID) }
        })
        updateStatus()
    }

    private fun updateStatus() {
        try {
            val service = project?.getService(RunLensProjectService::class.java)
            service?.daemon?.call("daemon.status")?.thenAccept {
                connected = true
                myStatusBar?.updateWidget(ID)
            }
        } catch (_: Exception) {}
    }

    override fun ID() = "runlens.status"
    override fun getPresentation() = StatusBarWidget.WidgetPresentation { _ ->
        object : JComponent() {
            override fun getPreferredSize() = Dimension(100, 24)
            override fun paintComponent(g: Graphics) {
                val g2 = g.create() as Graphics2D
                g2.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON)
                val color = when {
                    recording -> Color(0, 180, 60)
                    connected -> Color(60, 120, 220)
                    else -> Color(160, 160, 160)
                }
                g2.color = color
                g2.fillOval(4, 8, 8, 8)
                g2.color = Color(50, 50, 50)
                val label = if (recording) "REC" else if (connected) "RL" else "OFF"
                g2.drawString(label, 16, 16)
                g2.dispose()
            }
        }
    }
    override fun getPresentationType() = StatusBarWidget.WidgetPresentationType.CUSTOM
    override fun install(statusBar: com.intellij.openapi.wm.StatusBar) { super.install(statusBar) }
}