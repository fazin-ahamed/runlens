package runlenseclipse;

import org.eclipse.ui.plugin.AbstractUIPlugin;
import org.osgi.framework.BundleContext;

public class Activator extends AbstractUIPlugin {
    public static final String PLUGIN_ID = "runlenseclipse";
    private static Activator plugin;
    private DaemonClient daemon;

    @Override
    public void start(BundleContext context) throws Exception {
        super.start(context);
        plugin = this;
        daemon = new DaemonClient("ws://localhost:9876");
        daemon.connect();
    }

    @Override
    public void stop(BundleContext context) throws Exception {
        if (daemon != null) daemon.disconnect();
        plugin = null;
        super.stop(context);
    }

    public static Activator getDefault() { return plugin; }
    public DaemonClient getDaemon() { return daemon; }
}
