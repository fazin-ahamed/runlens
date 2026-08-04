package runlens

import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.intellij.openapi.project.Project
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.table.JBTable
import java.awt.BorderLayout
import javax.swing.*
import javax.swing.table.AbstractTableModel

class SessionToolWindow(project: Project) : JPanel(BorderLayout()) {
    private val service = project.getService(RunLensProjectService::class.java)
    private val tableModel = SessionTableModel()
    private val table = JBTable(tableModel).apply {
        setSelectionMode(ListSelectionModel.SINGLE_SELECTION)
        columnModel.getColumn(0).preferredWidth = 120
        columnModel.getColumn(1).preferredWidth = 80
        columnModel.getColumn(2).preferredWidth = 300
    }

    init {
        add(JBScrollPane(table), BorderLayout.CENTER)
        val refreshBtn = JButton("Refresh").apply {
            addActionListener { refresh() }
        }
        add(refreshBtn, BorderLayout.SOUTH)
        refresh()
    }

    private fun refresh() {
        service.call("session.list").thenAccept { result ->
            val sessions = result?.asJsonArray ?: JsonArray()
            SwingUtilities.invokeLater { tableModel.setSessions(sessions) }
        }
    }

    private class SessionTableModel : AbstractTableModel() {
        private val columns = listOf("ID", "Events", "Label")
        private var sessions = listOf<JsonElement>()

        fun setSessions(arr: JsonArray) {
            sessions = arr.toList()
            fireTableDataChanged()
        }

        override fun getRowCount() = sessions.size
        override fun getColumnCount() = columns.size
        override fun getColumnName(col: Int) = columns[col]

        override fun getValueAt(row: Int, col: Int): Any? {
            val s = sessions[row].asJsonObject
            return when (col) {
                0 -> s.get("id")?.asString?.take(8) ?: "?"
                1 -> s.get("event_count")?.asInt ?: 0
                2 -> s.get("label")?.asString ?: ""
                else -> null
            }
        }
    }
}