use proc_macro::TokenStream;
use quote::quote;

/// Returns the MIDI note constant for the given note name and octave.
///
/// # Examples
///
/// ```
/// let note = raug_macros::note!["C4"];
/// assert_eq!(note, 60);
///
/// let note = raug_macros::note!["C#4"];
/// assert_eq!(note, 61);
///
/// let note = raug_macros::note!["Bb3"];
/// assert_eq!(note, 58);
/// ```
#[proc_macro]
pub fn note(input: TokenStream) -> TokenStream {
    let input: syn::LitStr = syn::parse(input).unwrap();
    let input = input.value();

    let note = parse_note(&input);

    let output = quote! {
        #note
    };

    output.into()
}

/// Returns an array of MIDI notes for the given note names.
/// The note names should be separated by whitespace.
///
/// # Examples
///
/// ```
/// let notes = raug_macros::note_array!["C4 D4 E4"];
/// assert_eq!(notes, [60, 62, 64]);
/// ```
#[proc_macro]
pub fn note_array(input: TokenStream) -> TokenStream {
    let input: syn::LitStr = syn::parse(input).unwrap();
    let input = input.value();

    let notes = input.split_whitespace().map(parse_note);

    let output = quote! {
        [#(#notes),*]
    };

    output.into()
}

fn parse_note(input: &str) -> u8 {
    let input = input.trim();
    let input = input.to_uppercase();

    let mut chars = input.chars();

    let mut note: i8 = match chars.next().expect("Invalid note: empty input") {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => panic!("Invalid note: invalid note name"),
    };

    let mut octave: i8 = 0;
    let mut stop = false;
    while !stop {
        let next = chars.next();
        match next {
            Some('#') => note += 1, // keep going
            Some('B') => note -= 1, // keep going
            Some('0'..='9') => {
                let num = next.unwrap().to_digit(10).unwrap() as i8;
                octave = num + 1;
                stop = true;
            }
            Some('-') => {
                let num = chars.next().unwrap().to_digit(10).unwrap() as i8;
                octave = -num + 1;
                stop = true;
            }
            None => {
                stop = true;
            }
            _ => {
                panic!("Invalid note: unexpected character");
            }
        }
    }

    let octave = (octave)
        .checked_mul(12)
        .expect("Invalid note: octave out of range");
    let note = note
        .checked_add(octave)
        .expect("Invalid note: note out of range");

    note as u8
}

/// Derive the `Processor` trait for a struct.
///
/// The `Processor` trait is used to define a processor that can be used in a signal processing graph.
///
/// # Attributes
///
/// The following attributes can be used to specify the inputs and outputs of the processor:
///
/// - `#[input]`: Specifies that a field is an input.
/// - `#[output]`: Specifies that a field is an output.
/// - `#[processor_typetag]`: Specifies that the processor should be serializable using `typetag`.
#[proc_macro_derive(Processor, attributes(input, output, processor_typetag))]
pub fn derive_processor(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    impl_processor(&ast)
}

fn impl_processor(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (fields, typetag) = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                let typetag = ast
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("processor_typetag"));
                (fields.named.iter().collect::<Vec<_>>(), typetag)
            }
            _ => panic!("Processor must be a struct with named fields"),
        },
        _ => panic!("Processor must be a struct"),
    };

    let input_fields = fields
        .iter()
        .filter(|field| field.attrs.iter().any(|attr| attr.path().is_ident("input")))
        .collect::<Vec<_>>();

    let output_fields = fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("output"))
        })
        .collect::<Vec<_>>();

    let input_field_names = input_fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();
    let output_field_names = output_fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let input_field_types = input_fields
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();
    // let output_field_types = output_fields
    //     .iter()
    //     .map(|field| &field.ty)
    //     .collect::<Vec<_>>();

    let input_field_indices = input_fields
        .iter()
        .enumerate()
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let output_field_indices = output_fields
        .iter()
        .enumerate()
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    let mut input_field_signal_types = Vec::new();
    for field in input_fields {
        let ty = &field.ty;
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            panic!("Input fields must have a type path");
        };
        let ident = path.segments.iter().last().unwrap().ident.to_string();
        let ident = match ident.as_str() {
            "bool" => "raug::signal::SignalType::Bool",
            "f32" => "raug::signal::SignalType::Float",
            "f64" => "raug::signal::SignalType::Float",
            "Float" => "raug::signal::SignalType::Float",
            "i64" => "raug::signal::SignalType::Int",
            "MidiMessage" => "raug::signal::SignalType::Midi",
            _ => panic!("Unsupported input type: {}", ident),
        };
        let path: syn::Path = syn::parse_str(ident).unwrap();
        input_field_signal_types.push(path);
    }

    let mut output_field_signal_types = Vec::new();
    for field in output_fields {
        let ty = &field.ty;
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            panic!("Output fields must have a type path");
        };
        let ident = path.segments.iter().last().unwrap().ident.to_string();
        let ident = match ident.as_str() {
            "bool" => "raug::signal::SignalType::Bool",
            "f32" => "raug::signal::SignalType::Float",
            "f64" => "raug::signal::SignalType::Float",
            "Float" => "raug::signal::SignalType::Float",
            "i64" => "raug::signal::SignalType::Int",
            "MidiMessage" => "raug::signal::SignalType::Midi",
            _ => panic!("Unsupported output type: {}", ident),
        };
        let path: syn::Path = syn::parse_str(ident).unwrap();
        output_field_signal_types.push(path);
    }

    let typetag = if typetag {
        quote! {
            #[raug::__typetag::serde]
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #typetag
        impl raug::processor::Processor for #name {
            fn input_spec(&self) -> Vec<raug::processor::SignalSpec> {
                vec![
                    #(
                        raug::processor::SignalSpec::new(stringify!(#input_field_names), #input_field_signal_types),
                    )*
                ]
            }

            fn output_spec(&self) -> Vec<raug::processor::SignalSpec> {
                vec![
                    #(
                        raug::processor::SignalSpec::new(stringify!(#output_field_names), #output_field_signal_types),
                    )*
                ]
            }

            fn process(&mut self, inputs: raug::processor::ProcessorInputs, mut outputs: raug::processor::ProcessorOutputs) -> Result<(), raug::processor::ProcessorError> {
                #(
                    let #input_field_names = inputs.input_as::<#input_field_types>(#input_field_indices);
                )*

                for __i in 0..inputs.block_size() {
                    #(
                        self.#input_field_names = #input_field_names.and_then(|inp| inp[__i]).unwrap_or(self.#input_field_names);
                    )*
                    self.update(&inputs.env);
                    #(
                        outputs.set_output_as(#output_field_indices, __i, self.#output_field_names)?;
                    )*
                }

                Ok(())
            }
        }
    };

    expanded.into()
}

