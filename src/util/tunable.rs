#[macro_export]
macro_rules! tunable_params {

    ($($name:ident : $ty:ty = $val:literal ($min:literal..=$max:literal);)*) => {

        #[cfg(feature = "tune")]
        pub fn list_params() {
            $(
                println!(
                    "option name {} type spin default {} min {} max {}",
                    stringify!($name),
                    $name(),
                    $min,
                    $max,
                );
            )*
        }

        #[cfg(feature = "tune")]
        pub fn valid_param_name(name: &str) -> bool {
            match name {
                $(stringify!($name) => true,)*
                _ => false,
            }
        }

        #[cfg(feature = "tune")]
        pub fn set_param(name: &str, val: &str) {
            match name {
                $(
                    stringify!($name) => unsafe { *vals::$name.0.get() = val.parse().expect("Invalid param!") },
                )*
                _ => println!("info error unknown option"),
            }
        }

        #[cfg(feature = "tune")]
        pub fn print_params_ob() {
            $(
                let step = ($max as f32 - $min as f32) / 20.0;
                println!(
                    "{}, int, {:.1}, {:.1}, {:.1}, {}, 0.002",
                    stringify!($name),
                    $name() as f32,
                    $min as f32,
                    $max as f32,
                    step,
                );
            )*
        }

        #[cfg(feature = "tune")]
        mod vals {
            #![allow(non_upper_case_globals)]
            use std::cell::UnsafeCell;
            pub struct SyncUnsafeCell<T>(pub UnsafeCell<T>);

            unsafe impl<T: Sync> Sync for SyncUnsafeCell<T> {}

            $(
                pub static $name: SyncUnsafeCell<$ty> = SyncUnsafeCell(UnsafeCell::new($val));
            )*
        }

        $(
            #[cfg(feature = "tune")]
            #[inline]
            pub fn $name() -> $ty {
                unsafe { *vals::$name.0.get() }
            }

            #[cfg(not(feature = "tune"))]
            #[inline]
            pub fn $name() -> $ty {
                $val
            }
        )*
    };
}
