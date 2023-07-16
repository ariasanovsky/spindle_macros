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

static RANGE_FILES: &[(&str, &str, &str)] = &[
    ("Cargo.toml", "", include_str!("range/Cargo.toml")),
    ("rust-toolchain.toml", "", include_str!("range/rust-toolchain.toml")),
    ("config.toml", ".cargo", include_str!("range/.cargo/config.toml")),
    ("device.rs", "src", ""),
    ("lib.rs", "src", include_str!("range/src/lib.rs")),
    ("kernel.rs", "target/nvptx64-nvidia-cuda/release", ""),
];

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

impl RangeSpindle {
    fn generate(name: &str, device: &RangeFn) -> Result<Self, TokenStream> {
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
        match command_output_result(output) {
            Ok(output) => {
                let Self {
                    home,
                    name,
                    populated: _,
                    compiled,
                    device: _,
                    msg,
                    kernel
                } = self;
                *compiled = true;
                *msg = Some(output.clone());
                let _kernel = PathBuf::from(&home)
                    .join(name)
                    .join("target")
                    .join("nvptx64-nvidia-cuda")
                    .join("release")
                    .join("kernel.ptx");
                *kernel = Some(
                    std::fs::read_to_string(&_kernel)
                    .map_err(NaivelyTokenize::naively_tokenize)?
                );
                self.write()?;
                Ok(output)
            },
            Err(err) => {
                let Self {
                    home: _,
                    name: _,
                    populated: _,
                    compiled,
                    device: _,
                    msg,
                    kernel
                } = self;
                *compiled = false;
                *msg = Some(err.to_string());
                *kernel = None;
                self.write()?;
                return Err(err.naively_tokenize());
            }
        }
    }
}

static KERNELS: &'static str = "target/kernels/";
// static RANGE_KERNEL: &'static str = include_str!("range/src/lib.rs");
// static RANGE_CARGO_TOML: &'static str = include_str!("range/Cargo.toml");

fn camel_word(s: &str) -> String {
    let mut chars = s.chars();
    let mut camel = if let Some(c) = chars.next() {
        String::from(c).to_uppercase()
    } else {
        return String::new();
    };
    chars.map(char::to_lowercase)
    .for_each(|c| {
        camel = format!("{camel}{c}");
    });
    camel
}

fn snake_to_camel(s: &str) -> String {
    let s = s.split('_')
    .map(camel_word)
    .collect::<Vec<_>>();
    s.join("")
}


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
    let mut spindle = RangeSpindle::generate(&name, &device)?;
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

    let name = &item.0.sig.ident;
    let launch_name = syn::Ident::new(
        &format!("_{name}"),
        item.0.sig.ident.span()
    );

    let input_type = match item.0.sig.inputs.first().unwrap() {
        syn::FnArg::Receiver(_) => todo!("Range fn is not a method"),
        syn::FnArg::Typed(p) => &p.ty,
    };

    let return_type = match &device.0.sig.output {
        syn::ReturnType::Default => unreachable!("RangeFn has a return type"),
        syn::ReturnType::Type(_, return_type) => return_type,
    };

    let trait_name = syn::Ident::new(
        &format!("_{}", snake_to_camel(&item.name())),
        item.0.sig.ident.span()
    );

    let range_trait = quote::quote! {
        trait #trait_name {
            type Returns;
            unsafe fn #name (&self) -> Result<Self::Returns, spindle::range::Error>;
        }
    };

    dbg!(&range_trait.to_string());

    let ptx_path = syn::LitStr::new(
        &format!("target/kernels/{}/target/nvptx64-nvidia-cuda/release/kernel.ptx", name),
        name.span()
    );
    
    // let foo = quote::quote! {
    //     #ptx_path
    // };
    // dbg!(foo.to_string());

    println!("aadfhasdkj;fnsadklfns\n\n\n");

    let int_impl = quote::quote! {
        impl #trait_name for #input_type {
            type Returns = Vec<#return_type>;
            unsafe fn #name (&self) -> Result<Self::Returns, spindle::range::Error> {
                use cudarc::{driver::{CudaDevice, DriverError, LaunchAsync, LaunchConfig}, nvrtc::Ptx};
                let dev = CudaDevice::new(0).unwrap(); //.map_err(Into::into)?;
                dev.load_ptx(
                    Ptx::from_file(#ptx_path),
                    "kernel",
                    &["kernel"]
                ).unwrap(); // .map_err(Into::into)?;
                let f = dev.get_func("kernel", "kernel").unwrap(); // .map_err(Into::into)?;
                
                let n = *self as usize; //.map_err(Into::into)?;
                let layout = core::alloc::Layout::array::<#return_type>(n)
                    .unwrap(); //.map_err(Into::into)?
                let mut out_host_ptr = std::alloc::alloc(layout.clone());
                let out_host_vec = if out_host_ptr.is_null() {
                    std::alloc::dealloc(out_host_ptr, layout);
                    return Err(spindle::range::Error::AllocationFailed);
                } else {
                    Vec::from_raw_parts(out_host_ptr as *mut #return_type, n, n)
                };
                // let out_host = unsafe { Box::from_raw(out_host as *mut [#return_type]) };
                let mut out_dev = dev.htod_copy(out_host_vec).unwrap(); //.map_err(Into::into)?;

                let config = LaunchConfig::for_num_elems(*self as u32);
                f.launch(config, (&mut out_dev, *self as i32)).unwrap(); //.map_err(Into::into)?;
                let out_host_2 = dev.sync_reclaim(out_dev).unwrap(); //.map_err(Into::into)?;
                Ok(out_host_2)
            }
        }
    };

    println!("{}", &int_impl.to_string());

    let launcher = quote::quote! {
        unsafe fn #launch_name <const N: usize>() -> Box<[ #return_type ; N ]> {
            use cudarc::{driver::{CudaDevice, DriverError, LaunchAsync, LaunchConfig}, nvrtc::Ptx};
            let dev = CudaDevice::new(0).unwrap();
            dev.load_ptx(
                Ptx::from_file(#ptx_path),
                "kernel",
                &["kernel"]
            ).unwrap();
            let f = dev.get_func("kernel", "kernel").unwrap();
            
            // const LAYOUT: core::alloc::Layout = core::alloc::Layout::new::< [#return_type; N] >();
            // let host_output: [#return_type; N] = unsafe { core::mem::MaybeUninit::uninit().assume_init() };
            // dbg!(&host_output);
            // let mut dev_output = dev.htod_copy(host_output.into()).unwrap();
            let config = LaunchConfig::for_num_elems(N as u32);
            
            
            let mut dev_output: cudarc::driver::CudaSlice< #return_type > = 
                unsafe { dev.alloc(N) }.unwrap();

            unsafe { f.launch(config, (&mut dev_output, N as i32)) }.unwrap();
            dev.synchronize().unwrap();
            // dbg!(&dev_output);
            let host_output = dev.sync_reclaim(dev_output).unwrap();
            // dbg!(&host_output);
            host_output.try_into().unwrap()
            // todo!("clam chowder")
        }
    };
    // let out = quote::quote! {
    //     #item
    //     #launcher
    // };
    // println!("{}", out.to_string());

    Ok(quote::quote! {
        #item
        #range_trait
        #int_impl
        #launcher
    })
}
