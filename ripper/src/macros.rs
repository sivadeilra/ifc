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
            pub fn tag(self) -> $tag_ty {
                const TAG_BITS: usize = $tag_bits;
                const TAG_MASK: u32 = (1u32 << TAG_BITS) - 1;
                $tag_ty(self.0 & TAG_MASK)
            }

            pub fn index(self) -> u32 {
                const TAG_BITS: usize = $tag_bits;
                self.0 >> TAG_BITS
            }
        }

        impl Debug for $name {
            fn fmt(&self, fmt: &mut Formatter) -> core::fmt::Result {
                write!(fmt, "{:?}({})", self.tag(), self.index())
            }
        }
    };
}
