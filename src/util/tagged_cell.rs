//! Tagged cell type that allows for runtime data initialization with subsequent zero cost accesses.
//! Implementation adapted from <https://www.hardmo.de/article/2021-03-14-zst-proof-types.md#proof-of-work>

use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, ptr, sync::Once};

use zerocopy::FromZeros;

pub struct TaggedCell<T, Tag> {
    data: UnsafeCell<MaybeUninit<T>>,
    once: Once,
    tag: PhantomData<Tag>,
}

// A TaggedCell may be destroyed on a different thread than the one that initialized
// it, so it can only be Sync if the inner type is both Sync and Send.
unsafe impl<T: Sync + Send, Tag> Sync for TaggedCell<T, Tag> {}

/// A ZST proof that a [`TaggedCell`] of a specific tag type has been initialized.
/// The only way to acquire this proof is by calling [`TaggedCell::init`] or [`TaggedCell::init_inplace`].
#[derive(Clone, Copy)]
pub struct Init<Tag>(PhantomData<Tag>);

impl<T, Tag> TaggedCell<T, Tag> {
    /// Make an uninitialized cell.
    ///
    /// # Safety
    /// This must only be called once for each `Tag` type.
    pub const unsafe fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            once: Once::new(),
            tag: PhantomData,
        }
    }

    /// If the cell is not yet initialized, initialize it with the value returned from `f()`.
    pub fn init(&self, f: impl FnOnce() -> T) -> Init<Tag> {
        // SAFETY: `call_once()` ensures only one thread can write to `self.data`, and no
        // other threads can read from `self.data`, because they cannot have a tag before
        // `call_once()` completes.
        self.once
            .call_once(|| unsafe { self.data.get().write(MaybeUninit::new(f())) });
        Init(PhantomData)
    }

    /// Same as [`TaggedCell::init()`], but constructing the value in place. To prevent accessing
    /// uninitialized memory, this is only callable on types for which all-zeros is a valid bit
    /// pattern. The cell value is then fully zeroed before being passed to `f`.
    pub fn init_inplace(&self, f: impl FnOnce(&mut T)) -> Init<Tag>
    where
        T: FromZeros,
    {
        // SAFETY: Same as above.
        self.once.call_once(|| unsafe {
            let data = &mut *self.data.get();
            ptr::write_bytes(data, 0, 1);
            f(data.assume_init_mut());
        });
        Init(PhantomData)
    }

    pub fn get(&self, _: Init<Tag>) -> &T {
        // SAFETY: Existence of the tag means the data is already fully initialized.
        unsafe { (&*self.data.get()).assume_init_ref() }
    }
}

#[macro_export]
macro_rules! tagged_cell {
    ($(#[$meta:meta])* static $name:ident : TaggedCell<$type:ty, _> = TaggedCell::new();) => {
        #[allow(non_snake_case)]
        mod $name {
            #[allow(dead_code)]
            #[derive(Clone, Copy)]
            pub struct TagType;
        }

        $(#[$meta])*
        static $name: $crate::util::tagged_cell::TaggedCell<$type, self::$name::TagType> = unsafe { $crate::util::tagged_cell::TaggedCell::new() };
    };
    ($(#[$meta:meta])* static $name:ident : TaggedCell<$type:ty, $vis:vis $tag:ident> = TaggedCell::new();) => {
        $crate::tagged_cell!(static $name: TaggedCell<$type, _> = TaggedCell::new(););
        $vis type $tag = $crate::util::tagged_cell::Init<self::$name::TagType>;
    }
}
