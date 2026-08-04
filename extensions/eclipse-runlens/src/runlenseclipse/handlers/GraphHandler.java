package runlenseclipse.handlers;

import org.eclipse.core.commands.AbstractHandler;
import org.eclipse.core.commands.ExecutionEvent;
import org.eclipse.core.commands.ExecutionException;
import org.eclipse.jface.dialogs.MessageDialog;
import org.eclipse.ui.IWorkbenchWindow;
import org.eclipse.ui.handlers.HandlerUtil;

import runlenseclipse.Activator;
import runlenseclipse.DaemonClient;

public class GraphHandler extends AbstractHandler {
    @Override
    public Object execute(ExecutionEvent event) throws ExecutionException {
        IWorkbenchWindow window = HandlerUtil.getActiveWorkbenchWindowChecked(event);
        DaemonClient daemon = Activator.getDefault().getDaemon();

        daemon.call("session.list").thenCompose(result -> {
            var arr = result.getAsJsonArray();
            if (arr.size() == 0) {
                window.getShell().getDisplay().asyncExec(() ->
                    MessageDialog.openWarning(window.getShell(), "RunLens", "No sessions available"));
                return null;
            }
            String sessionId = arr.get(0).getAsJsonObject().get("id").getAsString();
            return daemon.call("graph.critical", java.util.Map.of("session_id", sessionId));
        }).thenAccept(result -> {
            if (result == null) return;
            int count = result.getAsJsonObject().getAsJsonArray("critical_path").size();
            String msg = "Critical path: " + count + " nodes";
            window.getShell().getDisplay().asyncExec(() ->
                MessageDialog.openInformation(window.getShell(), "RunLens", msg));
        });

        return null;
    }
}
