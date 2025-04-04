use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

// https://github.com/dtolnay/dyn-clone

#[proc_macro_derive(PGTask)]
pub fn derive_pg_task(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl PGTask for #name {

            fn insert_task(&self, commands: &mut Commands, entity: &Entity) {
                commands.entity(*entity).insert(self.clone());
            }

            fn remove(&self, commands: &mut Commands, entity: &Entity){
                commands.entity(*entity).remove::<Self>();
            }

            fn spawn_with_task(&self, commands: &mut Commands) -> Entity {
                commands.spawn(self.clone()).id()
            }
        }
    };

    TokenStream::from(expanded)
}
