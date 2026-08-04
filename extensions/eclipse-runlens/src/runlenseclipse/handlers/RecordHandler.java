package runlenseclipse.handlers;

import org.eclipse.core.commands.AbstractHandler;
import org.eclipse.core.commands.ExecutionEvent;
import org.eclipse.core.commands.ExecutionException;
import org.eclipse.jface.dialogs.MessageDialog;
import org.eclipse.ui.IWorkbenchWindow;
import org.eclipse.ui.handlers.HandlerUtil;

import runlenseclipse.Activator;
import runlenseclipse.DaemonClient;

public class RecordHandler extends AbstractHandler {
    private boolean recording = false;

    @Override
    public Object execute(ExecutionEvent event) throws ExecutionException {
        IWorkbenchWindow window = HandlerUtil.getActiveWorkbenchWindowChecked(event);
        DaemonClient daemon = Activator.getDefault().getDaemon();

        String method = recording ? "record.stop" : "record.start";
        daemon.call(method).thenAccept(result -> {
            recording = !recording;
            String msg = recording ? "Recording started" : "Recording stopped";
            window.getShell().getDisplay().asyncExec(() ->
                MessageDialog.openInformation(window.getShell(), "RunLens", msg));
        });

        return null;
    }
}