struct IterProcIoAs {
    inputs: syn::Ident,
    input_types: syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
    outputs: syn::Ident,
    output_types: syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
}

impl syn::parse::Parse for IterProcIoAs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inputs = input.parse()?;
        input.parse::<syn::Token![as]>()?;
        let input_types;
        syn::bracketed!(input_types in input);
        let input_types = input_types.parse_terminated(syn::Type::parse, syn::Token![,])?;
        input.parse::<syn::Token![,]>()?;
        let outputs = input.parse()?;
        input.parse::<syn::Token![as]>()?;
        let output_types;
        syn::bracketed!(output_types in input);
        let output_types = output_types.parse_terminated(syn::Type::parse, syn::Token![,])?;
        Ok(Self {
            inputs,
            outputs,
            input_types,
            output_types,
        })
    }
}

#[proc_macro]
pub fn iter_proc_io_as(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as IterProcIoAs);

    let inputs = input.inputs;
    let outputs = input.outputs;

    let input_count = input.input_types.len();
    let output_count = input.output_types.len();

    let mut input_idents = vec![];
    for i in 0..input_count {
        let ident = syn::Ident::new(&format!("in{}", i), proc_macro2::Span::call_site());
        input_idents.push(ident);
    }

    let mut output_idents = vec![];
    for i in 0..output_count {
        let ident = syn::Ident::new(&format!("out{}", i), proc_macro2::Span::call_site());
        output_idents.push(ident);
    }

    let start = quote! {
        let raug::processor::ProcessorInputs {
            input_specs,
            inputs,
            env,
            ..
        } = #inputs;

        let [#(#input_idents),*] = inputs else {
            panic!("Expected {} inputs, got {}", #input_count, inputs.len());
        };

        let raug::processor::ProcessorOutputs {
            output_spec,
            outputs,
            mode,
            ..
        } = #outputs;

        let [#(#output_idents),*] = outputs else {
            panic!("Expected {} outputs, got {}", #output_count, outputs.len());
        };
    };

    let mut chunks = vec![];

    for (i, (input_ident, input_typ)) in input_idents
        .iter()
        .zip(input.input_types.iter())
        .enumerate()
    {
        if let syn::Type::Path(path) = input_typ {
            if path.path.get_ident().unwrap() == "Any" {
                let chunk = quote! {
                    raug::processor::ProcessorInputs::new(
                        std::slice::from_ref(&input_specs[#i]),
                        std::slice::from_ref(#input_ident),
                        env,
                    ).iter_input(0)
                };
                chunks.push(chunk);
                continue;
            }
        }
        let chunk = quote! {
            raug::processor::ProcessorInputs::new(
                std::slice::from_ref(&input_specs[#i]),
                std::slice::from_ref(#input_ident),
                env,
            ).iter_input_as::<#input_typ>(0)?
        };
        chunks.push(chunk);
    }

    for (i, (output_ident, output_typ)) in output_idents
        .iter()
        .zip(input.output_types.iter())
        .enumerate()
    {
        if let syn::Type::Path(path) = output_typ {
            if path.path.get_ident().unwrap() == "Any" {
                let chunk = quote! {
                    raug::processor::ProcessorOutputs::new(
                        std::slice::from_ref(&output_spec[#i]),
                        std::slice::from_mut(#output_ident),
                        mode,
                    ).iter_output_mut(0)
                };
                chunks.push(chunk);
                continue;
            }
        }
        let chunk = quote! {
            raug::processor::ProcessorOutputs::new(
                std::slice::from_ref(&output_spec[#i]),
                std::slice::from_mut(#output_ident),
                mode,
            ).iter_output_mut_as::<#output_typ>(0)?
        };
        chunks.push(chunk);
    }

    let output = quote! {{
        #start

        raug::__itertools::izip!(#(#chunks),*)
    }};

    output.into()
}
