//! A simple crate containing the [`TypeUlid`] trait to allow associating [`Ulid`][ulid::Ulid]
//! identifiers with Rust types.
//!
//! # Example
//!
//! ```
//! # use type_ulid::TypeUlid;
//! #[derive(TypeUlid)]
//! #[ulid = "01GNDP9Y66JGBTGTX4XK6M32YB"]
//! struct MyStruct;
//! ```

pub use type_ulid_macros::TypeUlid;

pub use ulid::Ulid;

/// Associates a [`Ulid`] ID to a Rust type.
///
/// > **⚠️ Warning:** there is nothing enforcing that the [`Ulid`]s returned by different types will
/// > be different.
pub trait TypeUlid {
    /// Get the type's [`Ulid`].
    fn ulid() -> Ulid;
}

/// Helper to implement [`TypeUlid`] for a given type.
macro_rules! impl_ulid {
    ($t:ty, $ulid:expr) => {
        impl TypeUlid for $t {
            fn ulid() -> Ulid {
                Ulid($ulid)
            }
        }
    };
}

impl_ulid!(u8, 2021656255915497896209715855619467037);
impl_ulid!(u16, 2021656286850311440545371989823754456);
impl_ulid!(u32, 2021656307245333748489317905921628405);
impl_ulid!(u64, 2021656323508136892094714468002888594);
impl_ulid!(u128, 2021656335852505155406924275550256689);
impl_ulid!(i8, 2021656350234248601146822538398787257);
impl_ulid!(i16, 2021656360083645642982176250630573575);
impl_ulid!(i32, 2021656369452494039486156781916363945);
impl_ulid!(i64, 2021656381591958783376686378772853584);
impl_ulid!(i128, 2021656392493396182393399991976868589);
impl_ulid!(str, 2021656464899998250358740287628939875);
impl_ulid!(char, 2021656477744412248036447704707663978);
impl_ulid!(std::path::Path, 2021656526298641269718831147547183140);
impl_ulid!(std::path::PathBuf, 2021656533964195260964832532224399014);
impl_ulid!(String, 2021656411366161332396897323242175745);
impl_ulid!(std::ffi::CStr, 2021656593778807277069547360976271384);
impl_ulid!(std::ffi::CString, 2021656608266244812738352417994302175);
impl_ulid!(std::ffi::OsStr, 2021656632874440750318571899824814504);
impl_ulid!(std::ffi::OsString, 2021656640805438832313622968989918986);
impl_ulid!(std::time::Duration, 2021656695577227212934222356752834404);
impl_ulid!((), 2021656729314635244986430849253282093);
