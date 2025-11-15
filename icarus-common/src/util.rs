#[macro_export]
macro_rules! define_enum {
    (
        $(#[$($attrs:meta)+])*
        $vis:vis enum $name:ident {
            $(
                $(#[$($var_attrs:meta)+])*
                $var:ident
            ),+ $(,)?
        }
    ) => {
        $(#[$($attrs)+])*
        #[repr(u8)]
        #[derive(Clone, Copy, Eq, PartialEq, $crate::util::enum_map::Enum)]
        $vis enum $name {
            $(
                $(#[$($var_attrs)+])*
                $var,
            )+
        }

        impl $name {
            $vis const COUNT: usize = <[Self]>::len(&[$(Self::$var),+]);
            $vis const ALL: &[Self; Self::COUNT] = &[$(Self::$var),+];

            #[inline]
            $vis const fn idx(self) -> u8 {
                self as u8
            }

            /// # Safety
            /// `idx < Self::COUNT` must hold.
            #[inline]
            $vis const unsafe fn from_idx_unchecked(idx: u8) -> Self {
                unsafe { ::core::mem::transmute(idx) }
            }

            #[inline]
            $vis const fn try_from_idx(idx: u8) -> ::core::option::Option<Self> {
                if (idx as usize) < Self::COUNT {
                    unsafe { Some(Self::from_idx_unchecked(idx)) }
                } else {
                    None
                }
            }

            #[inline]
            #[track_caller]
            $vis const fn from_idx(idx: u8) -> Self {
                Self::try_from_idx(idx).expect(concat!("Index out of bounds for `", stringify!($name), "`!"))
            }

            #[inline]
            $vis fn all() -> impl ::core::iter::DoubleEndedIterator<Item = Self>
                                + ::core::iter::FusedIterator
                                + ::core::iter::ExactSizeIterator
                                + ::core::clone::Clone
            {
                const { assert!(Self::COUNT < 256, concat!("Too many enum variants in `", stringify!($name), "`!")) };

                (0..Self::COUNT as u8).map(|i| unsafe { Self::from_idx_unchecked(i) })
            }
        }
    };
}

use std::ops::{Deref, DerefMut};

pub use define_enum;

#[repr(align(64))]
pub struct Align64<T>(pub T);

impl<T> Deref for Align64<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Align64<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use enum_map;