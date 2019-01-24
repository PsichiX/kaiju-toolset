use core::error::*;
use itertools::Itertools;
use std::fmt;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct Value {
    pub address: usize,
    pub size: usize,
}

impl Value {
    pub fn new(address: usize, size: usize) -> Self {
        Self { address, size }
    }
}

#[derive(Clone)]
pub struct State {
    bytes: Vec<u8>,
    memory_size: usize,
    stack_size: usize,
    memory_free: Vec<(usize, usize)>,
    stack_pos: usize,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("State")
            .field("bytes", &format!("[...; {}]", self.bytes.len()))
            .field("memory_size", &self.memory_size)
            .field("stack_size", &self.stack_size)
            .field("memory_free", &self.memory_free)
            .field("stack_pos", &self.stack_pos)
            .finish()
    }
}

impl State {
    pub fn new(stack_size: usize, memory_size: usize) -> Self {
        Self {
            bytes: vec![0; stack_size + memory_size],
            stack_size,
            memory_size,
            stack_pos: 0,
            memory_free: vec![(0, memory_size)],
        }
    }

    #[inline]
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    #[inline]
    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    #[inline]
    pub fn all_size(&self) -> usize {
        self.stack_size + self.memory_size
    }

    #[inline]
    pub fn stack_pos(&self) -> usize {
        self.stack_pos
    }

    #[inline]
    pub fn stack_free(&self) -> usize {
        self.stack_size - self.stack_pos
    }

    #[inline]
    pub fn memory_free(&self) -> usize {
        self.memory_free.iter().map(|(_, c)| c).sum()
    }

