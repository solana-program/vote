#[cfg(test)]
use arbitrary::{Arbitrary, Unstructured};
use {
    serde::{Deserialize, Serialize},
    solana_frozen_abi_macro::AbiExample,
};

// this is how many epochs a voter can be remembered for slashing
const MAX_ITEMS: usize = 32;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, AbiExample)]
pub struct CircBuf<I> {
    buf: [I; MAX_ITEMS],
    /// next pointer
    idx: usize,
    is_empty: bool,
}

impl<I: Default + Copy> Default for CircBuf<I> {
    fn default() -> Self {
        Self {
            buf: [I::default(); MAX_ITEMS],
            idx: MAX_ITEMS
                .checked_sub(1)
                .expect("`MAX_ITEMS` should be positive"),
            is_empty: true,
        }
    }
}

impl<I> CircBuf<I> {
    pub fn append(&mut self, item: I) {
        // remember prior delegate and when we switched, to support later slashing
        self.idx = self
            .idx
            .checked_add(1)
            .and_then(|idx| idx.checked_rem(MAX_ITEMS))
            .expect("`self.idx` should be < `MAX_ITEMS` which should be non-zero");

        self.buf[self.idx] = item;
        self.is_empty = false;
    }

    pub fn buf(&self) -> &[I; MAX_ITEMS] {
        &self.buf
    }

    pub fn last(&self) -> Option<&I> {
        if !self.is_empty {
            Some(&self.buf[self.idx])
        } else {
            None
        }
    }
}

#[cfg(test)]
impl<'a, I: Default + Copy> Arbitrary<'a> for CircBuf<I>
where
    I: Arbitrary<'a>,
{
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut circbuf = Self::default();

        let len = u.arbitrary_len::<I>()?;
        for _ in 0..len {
            circbuf.append(I::arbitrary(u)?);
        }

        Ok(circbuf)
    }
}
