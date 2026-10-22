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
        #[derive(Clone, Copy, Eq, PartialEq, ::enum_map::Enum)]
        $vis enum $name {
            $(
                $(#[$($var_attrs)+])*
                $var,
            )+
        }

        impl $name {
            $vis const COUNT: usize = <[Self]>::len(&[$(Self::$var),+]);
            $vis const ALL: &[Self; Self::COUNT] = &[$(Self::$var),+];


            $vis const fn idx(self) -> u8 {
                self as u8
            }

            /// # Safety
            /// `idx < Self::COUNT` must hold.

            $vis const unsafe fn from_idx_unchecked(idx: u8) -> Self {
                unsafe { ::core::mem::transmute(idx) }
            }


            $vis const fn try_from_idx(idx: u8) -> ::core::option::Option<Self> {
                if (idx as usize) < Self::COUNT {
                    unsafe { Some(Self::from_idx_unchecked(idx)) }
                } else {
                    None
                }
            }


            #[track_caller]
            $vis const fn from_idx(idx: u8) -> Self {
                Self::try_from_idx(idx).expect(concat!("Index out of bounds for `", stringify!($name), "`!"))
            }


            $vis fn all() -> impl ::core::iter::DoubleEndedIterator<Item = Self>
                                + ::core::iter::FusedIterator
                                + ::core::iter::ExactSizeIterator
                                + ::core::clone::Clone
            {
                const { assert!(Self::COUNT < 256, concat!("Too many enum variants in `", stringify!($name), "`!")) };

                (0..Self::COUNT as u8).map(|i| unsafe { Self::from_idx_unchecked(i) })
            }
        }

        impl<T> ::std::ops::Index<$name> for [T; { $name::COUNT }] {
            type Output = T;


            fn index(&self, index: $name) -> &T {
                self.index(index.idx() as usize)
            }
        }

        impl<T> ::std::ops::IndexMut<$name> for [T; { $name::COUNT }] {

            fn index_mut(&mut self, index: $name) -> &mut T {
                self.index_mut(index.idx() as usize)
            }
        }
    };
}
