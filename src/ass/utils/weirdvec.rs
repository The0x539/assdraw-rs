//! This is all unused and pretty weird, but it's 100% safe and should work as intended.

use std::ops::{Deref, DerefMut};

pub trait GenericVec<'a, T: 'a>:
    IntoIterator<Item = T> + Deref<Target = [T]> + DerefMut<Target = [T]>
where
    &'a Self: IntoIterator<Item = &'a T> + 'a,
    &'a mut Self: IntoIterator<Item = &'a mut T> + 'a,
{
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;

    fn push(&mut self, val: T) -> Result<(), T>;
    fn pop(&mut self) -> Option<T>;

    fn iter(&'a self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
    fn iter_mut(&'a mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<'a, T: 'a> GenericVec<'a, T> for Vec<T> {
    fn len(&self) -> usize {
        self.len()
    }
    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn push(&mut self, val: T) -> Result<(), T> {
        self.push(val);
        Ok(())
    }
    fn pop(&mut self) -> Option<T> {
        self.pop()
    }
}

pub struct BorrowVec<'a, T> {
    data: &'a mut [T],
    len: usize,
}

impl<'a, T: Copy> IntoIterator for BorrowVec<'a, T> {
    type Item = T;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.data[..self.len].iter().copied()
    }
}

impl<'a, T: Copy> IntoIterator for &'a BorrowVec<'a, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self[..].iter()
    }
}

impl<'a, T: Copy> IntoIterator for &'a mut BorrowVec<'a, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self[..].iter_mut()
    }
}

impl<T> Deref for BorrowVec<'_, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data[..self.len]
    }
}

impl<T> DerefMut for BorrowVec<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[..self.len]
    }
}

impl<'a, T: Copy> GenericVec<'a, T> for BorrowVec<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
    fn capacity(&self) -> usize {
        self.data.len()
    }

    fn push(&mut self, val: T) -> Result<(), T> {
        if self.len() == self.capacity() {
            Err(val)
        } else {
            self.data[self.len] = val;
            self.len += 1;
            Ok(())
        }
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            let val = self.data[self.len - 1];
            self.len -= 1;
            Some(val)
        }
    }
}

impl<'a, T: Copy> BorrowVec<'a, T> {
    pub fn from_empty(data: &'a mut [T]) -> Self {
        Self { data, len: 0 }
    }

    pub fn split(&'a mut self) -> (&'a mut [T], BorrowVec<'a, T>) {
        let (left, right) = self.data.split_at_mut(self.len());
        (left, Self::from_empty(right))
    }

    pub fn clone_mut(&'a mut self) -> Self {
        Self {
            data: &mut self.data[..],
            len: self.len,
        }
    }
}
