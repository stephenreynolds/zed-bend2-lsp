use std::fs;

use sha2::{Digest, Sha256};
use zed_extension_api::{
    self as zed,
    lsp::{Completion, CompletionKind, Symbol, SymbolKind},
    serde_json::Value,
    settings::LspSettings,
    CodeLabel, CodeLabelSpan, LanguageServerId, Result, Worktree,
};

const SERVER_BINARY: &str = "bend2-lsp";
const SERVER_REPO: &str = "IlyaGulya/bend2-lsp-rs";

/// The bend2-lsp release fetched when the server is not already installed.
/// Bump the tag and the checksums together, from the release's `SHA256SUMS`.
const SERVER_VERSION: &str = "v0.7.1";

struct ServerAsset {
    name: &'static str,
    sha256: &'static str,
}

fn server_asset() -> Result<ServerAsset> {
    use zed::{Architecture::*, Os::*};
    let (name, sha256) = match zed::current_platform() {
        (Mac, Aarch64) => (
            "bend2-lsp-aarch64-apple-darwin",
            "3e671efbf24c561b278883620e14bc36474323dd1922ed097f1a39e721e990b1",
        ),
        (Mac, X8664) => (
            "bend2-lsp-x86_64-apple-darwin",
            "ffaa8f41c242a58ebb630195b03d7ecad0ce2f8a69a4c089063400109916c3d0",
        ),
        (Linux, Aarch64) => (
            "bend2-lsp-aarch64-unknown-linux-gnu",
            "d35b8c5e3c39a7d518f184b3b2c20cfb81f9d90bfd0a58713c2111ca73ec362b",
        ),
        (Linux, X8664) => (
            "bend2-lsp-x86_64-unknown-linux-gnu",
            "0ba0487a96594a1853de4ae7dd2c383257739afb31504249fdba4038077fb049",
        ),
        (Windows, Aarch64) => (
            "bend2-lsp-aarch64-pc-windows-msvc.exe",
            "2f0ed8b94c96f3e4e8f3360ef6dffb2a5c8c8f6481e29c29a726b68866d066a6",
        ),
        (Windows, X8664) => (
            "bend2-lsp-x86_64-pc-windows-msvc.exe",
            "557a37bd37fd3da18cec84bc9073b43060c0a3e6c6526eabf46f44dbd6dfffe6",
        ),
        _ => {
            return Err(format!(
                "no prebuilt {SERVER_BINARY} for this platform; install it yourself and \
                 put it on PATH or set lsp.{SERVER_BINARY}.binary.path"
            ))
        }
    };
    Ok(ServerAsset { name, sha256 })
}

fn sha256_hex(path: &str) -> Result<String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|stat| stat.is_file())
}

struct BendExtension {
    /// Path of the downloaded server, relative to the extension's work directory.
    cached_server: Option<String>,
}

