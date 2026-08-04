package runlenseclipse.views;

import org.eclipse.jface.action.Action;
import org.eclipse.jface.action.IToolBarManager;
import org.eclipse.jface.viewers.*;
import org.eclipse.swt.SWT;
import org.eclipse.swt.layout.GridData;
import org.eclipse.swt.layout.GridLayout;
import org.eclipse.swt.widgets.Composite;
import org.eclipse.swt.widgets.TableColumn;
import org.eclipse.ui.part.ViewPart;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import runlenseclipse.Activator;
import runlenseclipse.DaemonClient;

public class SessionView extends ViewPart {
    public static final String ID = "runlenseclipse.views.SessionView";
    private TableViewer viewer;

    @Override
    public void createPartControl(Composite parent) {
        parent.setLayout(new GridLayout(1, false));
        viewer = new TableViewer(parent, SWT.H_SCROLL | SWT.V_SCROLL | SWT.FULL_SELECTION | SWT.BORDER);
        viewer.getTable().setLayoutData(new GridData(GridData.FILL_BOTH));
        viewer.getTable().setHeaderVisible(true);
        viewer.getTable().setLinesVisible(true);

        String[] cols = {"ID", "Events", "Label"};
        int[] widths = {120, 80, 300};
        for (int i = 0; i < cols.length; i++) {
            TableColumn tc = new TableColumn(viewer.getTable(), SWT.NONE);
            tc.setText(cols[i]);
            tc.setWidth(widths[i]);
        }

        viewer.setContentProvider(ArrayContentProvider.getInstance());
        viewer.setLabelProvider(new ITableLabelProvider() {
            public String getColumnText(Object element, int col) {
                JsonObject s = ((JsonElement) element).getAsJsonObject();
                return switch (col) {
                    case 0 -> s.get("id") != null ? s.get("id").getAsString().substring(0, Math.min(8, s.get("id").getAsString().length())) : "?";
                    case 1 -> s.get("event_count") != null ? String.valueOf(s.get("event_count").getAsInt()) : "0";
                    case 2 -> s.get("label") != null ? s.get("label").getAsString() : "";
                    default -> "";
                };
            }
            public boolean isLabelProperty(Object element, String property) { return false; }
            public void addListener(ILabelProviderListener listener) {}
            public void removeListener(ILabelProviderListener listener) {}
            public void dispose() {}
            public Image getColumnImage(Object element, int col) { return null; }
        });

        getSite().setSelectionProvider(viewer);
        createToolbarActions();
        refresh();
    }

    private void createToolbarActions() {
        IToolBarManager mgr = getViewSite().getActionBars().getToolBarManager();
        mgr.add(new Action("Refresh") {
            public void run() { refresh(); }
        });
    }

    private void refresh() {
        DaemonClient daemon = Activator.getDefault().getDaemon();
        daemon.call("session.list").thenAccept(result -> {
            JsonArray sessions = result != null ? result.getAsJsonArray() : new JsonArray();
            getSite().getShell().getDisplay().asyncExec(() ->
                viewer.setInput(toArray(sessions)));
        });
    }

    private JsonElement[] toArray(JsonArray arr) {
        JsonElement[] result = new JsonElement[arr.size()];
        for (int i = 0; i < arr.size(); i++) result[i] = arr.get(i);
        return result;
    }

    @Override
    public void setFocus() { viewer.getControl().setFocus(); }
}