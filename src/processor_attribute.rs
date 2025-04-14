use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parser, parse_macro_input, punctuated::Punctuated};

struct ProcessorArg {
    name: syn::Ident,
    ty: syn::Type,
}

impl ToTokens for ProcessorArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ProcessorArg { name, ty, .. } = self;
        tokens.extend(quote! {
            #name: #ty,
        });
    }
}

pub fn processor_attribute(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated.parse(attr);

    let mut extra_derives = vec![];
    if let Ok(args) = args {
        for arg in args.iter() {
            if let syn::Meta::List(meta_list) = arg {
                if meta_list.path.is_ident("derive") {
                    meta_list
                        .parse_nested_meta(|meta| {
                            let ident = meta.path.get_ident().unwrap();
                            extra_derives.push(ident.clone());
                            Ok(())
                        })
                        .unwrap();
                } else {
                    return syn::Error::new_spanned(
                        meta_list.path.clone(),
                        "Unknown attribute. Only `derive` is supported.",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    }

    let item = parse_macro_input!(item as syn::ItemFn);
    let vis = item.vis.clone();
    let (ig, tg, wc) = item.sig.generics.split_for_impl();
    let func_name = item.sig.ident.clone().to_string();
    let struct_name = func_name.to_case(Case::Pascal);
    let struct_name = format_ident!("{}", struct_name);
    let attrs = item.attrs.clone();

    let mut proc_env_ident = None;

    let mut state = vec![];
    let mut input = vec![];
    let mut output = vec![];
    let mut struct_destructure = vec![];
    let mut clone_inputs = vec![];
    let mut input_spec = vec![];
    let mut output_spec = vec![];
    let mut create_output_buffers = vec![];
    let mut get_inputs = vec![];
    let mut assign_inputs = vec![];
    let mut assign_outputs = vec![];

    for arg in item.sig.inputs.iter() {
        if let syn::FnArg::Typed(arg) = arg {
            if let syn::Type::Path(type_path) = &*arg.ty {
                if type_path.path.is_ident("ProcEnv") {
                    if proc_env_ident.is_some() {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "Only one ProcEnv argument is allowed",
                        )
                        .to_compile_error()
                        .into();
                    }
                    proc_env_ident = Some(arg.pat.clone());
                    continue;
                }
            }
            if let Some(attr) = arg.attrs.first() {
                if attr.path().is_ident("state") {
                    let name = if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                        pat_ident.ident.clone()
                    } else {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "State argument must be a named identifier",
                        )
                        .to_compile_error()
                        .into();
                    };
                    let ty = *arg.ty.clone();
                    state.push(ProcessorArg { name, ty });
                } else if attr.path().is_ident("input") {
                    let name = if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                        pat_ident.ident.clone()
                    } else {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "Input argument must be a named identifier",
                        )
                        .to_compile_error()
                        .into();
                    };
                    let ty;
                    if let syn::Type::Group(group) = &*arg.ty {
                        if let syn::Type::Reference(reference) = &*group.elem {
                            if reference.mutability.is_some() {
                                return syn::Error::new_spanned(
                                    arg.pat.clone(),
                                    "Input argument must be immutable reference",
                                )
                                .to_compile_error()
                                .into();
                            }
                            ty = *reference.elem.clone();
                        } else {
                            return syn::Error::new_spanned(
                                arg.pat.clone(),
                                "Input argument must be a reference inside a group",
                            )
                            .to_compile_error()
                            .into();
                        }
                    } else if let syn::Type::Reference(reference) = &*arg.ty {
                        if reference.mutability.is_some() {
                            return syn::Error::new_spanned(
                                arg.pat.clone(),
                                "Input argument must be immutable reference",
                            )
                            .to_compile_error()
                            .into();
                        }
                        ty = *reference.elem.clone();
                    } else {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "Input argument must be a reference",
                        )
                        .to_compile_error()
                        .into();
                    }
                    input.push(ProcessorArg { name, ty });
                } else if attr.path().is_ident("output") {
                    let name = if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                        pat_ident.ident.clone()
                    } else {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "Output argument must be a named identifier",
                        )
                        .to_compile_error()
                        .into();
                    };
                    let ty;
                    if let syn::Type::Group(group) = &*arg.ty {
                        if let syn::Type::Reference(reference) = &*group.elem {
                            if reference.mutability.is_none() {
                                return syn::Error::new_spanned(
                                    arg.pat.clone(),
                                    "Output argument must be mutable reference",
                                )
                                .to_compile_error()
                                .into();
                            }
                            ty = *reference.elem.clone();
                        } else {
                            return syn::Error::new_spanned(
                                arg.pat.clone(),
                                "Output argument must be a reference inside a group",
                            )
                            .to_compile_error()
                            .into();
                        }
                    } else if let syn::Type::Reference(reference) = &*arg.ty {
                        if reference.mutability.is_none() {
                            return syn::Error::new_spanned(
                                arg.pat.clone(),
                                "Output argument must be mutable reference",
                            )
                            .to_compile_error()
                            .into();
                        }
                        ty = *reference.elem.clone();
                    } else {
                        return syn::Error::new_spanned(
                            arg.pat.clone(),
                            "Output argument must be a reference",
                        )
                        .to_compile_error()
                        .into();
                    }
                    output.push(ProcessorArg { name, ty });
                } else {
                    return syn::Error::new_spanned(
                        attr.path().clone(),
                        "Unknown attribute. Only `state`, `input`, and `output` are supported.",
                    )
                    .to_compile_error()
                    .into();
                }
            } else {
                return syn::Error::new_spanned(
                    arg.pat.clone(),
                    "Expected a function argument with attributes",
                )
                .to_compile_error()
                .into();
            }
        } else {
            return syn::Error::new_spanned(
                arg.clone(),
                "Expected a function argument with attributes",
            )
            .to_compile_error()
            .into();
        }
    }

    let proc_env_decl = if let Some(proc_env_ident) = proc_env_ident {
        quote! {
            let #proc_env_ident = env;
        }
    } else {
        quote! {}
    };

    let mut struct_fields = vec![];
    for arg in state.iter() {
        let ProcessorArg { name, ty, .. } = arg;
        let ty = if let syn::Type::Reference(ty) = ty {
            if ty.mutability.is_none() {
                return syn::Error::new_spanned(
                    ty.clone(),
                    "State argument must be mutable reference",
                )
                .to_compile_error()
                .into();
            }
            ty.elem.clone()
        } else {
            return syn::Error::new_spanned(
                ty.clone(),
                "State argument must be a mutable reference",
            )
            .to_compile_error()
            .into();
        };
        struct_fields.push(quote! {
            pub #name: #ty,
        });
        struct_destructure.push(quote! {
            #name,
        });
    }

    for (arg_index, arg) in input.iter().enumerate() {
        let ProcessorArg { name, ty } = arg;

        struct_fields.push(quote! {
            pub #name: #ty,
        });

        struct_destructure.push(quote! {
            #name,
        });
        clone_inputs.push(quote! {
            let #name = &*#name;
        });
        input_spec.push(quote! {
            raug::processor::io::SignalSpec::new(stringify!(#name), <#ty as raug::signal::Signal>::signal_type())
        });
        get_inputs.push(quote! {
            let #name = inputs.input_as::<#ty>(#arg_index);
        });
        assign_inputs.push(quote! {
            if let Some(#name) = #name.map(|inp| &inp[__i]) {
                self.#name.clone_from(#name);
            }
        });
    }

    for (arg_index, arg) in output.iter().enumerate() {
        let ProcessorArg { name, ty } = arg;

        struct_fields.push(quote! {
            pub #name: #ty,
        });
        struct_destructure.push(quote! {
            #name,
        });
        output_spec.push(quote! {
            raug::processor::io::SignalSpec::new(stringify!(#name), <#ty as raug::signal::Signal>::signal_type())
        });
        create_output_buffers.push(quote! {
            raug::signal::type_erased::ErasedBuffer::zeros::<#ty>(size)
        });
        assign_outputs.push(quote! {
            outputs.set_output_as::<#ty>(#arg_index, __i, &self.#name)?;
        });
    }

    let struct_def = quote! {
        #(#attrs)*
        #[derive(Clone, #(#extra_derives),*)]
        #[allow(missing_docs)]
        #vis struct #struct_name #tg #wc {
            #(#struct_fields)*
        }
    };

    let body = item.block.clone();

    let struct_update_impl = quote! {
        impl #ig #struct_name #tg #wc {
            #[doc = "Update function for the processor."]
            pub fn update(&mut self, env: raug::processor::io::ProcEnv) -> raug::processor::ProcResult<()> {
                let #struct_name { #(#struct_destructure)* } = self;
                #(#clone_inputs)*
                #proc_env_decl
                #body
            }
        }
    };

    let fn_name = item.sig.ident.clone();
    let mut fn_args = item.sig.inputs.clone();
    // remove the attributes from the function arguments
    for arg in fn_args.iter_mut() {
        if let syn::FnArg::Typed(arg) = arg {
            arg.attrs.retain(|attr| {
                !(attr.path().is_ident("state")
                    || attr.path().is_ident("input")
                    || attr.path().is_ident("output"))
            });
        }
    }

    let outputs = item.sig.output.clone();

    let fn_def = quote! {
        #(#attrs)*
        #vis fn #fn_name #tg(#fn_args) #outputs #wc  {
            #body
        }
    };

    let processor_impl = quote! {
        impl #ig raug::processor::Processor for #struct_name #tg #wc {
            fn name(&self) -> &str {
                stringify!(#struct_name)
            }

            fn input_spec(&self) -> Vec<raug::processor::io::SignalSpec> {
                vec![#(#input_spec),*]
            }

            fn output_spec(&self) -> Vec<raug::processor::io::SignalSpec> {
                vec![#(#output_spec),*]
            }

            fn create_output_buffers(&self, size: usize) -> Vec<raug::signal::type_erased::ErasedBuffer> {
                vec![#(#create_output_buffers),*]
            }

            fn process(&mut self, inputs: raug::processor::io::ProcessorInputs, mut outputs: raug::processor::io::ProcessorOutputs) -> Result<(), raug::processor::ProcessorError> {
                #(#get_inputs)*

                for __i in 0..inputs.block_size() {
                    #(#assign_inputs)*
                    self.update(inputs.env)?;
                    #(#assign_outputs)*
                }

                Ok(())
            }
        }
    };

    quote! {
        #fn_def
        #struct_def
        #struct_update_impl
        #processor_impl
    }
    .into()
}