impl BendExtension {
    /// Returns the downloaded server binary, fetching the pinned release first
    /// if this is the first use. The download is checked against the release's
    /// published SHA-256 before it is made executable.
    fn downloaded_server(&mut self, id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_server {
            if is_file(path) {
                return Ok(path.clone());
            }
        }

        let asset = server_asset()?;
        let version_dir = format!("{SERVER_BINARY}-{SERVER_VERSION}");
        let binary_path = format!("{version_dir}/{}", asset.name);

        if !is_file(&binary_path) {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            fs::create_dir_all(&version_dir)
                .map_err(|e| format!("failed to create {version_dir}: {e}"))?;

            let download_path = format!("{binary_path}.download");
            let url = format!(
                "https://github.com/{SERVER_REPO}/releases/download/{SERVER_VERSION}/{}",
                asset.name
            );
            zed::download_file(&url, &download_path, zed::DownloadedFileType::Uncompressed)
                .map_err(|e| format!("failed to download {url}: {e}"))?;

            let actual = sha256_hex(&download_path)?;
            if actual != asset.sha256 {
                fs::remove_file(&download_path).ok();
                return Err(format!(
                    "checksum mismatch for {url}: expected {}, got {actual}",
                    asset.sha256
                ));
            }

            zed::make_file_executable(&download_path)?;
            fs::rename(&download_path, &binary_path)
                .map_err(|e| format!("failed to install {binary_path}: {e}"))?;

            // Drop servers left behind by earlier versions of the extension.
            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with(SERVER_BINARY) && name != version_dir {
                        fs::remove_dir_all(entry.path()).ok();
                    }
                }
            }
        }

        self.cached_server = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for BendExtension {
    fn new() -> Self {
        Self {
            cached_server: None,
        }
    }

    /// Server lookup order: `lsp.bend2-lsp.binary.path` from settings, then
    /// `bend2-lsp` on the project's PATH, then the pinned release download.
    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary);

        let command = match binary.as_ref().and_then(|binary| binary.path.clone()) {
            Some(path) => path,
            None => match worktree.which(SERVER_BINARY) {
                Some(path) => path,
                None => self.downloaded_server(id)?,
            },
        };

        // The server shells out to the `bend` compiler, so it needs the
        // project's shell environment (PATH in particular) to find it.
        let mut env = worktree.shell_env();
        if let Some(overrides) = binary.as_ref().and_then(|binary| binary.env.clone()) {
            for (key, value) in overrides {
                env.retain(|(existing, _)| *existing != key);
                env.push((key, value));
            }
        }

        Ok(zed::Command {
            command,
            args: binary
                .and_then(|binary| binary.arguments)
                .unwrap_or_default(),
            env,
        })
    }

    /// `lsp.bend2-lsp.settings` is sent to the server as-is through
    /// `workspace/didChangeConfiguration`. The server reads compiler settings
    /// from a top-level `"bend2-lsp"` object and lint settings from `"bend"`.
    fn language_server_workspace_configuration(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        Ok(LspSettings::for_worktree(id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.settings))
    }

    /// Renders completions as highlighted Bend code built from the signature
    /// the server puts in `detail`, for example `IO.print(text: String) -> IO(Unit)`.
    fn label_for_completion(
        &self,
        _id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        let detail = completion.detail.as_deref()?;
        if detail.contains('\n') {
            return None;
        }
        let label = completion.label.as_str();
        match completion.kind? {
            CompletionKind::Function => {
                let signature = detail.strip_prefix("def ")?;
                let name_len = signature.find('(')?;
                // After `IO.`, the server labels and filters the item as
                // `print` while the signature names `IO.print`, so match on
                // the last segment only.
                signature[..name_len].ends_with(label).then(|| CodeLabel {
                    // A body makes the snippet a complete def for the parser.
                    code: format!("def {signature}:\n  _"),
                    spans: vec![CodeLabelSpan::code_range(4..4 + signature.len())],
                    filter_range: (name_len - label.len()..name_len).into(),
                })
            }
            CompletionKind::Struct => {
                let signature = detail.strip_prefix("type ")?;
                let name_len = signature.find(['<', ' '])?;
                (&signature[..name_len] == label).then(|| CodeLabel {
                    code: format!("type {signature}:"),
                    spans: vec![CodeLabelSpan::code_range(5..5 + signature.len())],
                    filter_range: (0..name_len).into(),
                })
            }
            CompletionKind::Constructor => {
                let name_len = detail.find('{')?;
                let prefix = "type T is Type:\n  ";
                (&detail[..name_len] == label).then(|| CodeLabel {
                    code: format!("{prefix}{detail}"),
                    spans: vec![CodeLabelSpan::code_range(
                        prefix.len()..prefix.len() + detail.len(),
                    )],
                    filter_range: (0..name_len).into(),
                })
            }
            _ => None,
        }
    }

    /// Workspace symbols carry only a name and a kind; show them as the
    /// declaration they come from (`def area`, `type Shape`).
    fn label_for_symbol(&self, _id: &LanguageServerId, symbol: Symbol) -> Option<CodeLabel> {
        let name = &symbol.name;
        let (code, display, name_start) = match symbol.kind {
            SymbolKind::Function => (format!("def {name}():\n  _"), 0..4 + name.len(), 4),
            SymbolKind::Struct => (format!("type {name} is Type:"), 0..5 + name.len(), 5),
            SymbolKind::EnumMember => {
                let prefix = "type T is Type:\n  ";
                (
                    format!("{prefix}{name}{{}}"),
                    prefix.len()..prefix.len() + name.len(),
                    0,
                )
            }
            _ => return None,
        };
        Some(CodeLabel {
            code,
            spans: vec![CodeLabelSpan::code_range(display)],
            filter_range: (name_start..name_start + name.len()).into(),
        })
    }
}

zed::register_extension!(BendExtension);
