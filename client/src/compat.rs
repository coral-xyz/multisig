/// Allows you to use accounts and instruction structs from a program that
/// uses a different version of anchor-lang from this library

// todo - maybe something like this belongs in anchor itself?

#[macro_export]
macro_rules! implement_anchor_lang_compatibility {
    ($my_anchor_lang:ident $(: $visibility:ident)?) => {
        implement_anchor_lang_compatibility!{
            $my_anchor_lang,
            wrap $(: $visibility)?,
            Wrapper $(: $visibility)?
        }
    };

    (
        $my_anchor_lang:ident,
        $function_name:ident $(: $visibility:ident)?
    ) => {
        implement_anchor_lang_compatibility!{
            $my_anchor_lang,
            $function_name $(: $visibility)?,
            Wrapper $(: $visibility)?
        }
    };

    (
        $my_anchor_lang:ident,
        $function_name:ident $(: $fn_visibility:ident)?,
        $struct_name:ident $(: $st_visibility:ident)?
    ) => {
        use $my_anchor_lang::{
            ToAccountMetas as MyToAccountMetas,
            AnchorSerialize as MyAnchorSerialize,
            InstructionData as MyInstructionData
        };
        use multisig_client::anchor_client::anchor_lang::{
            ToAccountMetas as MultisigToAccountMetas,
            AnchorSerialize as MultisigAnchorSerialize,
            InstructionData as MultisigInstructionData
        };

        $($fn_visibility)? fn $function_name<T>(anything: T) -> Wrapper<T> {
            Wrapper {
                inner: anything
            }
        }

        $($st_visibility)? struct $struct_name<T> {
            inner: T,
        }

        impl<T: MyToAccountMetas> MultisigToAccountMetas for Wrapper<T> {
            fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_client::solana_sdk::instruction::AccountMeta> {
                self.inner.to_account_metas(is_signer)
            }
        }

        impl<T: MyAnchorSerialize> MultisigAnchorSerialize for Wrapper<T> {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                self.inner.serialize(writer)
            }
        }

        impl<T: MyInstructionData> MultisigInstructionData for Wrapper<T> {
            fn data(&self) -> Vec<u8> {
                self.inner.data()
            }
        }
    };
}

pub use implement_anchor_lang_compatibility;
