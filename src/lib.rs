use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::{parse_macro_input, /* token::Token */};

use crate::error::{NaivelyTokenize, command_output_result};

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

impl ToTokens for RangeFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RangeSpindle {
    home: String,
    name: String,
    populated: bool,
    compiled: bool,
    device: Option<String>,
    msg: Option<String>,
    kernel: Option<String>,
}

static RANGE_FILES: &[(&str, &str, &str)] = &[
    ("Cargo.toml", "", include_str!("range/Cargo.toml")),
    ("rust-toolchain.toml", "", include_str!("range/rust-toolchain.toml")),
    ("config.toml", ".cargo", include_str!("range/.cargo/config.toml")),
    ("device.rs", "src", ""),
    ("lib.rs", "src", include_str!("range/src/lib.rs")),
];

impl RangeSpindle {
    fn generate(name: &str, device: RangeFn) -> Result<Self, TokenStream> {
        let spindle = PathBuf::from(KERNELS).join(name).with_extension("json");
        let new_device = device.clone().into_token_stream().to_string();
        let spindle = if spindle.exists() {
            let spindle = std::fs::read_to_string(spindle).map_err(NaivelyTokenize::naively_tokenize)?;
            let mut spindle: RangeSpindle = serde_json::from_str(&spindle).map_err(NaivelyTokenize::naively_tokenize)?;
            spindle.update_device(new_device)?;
            spindle
        } else {
            Self {
                home: String::from(KERNELS),
                name: name.into(),
                populated: false,
                compiled: false,
                device: Some(new_device),
                msg: None,
                kernel: None,
            }
        };
        let path = PathBuf::from(KERNELS).join(name);
        std::fs::create_dir_all(&path).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::create_dir_all(path.join(".cargo")).map_err(NaivelyTokenize::naively_tokenize)?;
        std::fs::create_dir_all(path.join("src")).map_err(NaivelyTokenize::naively_tokenize)?;
        if !spindle.populated {
            for (name, dir, contents) in RANGE_FILES {
                if contents.ne(&"") {
                    std::fs::write(path.join(dir).join(name), contents)
                        .map_err(NaivelyTokenize::naively_tokenize)?;
                }
            }
            let device = device.into_token_stream().to_string();
            std::fs::write(path.join("src/device.rs"), &device)
                .map_err(NaivelyTokenize::naively_tokenize)?;    
        }
        
        Ok(spindle)
    }

    fn remove_files(&mut self) -> Result<(), TokenStream> {
        let path = PathBuf::from(&self.home).join(&self.name);
        for (file, dir, _) in RANGE_FILES {
            let path = path.join(dir).join(file);
            if path.exists() {
                std::fs::remove_file(path).map_err(NaivelyTokenize::naively_tokenize)?;
            }
        }
        let Self {
            home: _home,
            name: _name,
            populated,
            compiled,
            device,
            msg,
            kernel
        } = self;
        
        *populated = false;
        *compiled = false;
        *device = None;
        *msg = None;
        *kernel = None;
        self.write()?;
        Ok(())
    }

    fn update_device(&mut self, new_device: String) -> Result<(), TokenStream> {
        let Self {
            home: _,
            name: _,
            populated,
            compiled,
            device,
            msg,
            kernel
        } = self;

        if device.as_ref().is_some_and(|device| new_device.eq(device)) {
            return Ok(())
        }
        
        *compiled = false;
        *populated = false;
        *device = Some(new_device);
        *msg = None;
        *kernel = None;
        self.remove_files()?;
        self.write()
    }

    fn write(&self) -> Result<(), TokenStream> {
        let json = serde_json::to_string_pretty(&self).map_err(NaivelyTokenize::naively_tokenize)?;
        let crate_json = PathBuf::from(&self.home).join(&self.name).with_extension("json");
        std::fs::write(crate_json, json).map_err(NaivelyTokenize::naively_tokenize)
    }

    fn compile(&mut self) -> Result<String, TokenStream> {
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
        let output = cmd.output().map_err(NaivelyTokenize::naively_tokenize)?;
        let output = match command_output_result(output) {
            Ok(output) => output,
            Err(err) => {
                self.msg = Some(err.to_string());
                self.compiled = false;
                self.write()?;
                return Err(err.naively_tokenize());
            }
        };
        self.compiled = true;
        self.write()?;
        Ok(output)
    }
}

static KERNELS: &'static str = "target/kernels/";
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

fn emit_range_kernel(_attr: RangeAttributes, item: RangeFn) -> TokenResult {
    let name = item.name();
    let mut device = item.clone();
    device.make_visible();
    device.rename("device");
    let mut spindle = RangeSpindle::generate(&name, device)?;
    // dbg!(&spindle);
    const WARNING: &'static str = "\
        #![no_std] \
        #![feature(abi_ptx)] \
        #![feature(stdsimd)] \
        #![feature(core_intrinsics)] \
        core::arch::nvptx::*; \
    ";
    const COLOR: &'static str = "\x1b[33m";
    const RESET: &'static str = "\x1b[0m";
    println!("{COLOR}{name} uses {}{}", WARNING, RESET);
    let output = spindle.compile()?;
    println!("{}", output.trim_end());
    Ok(quote::quote! {
        #item
    })
}
