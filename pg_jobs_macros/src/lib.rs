use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

// https://github.com/dtolnay/dyn-clone

#[proc_macro_derive(PGTask)]
pub fn derive_pg_task(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[typetag::serde]
        impl PGTask for #name {

            fn insert(&self, commands: &mut Commands, entity: &Entity) {
                commands.entity(*entity).insert(self.clone());
            }

            fn remove(&self, commands: &mut Commands, entity: &Entity){
                commands.entity(*entity).remove::<Self>();
            }

            fn spawn(&self, commands: &mut Commands) -> Entity {
                let entity = commands.spawn(self.clone()).id();
                return entity;
            }
        }
    };

    TokenStream::from(expanded)
}
