macro_rules! tagged_index {
    (
        pub struct $name:ident {
            const TAG_BITS: usize = $tag_bits:expr;
            $tag_v:vis tag: $tag_ty:ident,
            $index_v:vis index: $index_ty:ty,
        }
    ) => {
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Hash, AsBytes, FromBytes)]
        pub struct $name(pub u32);

        impl $name {
            const TAG_BITS: usize = $tag_bits;
            const TAG_MASK: u32 = (1u32 << Self::TAG_BITS) - 1;

            pub const fn tag(self) -> $tag_ty {
                $tag_ty::from_u32(self.0 & Self::TAG_MASK)
            }

            pub const fn index(self) -> u32 {
                const TAG_BITS: usize = $tag_bits;
                self.0 >> TAG_BITS
            }

            pub const fn new(tag: $tag_ty, value: u32) -> Self {
                assert!((tag.0 as u32 & !Self::TAG_MASK) == 0);
                //assert!((value & Self::TAG_MASK) == 0);
                Self(
                    tag.0 as u32 | (value << Self::TAG_BITS)
                )
            }
        }

        impl Debug for $name {
            fn fmt(&self, fmt: &mut Formatter) -> core::fmt::Result {
                write!(fmt, "{:?}({})", self.tag(), self.index())
            }
        }
    };
}

#[macro_export]
macro_rules! nyi {
    () => {
        println!("{}:{} >>> not yet implemented", file!(), line!());
    }
}
