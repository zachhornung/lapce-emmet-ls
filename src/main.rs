// Deny usage of print and eprint as it won't have same result
// in WASI as if doing in standard program, you must really know
// what you are doing to disable that lint (and you don't know)
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

extern crate anyhow;
extern crate lapce_plugin;
extern crate serde_json;

use std::process;

use anyhow::{anyhow, Result, Error};
use lapce_plugin::{
    psp_types::{
        lsp_types::{request::Initialize, DocumentFilter, DocumentSelector, InitializeParams, Url, MessageType},
        Request,
    },
    register_plugin, LapcePlugin, VoltEnvironment, PLUGIN_RPC,
};
use serde_json::Value;

#[derive(Default)]
struct State {}

register_plugin!(State);

fn initialize(params: InitializeParams) -> Result<()> {
    let document_selector: DocumentSelector = vec![DocumentFilter {
        // lsp language id
        language: Some(String::from("html")),
        // glob pattern
        pattern: Some(String::from("**/*.{jsx,tsx}")),
        // like file:
        scheme: None,
    }];
    let mut server_args = vec![];

    // Check for user specified LSP server path
    // ```
    // [lapce-plugin-name.lsp]
    // serverPath = "[path or filename]"
    // serverArgs = ["--arg1", "--arg2"]
    // ```
    if let Some(options) = params.initialization_options.as_ref() {
        if let Some(lsp) = options.get("lsp") {
            if let Some(args) = lsp.get("serverArgs") {
                if let Some(args) = args.as_array() {
                    if !args.is_empty() {
                        server_args = vec![];
                    }
                    for arg in args {
                        if let Some(arg) = arg.as_str() {
                            server_args.push(arg.to_string());
                        }
                    }
                }
            }

            if let Some(server_path) = lsp.get("serverPath") {
                if let Some(server_path) = server_path.as_str() {
                    if !server_path.is_empty() {
                        let server_uri = Url::parse(&format!("urn:{}", server_path))?;
                        PLUGIN_RPC.start_lsp(
                            server_uri,
                            server_args,
                            document_selector,
                            params.initialization_options,
                        );
                        return Ok(());
                    }
                }
            }
        }
    }

    // Architecture check
    let _ = match VoltEnvironment::architecture().as_deref() {
        Ok("x86_64") => "x86_64",
        Ok("aarch64") => "aarch64",
        _ => return Ok(()),
    };

    // OS check
    let _ = match VoltEnvironment::operating_system().as_deref() {
        Ok("macos") => "macos",
        Ok("linux") => "linux",
        Ok("windows") => "windows",
        _ => return Ok(()),
    };
    
    // check that npm is installed
    // and throw an error if it isnt
    fn check_npm_available() -> Result<(), Error> {
        let res = process::Command::new("npm")
            .arg("--version")
            .status()?;
        Ok(())
    }
    
    fn install_emmet() -> Result<(), Error>{
        process::Command::new("npm")
            .arg("install")
            .arg("-g")
            .arg("emmet-ls")
            .status()?;
        Ok(())
    }
    
    match check_npm_available() {
        Ok(()) => match install_emmet() {
            Ok(()) => PLUGIN_RPC.window_show_message(
                MessageType::INFO,
                format!("Emmet-ls installed successfully!")
            ),
            Err(e) => {
                PLUGIN_RPC.window_show_message(
                    MessageType::ERROR,
                    format!("Emmet-ls failed to install.")
                );
                return Err(anyhow!(e))
            }
        },
        Err(e) => {
            PLUGIN_RPC.window_show_message(
                MessageType::ERROR,
                format!("Could not find npm. Npm must be available to download emmet-ls.")
            );
            return Err(anyhow!(e))
        }
    }

    let _ = match VoltEnvironment::operating_system().as_deref() {
        Ok("windows") => {
            format!("{}.exe", "[filename]")
        }
        _ => "[filename]".to_string(),
    };

    // Plugin working directory
    let volt_uri = VoltEnvironment::uri()?;
    let server_uri = Url::parse(&volt_uri)?.join("emmet")?;

    // if you want to use server from PATH
    // let server_uri = Url::parse(&format!("urn:{filename}"))?;

    // Available language IDs
    // https://github.com/lapce/lapce/blob/HEAD/lapce-proxy/src/buffer.rs#L173
    PLUGIN_RPC.start_lsp(
        server_uri,
        server_args,
        document_selector,
        params.initialization_options,
    );

    Ok(())
}

impl LapcePlugin for State {
    fn handle_request(&mut self, _id: u64, method: String, params: Value) {
        #[allow(clippy::single_match)]
        match method.as_str() {
            Initialize::METHOD => {
                let params: InitializeParams = serde_json::from_value(params).unwrap();
                if let Err(e) = initialize(params) {
                    PLUGIN_RPC.window_show_message(MessageType::ERROR, format!("plugin returned with error: {e}"))
                }
            }
            _ => {}
        }
    }
}
