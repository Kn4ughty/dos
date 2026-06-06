// copied from https://docs.rs/enum-try-from/latest/src/enum_try_from/lib.rs.html#187-207
macro_rules! tryfrom {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $( $(#[$vmeta:meta])* $variant:ident = $val:expr,)*
    }, $type:ty) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$vmeta])* $variant = $val,)*
        }


        impl TryFrom<$type> for $name {
            type Error = $type;

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

// This did my head in to write.
// Rust macros are hard
macro_rules! tryfrom2arg {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $( $(#[$vmeta:meta])* $variant:ident ($name2:ident) = $val:expr,)*
    }, $type:ty) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$vmeta])* $variant ($name2) = $val,)*
        }


        impl TryFrom<($type, $type)> for $name {
            type Error = $type;

            fn try_from(tup: ($type,$type)) -> Result<Self, Self::Error> {
                let (v1, v2) = tup;

                match v1 {
                    $($val => {
                        Ok($name::$variant($name2::try_from(v2)?))
                    },)*
                    _ => Err(v1)
                }
            }
        }
    }
}
pub(crate) use tryfrom2arg;
