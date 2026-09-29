use zed_extension_api::{
    self as zed,
    settings::LspSettings,
    Command, Extension, LanguageServerId, Result, Worktree,
};

struct CezExtension;

impl Extension for CezExtension {
    fn new() -> Self {
        CezExtension
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let binary_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary);

        let path = binary_settings
            .as_ref()
            .and_then(|b| b.path.clone())
            .or_else(|| worktree.which("cez-lsp"))
            .or_else(|| worktree.which("cez"))
            .unwrap_or_else(|| "/home/low4rch/.local/bin/cez-lsp".to_string());

        let mut args = binary_settings
            .as_ref()
            .and_then(|b| b.arguments.clone())
            .unwrap_or_default();

        if args.is_empty() && (path.ends_with("/cez") || path == "cez") {
            args.push("lsp".to_string());
        }

        let env = binary_settings
            .and_then(|b| b.env)
            .map(|env| env.into_iter().collect())
            .unwrap_or_default();

        Ok(Command {
            command: path,
            args,
            env,
        })
    }
}

zed::register_extension!(CezExtension);
