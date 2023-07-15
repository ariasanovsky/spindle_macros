use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::{parse_macro_input, token::Token};

use crate::error::{NaivelyTokenize};

mod error;
mod parse;

#[proc_macro_attribute]
pub fn range_kernel(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr = parse_macro_input!(attr as RangeAttributes);
    let item = parse_macro_input!(item as RangeFn);
    let result = emit_range_kernel(attr, item);
    into_token_stream(result)
}

type TokenResult = Result<TokenStream, TokenStream>;

fn into_token_stream(result: TokenResult) -> proc_macro::TokenStream {
    match result {
        Ok(output) => proc_macro::TokenStream::from(output),
        Err(error) => proc_macro::TokenStream::from(error),
    }
}

#[derive(Clone)]
struct RangeAttributes;

#[derive(Clone)]
struct RangeFn(syn::ItemFn);

#[derive(Debug, Serialize, Deserialize)]
struct RangeSpindle {
    home: String,
    name: String,
    device: String,
    compiled: bool,
}

impl RangeSpindle {
    fn generate(name: &str, device: RangeFn) -> Result<Self, TokenStream> {
        let crate_json = PathBuf::from(KERNELS).join(name).with_extension("json");
        if crate_json.exists() {
            let json = std::fs::read_to_string(crate_json).map_err(NaivelyTokenize::naively_tokenize)?;
            let spindle: RangeSpindle = serde_json::from_str(&json).map_err(NaivelyTokenize::naively_tokenize)?;
            return Ok(spindle);
        }
        let path = PathBuf::from(KERNELS).join(name);
        std::fs::create_dir_all(&path).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::create_dir(path.join(".cargo")).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::create_dir(path.join("src")).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::write(
            path.join("Cargo.toml"),
            include_str!("range/Cargo.toml")
        ).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::write(
            path.join("rust-toolchain.toml"),
            include_str!("range/rust-toolchain.toml")
        ).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::write(
            path.join(".cargo/config.toml"),
            include_str!("range/.cargo/config.toml")
        ).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::write(
            path.join("src/lib.rs"),
            include_str!("range/src/lib.rs")
        ).map_err(NaivelyTokenize::naively_tokenize)?;
        let device = device.0.into_token_stream().to_string();
        std::fs::write(
            path.join("src/device.rs"),
            &device
        ).map_err(NaivelyTokenize::naively_tokenize)?;
        
        let spindle = RangeSpindle {
            home: KERNELS.into(),
            name: name.into(),
            device,
            compiled: false,
        };
        spindle.serialize()?;
        Ok(spindle)
    }

    fn serialize(&self) -> Result<(), TokenStream> {
        let json = serde_json::to_string_pretty(&self).map_err(NaivelyTokenize::naively_tokenize)?;
        let crate_json = PathBuf::from(&self.home).join(&self.name).with_extension("json");
        std::fs::write(crate_json, json).map_err(NaivelyTokenize::naively_tokenize)
    }

    fn compile(&mut self) -> Result<(), TokenStream> {
        let mut cmd = std::process::Command::new("cargo");
        let home = format!("{}/{}", self.home, self.name);
        cmd.args([
            "+nightly",
            "-Z",
            "unstable-options",
            "-C",
            &home,
            "build",
            "--release",
        ]);
        dbg!(&cmd);
        let output = cmd.output().map_err(NaivelyTokenize::naively_tokenize)?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
        // .map_err(NaivelyTokenize::naively_tokenize)?;
        // dbg!(&output);
        todo!()
    }
}

static KERNELS: &'static str = "target/kernels";
// static RANGE_KERNEL: &'static str = include_str!("range/src/lib.rs");
// static RANGE_CARGO_TOML: &'static str = include_str!("range/Cargo.toml");

impl RangeFn {
    fn name(&self) -> String {
        self.0.sig.ident.to_string()
    }

    fn rename(&mut self, name: &str) {
        self.0.sig.ident = syn::Ident::new(name, self.0.sig.ident.span());
    }

    fn make_visible(&mut self) {
        self.0.vis = syn::Visibility::Public(Default::default());
    }
}

fn emit_range_kernel(attr: RangeAttributes, item: RangeFn) -> TokenResult {
    let name = item.name();
    let mut device = item.clone();
    device.make_visible();
    device.rename("device");
    let mut spindle = RangeSpindle::generate(&name, device)?;
    spindle.compile()?;
    Ok(item.0.into_token_stream())
}
