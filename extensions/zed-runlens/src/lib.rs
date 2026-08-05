use zed_extension_api as zed;

struct RunLensExtension;

impl zed::Extension for RunLensExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> Result<zed::Command, String> {
        Ok(zed::Command {
            command: "runlens".to_string(),
            args: vec!["mcp".to_string()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(RunLensExtension);