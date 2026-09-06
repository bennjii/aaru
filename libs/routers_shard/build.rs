//! Fingerprints the `ShardedNetwork` on-disk schema for the cache header.
//!
//! Only `struct`, `enum`, `type` and `const` declarations are hashed, with
//! doc comments stripped and items sorted by name. Comments, formatting and
//! `impl`/`fn` changes therefore leave existing shards valid.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{Attribute, Item};

fn fnv1a(bytes: &[u8], h: u64) -> u64 {
    let mut h = h;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn strip_docs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|a| !a.path().is_ident("doc"));
}

fn collect(items: &[Item], prefix: &str, out: &mut Vec<(String, String)>) {
    for item in items {
        match item.clone() {
            Item::Struct(mut s) => {
                strip_docs(&mut s.attrs);
                for f in s.fields.iter_mut() {
                    strip_docs(&mut f.attrs);
                }
                out.push((
                    format!("{prefix}{}", s.ident),
                    s.to_token_stream().to_string(),
                ));
            }
            Item::Enum(mut e) => {
                strip_docs(&mut e.attrs);
                for v in e.variants.iter_mut() {
                    strip_docs(&mut v.attrs);
                    for f in v.fields.iter_mut() {
                        strip_docs(&mut f.attrs);
                    }
                }
                out.push((
                    format!("{prefix}{}", e.ident),
                    e.to_token_stream().to_string(),
                ));
            }
            Item::Type(mut t) => {
                strip_docs(&mut t.attrs);
                out.push((
                    format!("{prefix}{}", t.ident),
                    t.to_token_stream().to_string(),
                ));
            }
            Item::Const(mut c) => {
                strip_docs(&mut c.attrs);
                out.push((
                    format!("{prefix}{}", c.ident),
                    c.to_token_stream().to_string(),
                ));
            }
            Item::Mod(m) => {
                if let Some((_, items)) = m.content {
                    collect(&items, &format!("{prefix}{}::", m.ident), out);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let files = [
        "src/network.rs",
        "src/selection.rs",
        "src/strategy/quadtree.rs",
        "src/strategy/geohash.rs",
    ];

    let mut decls: Vec<(String, String)> = Vec::new();
    for rel in files {
        let path: &Path = &manifest.join(rel);
        let src = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("build.rs: cannot read {} — {e}", path.display()));
        let ast = syn::parse_file(&src)
            .unwrap_or_else(|e| panic!("build.rs: cannot parse {} — {e}", path.display()));
        collect(&ast.items, &format!("{rel}::"), &mut decls);
        println!("cargo:rerun-if-changed={}", path.display());
    }
    decls.sort();

    let mut h: u64 = 0xcbf29ce484222325;
    for (name, tokens) in &decls {
        h = fnv1a(name.as_bytes(), h);
        h = fnv1a(tokens.as_bytes(), h);
    }

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("format_hash.rs");
    fs::write(
        &out,
        format!("pub(crate) const FORMAT_HASH: u64 = {h}u64;\n"),
    )
    .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