    pub fn stack_push_data<T>(&mut self, value: &T) -> SimpleResult<Value> {
        let size = size_of::<T>();
        if self.stack_pos + size > self.stack_size {
            Err(SimpleError::new(format!(
                "Stack overflow while trying to push {} bytes",
                size
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(self.stack_pos);
                let sp = value as *const T as *const u8;
                copy_nonoverlapping(sp, dp, size);
            }
            self.stack_pos += size;
            Ok(Value::new(self.stack_pos - size, size))
        }
    }

    pub fn stack_push_bytes(&mut self, source: &[u8]) -> SimpleResult<Value> {
        if self.stack_pos + source.len() > self.stack_size {
            Err(SimpleError::new(format!(
                "Stack overflow while trying to push {} bytes",
                source.len()
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(self.stack_pos);
                let sp = source.as_ptr();
                copy_nonoverlapping(sp, dp, source.len());
            }
            self.stack_pos += source.len();
            Ok(Value::new(self.stack_pos - source.len(), source.len()))
        }
    }

    pub fn stack_push_move(&mut self, source: usize, size: usize) -> SimpleResult<Value> {
        if source + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to push move {} bytes from outside of memory",
                size
            )))
        } else if self.stack_pos + size > self.stack_size {
            Err(SimpleError::new(format!(
                "Stack overflow while trying to push {} bytes",
                size
            )))
        } else if source + size > self.stack_pos && source < self.stack_pos + size {
            Err(SimpleError::new(format!(
                "Trying to push {} bytes in same memory fragment",
                size
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(self.stack_pos);
                let sp = self.bytes.as_ptr().add(source);
                copy_nonoverlapping(sp, dp, size);
            }
            self.stack_pos += size;
            Ok(Value::new(self.stack_pos - size, size))
        }
    }

    pub fn stack_pop_bytes(&mut self, size: usize) -> SimpleResult<Vec<u8>> {
        if size > self.stack_pos {
            Err(SimpleError::new(format!(
                "Stack underflow while trying to pop {} bytes",
                size
            )))
        } else {
            self.stack_pos -= size;
            Ok(self.bytes[self.stack_pos..self.stack_pos + size].to_vec())
        }
    }

    pub fn stack_pop_data<T: Default>(&mut self) -> SimpleResult<T> {
        let size = size_of::<T>();
        if size > self.stack_pos {
            Err(SimpleError::new(format!(
                "Stack underflow while trying to pop {} bytes",
                size
            )))
        } else {
            unsafe {
                self.stack_pos -= size;
                let sp = self.bytes.as_ptr().add(self.stack_pos);
                let mut value = T::default();
                let dp = &mut value as *mut T as *mut u8;
                copy_nonoverlapping(sp, dp, size);
                Ok(value)
            }
        }
    }

    pub fn stack_pop_move(&mut self, destination: usize, size: usize) -> SimpleResult<()> {
        if destination + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to pop move {} bytes to outside of memory",
                size
            )))
        } else if size > self.stack_pos {
            Err(SimpleError::new(format!(
                "Stack overflow while trying to pop {} bytes",
                size
            )))
        } else if destination + size > self.stack_pos - size && destination < self.stack_pos {
            Err(SimpleError::new(format!(
                "Trying to pop {} bytes in same memory fragment",
                size
            )))
        } else {
            self.stack_pos -= size;
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(destination);
                let sp = self.bytes.as_ptr().add(self.stack_pos);
                copy_nonoverlapping(sp, dp, size);
            }
            Ok(())
        }
    }

    pub fn stack_reset(&mut self, position: usize) -> SimpleResult<()> {
        if position >= self.stack_size {
            Err(SimpleError::new(format!(
                "Stack overflow while trying to reset to position {}",
                position
            )))
        } else {
            self.stack_pos = position;
            Ok(())
        }
    }

    pub fn memory_move(
        &mut self,
        source: usize,
        size: usize,
        destination: usize,
    ) -> SimpleResult<()> {
        if source + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to move {} bytes from outside of memory",
                size
            )))
        } else if destination + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to move {} bytes to outside of memory",
                size
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(destination);
                let sp = self.bytes.as_ptr().add(source);
                copy_nonoverlapping(sp, dp, size);
            }
            Ok(())
        }
    }

    pub fn store_data<T>(&mut self, destination: usize, value: &T) -> SimpleResult<()> {
        let size = size_of::<T>();
        if destination + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to store {} bytes to outside of memory",
                size
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(destination);
                let sp = value as *const T as *const u8;
                copy_nonoverlapping(sp, dp, size);
            }
            Ok(())
        }
    }

    pub fn store_bytes(&mut self, destination: usize, value: &[u8]) -> SimpleResult<()> {
        let size = value.len();
        if destination + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to store {} bytes to outside of memory",
                size
            )))
        } else {
            unsafe {
                let dp = self.bytes.as_mut_ptr().add(destination);
                let sp = value.as_ptr();
                copy_nonoverlapping(sp, dp, size);
            }
            Ok(())
        }
    }

    pub fn load_data<T: Default>(&self, source: usize) -> SimpleResult<T> {
        let size = size_of::<T>();
        if source + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to load {} bytes from outside of memory",
                size
            )))
        } else {
            unsafe {
                let sp = self.bytes.as_ptr().add(source);
                let mut value = T::default();
                let dp = &mut value as *mut T as *mut u8;
                copy_nonoverlapping(sp, dp, size);
                Ok(value)
            }
        }
    }

    pub fn load_bytes(&self, source: usize, size: usize) -> SimpleResult<Vec<u8>> {
        if source + size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to load {} bytes from outside of memory",
                size
            )))
        } else {
            Ok(self.bytes[source..source + size].to_vec())
        }
    }

    pub fn load_bytes_while<P>(&self, source: usize, mut predicate: P) -> Vec<u8>
    where
        P: FnMut(u8) -> bool,
    {
        self.bytes
            .iter()
            .skip(source)
            .take_while(|b| predicate(**b))
            .cloned()
            .collect()
    }

    pub fn load_bytes_while_non_zero(&self, source: usize) -> Vec<u8> {
        self.load_bytes_while(source, |b| b != 0)
    }

    pub fn map(&self, value: Value) -> SimpleResult<&[u8]> {
        if value.address + value.size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to map {} bytes from outside of memory",
                value.size
            )))
        } else {
            Ok(&self.bytes[value.address..value.address + value.size])
        }
    }

    pub fn map_mut(&mut self, value: Value) -> SimpleResult<&mut [u8]> {
        if value.address + value.size > self.stack_size + self.memory_size {
            Err(SimpleError::new(format!(
                "Trying to map {} bytes from outside of memory",
                value.size
            )))
        } else {
            Ok(&mut self.bytes[value.address..value.address + value.size])
        }
    }

    pub fn map_stack(&self) -> &[u8] {
        &self.bytes[0..self.stack_size]
    }

    pub fn map_stack_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[0..self.stack_size]
    }

    pub fn map_memory(&self) -> &[u8] {
        &self.bytes[self.stack_size..]
    }

    pub fn map_memory_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[self.stack_size..]
    }

    pub fn map_all(&self) -> &[u8] {
        &self.bytes
    }

    pub fn map_all_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    pub fn alloc_stack_value(&mut self, size: usize) -> SimpleResult<Value> {
        let address = self.stack_pos;
        self.stack_push_bytes(&vec![0; size])?;
        Ok(Value { address, size })
    }

    pub fn alloc_memory_value(&mut self, size: usize) -> SimpleResult<Value> {
        let (index, address, s) = self.find_free_memory(size)?;
        if self.memory_free[index].1 == size {
            self.memory_free.remove(index);
        } else {
            self.memory_free[index] = (address + size, s - size);
        }
        Ok(Value {
            address: address + self.stack_size,
            size,
        })
    }

    pub fn dealloc_memory_value(&mut self, value: &Value) -> SimpleResult<()> {
        self.ensure_taken_memory(value)?;
        self.memory_free
            .push((value.address - self.stack_size, value.size));
        self.defragment_free_memory();
        Ok(())
    }

    fn find_free_memory(&self, size: usize) -> SimpleResult<(usize, usize, usize)> {
        if let Some((i, (a, s))) = self
            .memory_free
            .iter()
            .enumerate()
            .find(|(_, (_, s))| size <= *s)
        {
            Ok((i, *a, *s))
        } else {
            Err(SimpleError::new(format!(
                "Could not find free {} bytes in memory",
                size
            )))
        }
    }

    fn ensure_taken_memory(&self, value: &Value) -> SimpleResult<()> {
        let address = value.address - self.stack_size;
        if !self
            .memory_free
            .iter()
            .any(|(a, s)| address >= *a && address + value.size <= *a + *s)
        {
            Ok(())
        } else {
            Err(SimpleError::new(format!(
                "Memory block at {} is free",
                value.address
            )))
        }
    }

    fn defragment_free_memory(&mut self) {
        self.memory_free.sort_by(|a, b| a.0.cmp(&b.0));
        self.memory_free = self
            .memory_free
            .iter()
            .cloned()
            .coalesce(|a, b| {
                if a.0 + a.1 == b.0 {
                    Ok((a.0, a.1 + b.1))
                } else {
                    Err((a, b))
                }
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Value;

    #[test]
    fn test_stack() {
        let mut state = State::new(8, 0);
        assert_eq!(state.stack_pos(), 0);
        assert_eq!(state.stack_size(), 8);

        assert_eq!(
            state.alloc_stack_value(4).unwrap(),
            Value {
                address: 0,
                size: 4
            }
        );
        assert_eq!(
            state.alloc_stack_value(4).unwrap(),
            Value {
                address: 4,
                size: 4
            }
        );
        assert!(state.alloc_stack_value(4).is_err());

        assert_eq!(state.stack_pos(), 8);
        state.stack_pop_bytes(4).unwrap();
        assert_eq!(state.stack_pos(), 4);
        state.stack_pop_bytes(4).unwrap();
        assert_eq!(state.stack_pos(), 0);
    }

    #[test]
    fn test_memory() {
        let mut state = State::new(8, 8);
        assert_eq!(state.memory_free, vec![(0, 8)]);

        assert_eq!(
            state.alloc_memory_value(4).unwrap(),
            Value {
                address: 8,
                size: 4
            }
        );
        assert_eq!(state.memory_free, vec![(4, 4)]);
        assert_eq!(
            state.alloc_memory_value(4).unwrap(),
            Value {
                address: 12,
                size: 4
            }
        );
        assert_eq!(state.memory_free, vec![]);
        assert!(state.alloc_memory_value(4).is_err());

        let mut state = State::new(8, 8);
        let a = state.alloc_memory_value(4).unwrap();
        let b = state.alloc_memory_value(4).unwrap();
        assert_eq!(state.memory_free, vec![]);
        state.dealloc_memory_value(&b).unwrap();
        assert_eq!(state.memory_free, vec![(4, 4)]);
        state.dealloc_memory_value(&a).unwrap();
        assert_eq!(state.memory_free, vec![(0, 8)]);
    }
}
