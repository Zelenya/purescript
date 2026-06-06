use std::{env, fs};
use zed_extension_api::{self as zed, serde_json, settings::LspSettings, Result};

// nwolverson/purescript-language-server (Node, the original server)
const SERVER_PATH: &str = "node_modules/.bin/purescript-language-server";
const PACKAGE_NAME: &str = "purescript-language-server";

// purefunctor/purescript-alexandrite (PureScript analyzer)
const ALEXANDRITE_ID: &str = "purescript-alexandrite";
const ALEXANDRITE_REPO: &str = "purefunctor/purescript-alexandrite";
const ALEXANDRITE_BIN: &str = "purescript-alexandrite";
// Pinned: alexandrite is alpha (v0.0.x), so we track a tag rather than `latest`.
const ALEXANDRITE_VERSION: &str = "v0.0.12";

struct PurescriptExtension {
    did_find_node_server: bool,
}

impl zed::Extension for PurescriptExtension {
    fn new() -> Self {
        Self {
            did_find_node_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            ALEXANDRITE_ID => self.alexandrite_command(language_server_id, worktree),
            // "purescript-language-server" and any other id use the original server
            _ => self.node_server_command(language_server_id, worktree),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        match language_server_id.as_ref() {
            // Alexandrite discovers sources via spago.lock / --source-command;
            ALEXANDRITE_ID => Ok(Self::lsp_settings(language_server_id, worktree)
                .and_then(|settings| settings.initialization_options)),
            _ => Ok(Some(Self::node_server_config(language_server_id, worktree))),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        match language_server_id.as_ref() {
            ALEXANDRITE_ID => Ok(Self::lsp_settings(language_server_id, worktree)
                .and_then(|settings| settings.settings)),
            _ => Ok(Some(Self::node_server_config(language_server_id, worktree))),
        }
    }
}

impl PurescriptExtension {
    /// The user's `lsp.<id>` settings for this worktree, if any.
    fn lsp_settings(
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Option<LspSettings> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok()
    }

    // nwolverson/purescript-language-server (original language server)
    fn node_server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).is_ok_and(|stat| stat.is_file())
    }

