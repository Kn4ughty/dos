macro_rules! tryfrom {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($variant:ident = $val:expr,)*
    }, $type:ty) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant = $val,)*
        }


        impl TryFrom<u32> for $name {
            type Error = u32;

            fn try_from(value: $type) -> Result<Self, Self::Error> {
                match value {
                    $(x if x == $name::$variant as $type => Ok($name::$variant),)*
                    _ => Err(value)
                }
            }
        }
    }
}
pub(crate) use tryfrom;
