use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use syn::{
    parenthesized,
    parse::{Parse, ParseBuffer, ParseStream, Parser},
    parse_str, Attribute, Data, DeriveInput, Error, LitStr, Path, Result, Token, Type,
};

pub fn parse(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
    full_macro_path: &'static str,
) -> Result<Input> {
    let mut input = syn::parse_macro_input::parse::<DeriveInput>(input)?;
    let mut macro_args = syn::parse_macro_input::parse::<MacroArgs>(args)?;
    let resolver = AttributeResolver::new(full_macro_path);
    let field_args = parse_fields_attributes(&mut input, &macro_args, &resolver)?;
    return Ok(Input {
        macro_args,
        input,
        field_args,
    });
}

pub struct Input {
    pub macro_args: MacroArgs,
    pub input: DeriveInput,
    pub field_args: Vec<Vec<FieldArgs>>,
}

struct AttributeResolver {
    full_path: Path,
    name: Path,
    str_name: &'static str,
}

impl AttributeResolver {
    fn new(full_path: &'static str) -> AttributeResolver {
        // Proc macros must reside at the root of a crate.
        let i = full_path.rfind("::").unwrap();
        let str_name = &full_path[i + 2..];
        AttributeResolver {
            full_path: parse_str(full_path).unwrap(),
            name: parse_str(str_name).unwrap(),
            str_name,
        }
    }

    fn should_use(&self, path: &Path) -> bool {
        if *path == self.name || *path == self.full_path {
            return true;
        }
        false
    }
}

pub struct MacroArgs {
    pub kind: Type,
    pub skipping: bool,
    pub constructor: Option<TokenStream>,
    pub default: Option<TokenStream>, // Not used for structs
    pub error: Option<Type>,
}

impl Parse for MacroArgs {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> Result<Self> {
        let kind: Type = input.parse()?;
        let mut skipping = false;
        let mut default = None;
        let mut error = None;
        let mut constructor = None;
        let mut arg_lit: LitStr;
        loop {
            if input.parse::<Token![,]>().is_err() {
                break;
            }
            let ident: Ident = input.parse()?;
            if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
            }
            let string_ident = ident.to_string();
            if string_ident == "skipping" {
                skipping = true;
            } else if string_ident == "default" {
                arg_lit = input.parse()?;
                default = Some(parse_str(&arg_lit.value())?);
            } else if string_ident == "error" {
                arg_lit = input.parse()?;
                error = Some(parse_str(&arg_lit.value())?);
            } else if string_ident == "constructor" {
                arg_lit = input.parse()?;
                constructor = Some(parse_str(&arg_lit.value())?);
            } else {
                return Err(Error::new(
                    ident.span(),
                    "expected one of: `constructor`, `default`, `error`, `skipping`",
                ));
            }
        }
        Ok(MacroArgs {
            kind,
            skipping,
            constructor,
            default,
            error,
        })
    }
}

#[derive(Default)]
pub struct FieldArgs {
    pub kind: Option<Type>,
    pub skip: bool,
    pub include: bool,
    pub rename: Option<Ident>,
    pub with: Option<TokenStream>,
}

impl Parse for FieldArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let rename_with = "`rename` and `with` are mutually exclusive";
        let include_skip = "`include` and `skip` are mutually exclusive";
        let mut attr = FieldArgs::default();
        if input.is_empty() {
            return Ok(attr);
        }
        let content;
        parenthesized!(content in input);
        loop {
            let ident: Ident = content.parse()?;
            let string_ident = ident.to_string();
            if string_ident == "rename" {
                if attr.with.is_some() {
                    return Err(Error::new(ident.span(), rename_with));
                }
                content.parse::<Token![=]>()?;
                let arg_lit: LitStr = content.parse()?;
                attr.rename = Some(parse_str(&arg_lit.value())?);
            } else if string_ident == "skip" {
                attr.skip = true;
                if attr.include {
                    return Err(Error::new(ident.span(), include_skip));
                }
            } else if string_ident == "with" {
                content.parse::<Token![=]>()?;
                let arg_lit: LitStr = content.parse()?;
                attr.with = Some(parse_str(&arg_lit.value())?);
            } else if string_ident == "include" {
                attr.include = true;
                if attr.skip {
                    return Err(Error::new(ident.span(), include_skip));
                }
            } else {
                match parse_str(&ident.to_string()) {
                    Ok(kind) => attr.kind = Some(kind),
                    Err(_) => {
                        return Err(Error::new(
                            ident.span(),
                            "expected one of: `include`, `rename`, `skip`, `with` or a type",
                        ));
                    },
                }
            }
            if content.parse::<Token![,]>().is_err() {
                break;
            }
        }
        if let Ok(tt) = input.parse::<TokenTree>() {
            return Err(Error::new(tt.span(), "unexpected"));
        }
        Ok(attr)
    }
}

fn parse_fields_attributes(
    input: &mut DeriveInput,
    macro_args: &MacroArgs,
    resolver: &AttributeResolver,
) -> Result<Vec<Vec<FieldArgs>>> {
    let mut error = None;
    let mut parser =
        |attrs: &mut Vec<Attribute>| match parse_field_attributes(macro_args, attrs, resolver) {
            Ok(ok) => ok,
            Err(e) => {
                error = Some(e);
                vec![FieldArgs::default()]
            },
        };
    let args = match &mut input.data {
        Data::Struct(s) => {
            if macro_args.default.is_some() {
                return Err(Error::new(
                    Span::call_site(),
                    "`default` does nothing for structs",
                ));
            }
            s.fields.iter_mut().map(|x| parser(&mut x.attrs)).collect()
        },
        Data::Enum(e) => e
            .variants
            .iter_mut()
            .map(|x| parser(&mut x.attrs))
            .collect(),
        Data::Union(u) => u
            .fields
            .named
            .iter_mut()
            .map(|x| parser(&mut x.attrs))
            .collect(),
    };
    if let Some(e) = error {
        return Err(e);
    }
    Ok(args)
}

fn parse_field_attributes(
    macro_args: &MacroArgs,
    attrs: &mut Vec<Attribute>,
    resolver: &AttributeResolver,
) -> Result<Vec<FieldArgs>> {
    let mut args = Vec::new();
    let mut i = 0;
    while i < attrs.len() {
        if resolver.should_use(&attrs[i].path) {
            let attr = attrs.remove(i);
            let mut arg = syn::parse_macro_input::parse::<FieldArgs>(attr.tokens.into())?;

            if macro_args.skipping && arg.skip {
                return Err(Error::new(
                    Span::call_site(),
                    "`include` not allowed when including",
                ));
            } else if !macro_args.skipping && arg.include {
                return Err(Error::new(
                    Span::call_site(),
                    "`skip` not allowed when skipping",
                ));
            }
            args.push(arg);
            continue;
        }
        i += 1;
    }
    Ok(args)
}
