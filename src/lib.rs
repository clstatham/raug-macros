use proc_macro::TokenStream;
use quote::quote;

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
                    let #input_field_names = inputs.input_as::<#input_field_types>(#input_field_indices)?;
                    self.#input_field_names = #input_field_names.unwrap_or(self.#input_field_names);
                )*

                self.update(&inputs.env);

                #(
                    outputs.set_output_as(#output_field_indices, self.#output_field_names)?;
                )*

                Ok(())
            }
        }
    };

    expanded.into()
}
