//! Original protocol technical information [here](https://github.com/kphillisjr/dpmaster/blob/master/doc/techinfo.txt).

mod parse;

const PREFIX: &[u8] = b"\xFF\xFF\xFF\xFF";

/// All appropriate commands that is possible to send to a game server.
pub mod game_server_commands;
/// All appropriate commands that is possible to send to a master server.
pub mod master_server_commands;

#[derive(Debug, thiserror::Error)]
#[error("Parse response error")]
pub enum ParseResponseError {
    #[error("The input bytes is invalid for this response type")]
    InvalidResponse,
}

macro_rules! define_checked_string {
    (
        $error_message:literal,
        $error_name:ident,
        $struct_name:ident,
        $arg_name:ident,
        $check:expr
    ) => {
        #[derive(Debug, Error)]
        #[error($error_message)]
        pub struct $error_name;

        #[derive(Debug)]
        pub struct $struct_name<'a>(Cow<'a, [u8]>);
        impl $struct_name<'_> {
            pub fn new($arg_name: Cow<'_, [u8]>) -> Result<$struct_name<'_>, $error_name> {
                if !$arg_name.iter().all($check) {
                    return Err($error_name);
                }
                Ok($struct_name($arg_name))
            }

            #[inline]
            pub fn new_unchecked($arg_name: Cow<'_, [u8]>) -> $struct_name<'_> {
                $struct_name($arg_name)
            }

            #[inline]
            pub fn get(&self) -> &Cow<'_, [u8]> {
                &self.0
            }
        }
        impl AsRef<[u8]> for $struct_name<'_> {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
        impl<'a> TryFrom<&'a [u8]> for $struct_name<'a> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: &'a [u8]) -> Result<$struct_name<'a>, Self::Error> {
                $struct_name::new(Cow::Borrowed(value))
            }
        }
        impl TryFrom<Vec<u8>> for $struct_name<'_> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                $struct_name::new(Cow::Owned(value))
            }
        }
        impl<'a> TryFrom<&'a str> for $struct_name<'a> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: &'a str) -> Result<$struct_name<'a>, Self::Error> {
                $struct_name::new(Cow::Borrowed(value.as_bytes()))
            }
        }
        impl TryFrom<String> for $struct_name<'_> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: String) -> Result<Self, Self::Error> {
                $struct_name::new(Cow::Owned(value.into_bytes()))
            }
        }
    };
}
pub(crate) use define_checked_string;

#[cfg(feature = "tokio")]
pub mod client;