    fn node_server_script_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<String> {
        let server_exists = self.node_server_exists();
        if self.did_find_node_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.node_server_exists() {
                        Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'",
                        ))?;
                    }
                }
                Err(error) => {
                    if !self.node_server_exists() {
                        Err(error)?;
                    }
                }
            }
        }

        self.did_find_node_server = true;
        Ok(SERVER_PATH.to_string())
    }

    fn node_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.node_server_script_path(language_server_id)?;
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                env::current_dir()
                    .unwrap()
                    .join(&server_path)
                    .to_string_lossy()
                    .to_string(),
                "--stdio".to_string(),
            ],
            env: Self::lsp_settings(language_server_id, worktree)
                .and_then(|settings| settings.binary.and_then(|settings| settings.env))
                .map_or_else(Default::default, |binary| binary.into_iter().collect()),
        })
    }

    /// Config object sent to `purescript-language-server`,
    /// used for both initialization options and workspace configuration.
    ///
    /// While the purescript-language-server was built for vscode,
    /// if we don't do this explicitly, zed sends an empty `{}`.
    /// So, for example, options like `purescript.formatter` are silently ignored.
    fn node_server_config(
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> serde_json::Value {
        // Read user settings from `lsp.purescript-language-server.settings`
        let mut settings = Self::lsp_settings(language_server_id, worktree)
            .and_then(|settings| settings.settings)
            .unwrap_or_else(|| serde_json::json!({}));

        // Inject required defaults (set `purescript.addSpagoSources` to `true`).
        // The server expects objects here, so reset malformed (non-object)
        // shapes instead of forwarding them verbatim without the default.
        if !settings.is_object() {
            settings = serde_json::json!({});
        }
        if let Some(root) = settings.as_object_mut() {
            let purescript = root
                .entry("purescript")
                .or_insert_with(|| serde_json::json!({}));
            if !purescript.is_object() {
                *purescript = serde_json::json!({});
            }
            if let Some(purescript) = purescript.as_object_mut() {
                purescript
                    .entry("addSpagoSources")
                    .or_insert(serde_json::Value::Bool(true));
            }
        }

        settings
    }

    // purefunctor/purescript-alexandrite (PureScript analyzer)
    fn alexandrite_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (path, arguments, env) = Self::lsp_settings(language_server_id, worktree)
            .and_then(|s| s.binary)
            .map(|b| (b.path, b.arguments, b.env))
            .unwrap_or_default();

        // Binary resolution: explicit path > $PATH > downloaded release
        let command = match path.or_else(|| worktree.which(ALEXANDRITE_BIN)) {
            Some(path) => path,
            None => Self::alexandrite_download(language_server_id)?,
        };

        // Args: `--stdio` first, then any user-supplied flags.
        // Skip the injection if the user already passes `--stdio` themselves
        // (alexandrite rejects the flag when given twice).
        let user_args = arguments.unwrap_or_default();
        let mut args = Vec::new();
        if !user_args.iter().any(|arg| arg == "--stdio") {
            args.push("--stdio".to_string());
        }
        args.extend(user_args);

        let env = env.unwrap_or_default().into_iter().collect();

        Ok(zed::Command { command, args, env })
    }

    // This could be simplified when alexandrite is released (and we don't have to package it)
    fn alexandrite_download(language_server_id: &zed::LanguageServerId) -> Result<String> {
        // Map our platform to the target triple used in upstream's asset names
        let (os, arch) = zed::current_platform();
        let target = match (os, arch) {
            (zed::Os::Mac, _) => "universal-apple-darwin",
            (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc",
            (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
            // Upstream publishes no Linux ARM build (see release.yml).
            // Point users to a self-installed binary.
            (os, arch) => {
                return Err(format!(
                    "No purescript-alexandrite binary for {os:?}/{arch:?} yet. \
                     Install it manually (e.g. `cargo install --git \
                     https://github.com/purefunctor/purescript-alexandrite`) and set \
                     `\"lsp\": {{ \"purescript-alexandrite\": {{ \"binary\": {{ \"path\": \"…\" }} }} }}` \
                     in your Zed settings."
                ));
            }
        };

        // The binary lives one directory deep, named after the archive (without extension).
        // The version is pinned, so this path is fully deterministic.
        let version_dir = format!("alexandrite-{ALEXANDRITE_VERSION}");
        let bin_name = if matches!(os, zed::Os::Windows) {
            format!("{ALEXANDRITE_BIN}.exe")
        } else {
            ALEXANDRITE_BIN.to_string()
        };
        let binary_path = format!("{version_dir}/purescript-alexandrite-{target}/{bin_name}");

        // Re-use an already-downloaded binary (if it already exists).
        // Re-apply the exec bit in case a previous run was interrupted before setting it.
        if fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::make_file_executable(&binary_path)?;
            return Ok(binary_path);
        }

        // Show download progress in the status bar
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        // Fetch the pinned release's asset list from github
        let release = zed::github_release_by_tag_name(ALEXANDRITE_REPO, ALEXANDRITE_VERSION)?;

        // Pick the archive format upstream publishes for this OS
        let (extension, file_type) = match os {
            zed::Os::Windows => ("zip", zed::DownloadedFileType::Zip),
            _ => ("tar.gz", zed::DownloadedFileType::GzipTar),
        };
        // Find the matching release asset
        let asset_name = format!("purescript-alexandrite-{target}.{extension}");
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| {
                format!("no asset named '{asset_name}' in release {ALEXANDRITE_VERSION}")
            })?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(&asset.download_url, &version_dir, file_type)?;

        // Guard against upstream changing the archive layout: drop the bad
        // download so the next attempt retries instead of wedging on it.
        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            fs::remove_dir_all(&version_dir).ok();
            return Err(format!(
                "downloaded asset '{asset_name}' did not contain expected path '{binary_path}'"
            ));
        }
        zed::make_file_executable(&binary_path)?;

        // Drop stale versions to keep the extension work dir small.
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("alexandrite-") && name != version_dir {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        Ok(binary_path)
    }
}

zed::register_extension!(PurescriptExtension);
