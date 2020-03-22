use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::mem::size_of;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^22 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   

pub struct Allocator {
    // FIXME: Add the necessary fields.
    bin: [LinkedList; 30],
    current: usize,
    end: usize,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        Allocator {
            bin: [LinkedList::new(); 30],
            current: start,
            end: end,
        }
    }
}

fn get_align_index_upper(size: usize) -> usize {
    if size & (size - 1) == 0 {
        get_align_index_below(size)
    } else {
        let mut idx = 1;
        let mut sx = size;
        while (sx >> 1) != 0 {
            sx = sx >> 1;
            idx += 1;
        }
        if idx <=3 {
            0
        } else {
            idx - 3
        }
    }
}

fn get_align_index_below(size: usize) -> usize {
    let mut index = 0;
    let mut sx = size;
    while (sx >> 1) != 0 {
        sx = sx >> 1;
        index += 1;
    }
    if index <=3 {
        0
    } else {
        index - 3
    }
}


fn get_bin_size(index: usize) -> usize {
    let mut index = index + 3;
    let mut size : usize = 1;
    while index > 0 {
        size = size << 1;
        index -= 1;
    }
    size
}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        if layout.size() <= 0 || (layout.align() & (layout.align()- 1)) != 0 || layout.size() >= self.end {
            return ptr::null_mut() as *mut u8;
        }
        let idx = get_align_index_upper(layout.size());

        if idx < 30 {
            for ptr in self.bin[idx].iter_mut() {
                if (ptr.value() as usize) % layout.align() == 0 {
                    return ptr.pop() as *mut u8;
                }
            }

            for i in idx..29 {
                for ptr in self.bin[i].iter_mut() {
                    if (ptr.value() as usize) % layout.align() == 0 {
                        return ptr.pop() as *mut u8;
                    }
                    let bound = (ptr.value() as usize).saturating_add(get_bin_size(i));
                    if align_up(ptr.value() as usize, layout.align()).saturating_add(layout.size()) <= bound {
                        let ptr_1 = align_up(ptr.value() as usize, layout.align());
                        let ptr_2 = align_up(ptr.value() as usize, layout.align()).saturating_add(layout.size());
                        self.bin[get_align_index_below(ptr_1 - (ptr.value() as usize))].push(ptr.value() as *mut usize);
                        if bound != ptr_2 {
                            self.bin[get_align_index_below(bound - ptr_2)].push(ptr_2 as *mut usize);
                        }
                        return ptr_1 as *mut u8;
                    }
                }
            }

            let new_curr = align_up(self.current, layout.align());
            if new_curr.saturating_add(layout.size()) <= self.end {
                // todo: deal with the alignment block
                if new_curr != self.current{
                    self.bin[get_align_index_below(new_curr-self.current)].push(self.current as *mut usize);
                }
                
                let upper_size = get_bin_size(get_align_index_upper(layout.size()));

                self.current = new_curr.saturating_add(upper_size);
                return new_curr as *mut u8;
            }
            ptr::null_mut() as *mut u8
        }
        else {
            ptr::null_mut() as *mut u8
        }        
        // let size = align_up(layout.size().next_power_of_two(), layout.align());
        // let class = size.trailing_zeros() as usize;
        // for i in class..self.bin.len() {
        //     if !self.bin[i].is_empty() {
        //         for j in (class+1..i+1).rev() {
        //             let block = self.bin[j].pop().expect("bigger block should have free space");
        //             unsafe {
        //                 self.bin[j-1].push((block as usize + (1 << (j-1))) as *mut usize);
        //                 self.bin[j-1].push(block);
        //             }
        //         }
        //         let result = self.bin[class].pop().expect("current block should have free space now") as *mut u8;
        //         self.allocated += size;
        //         return result;
        //     }
        // }
        // return core::ptr::null_mut() as *mut u8;
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        

        // let size = ::core::cmp::max(
        //             layout.size().next_power_of_two(),
        //             ::core::cmp::max(layout.align(), size_of::<usize>())
        //         );
        // let class = size.trailing_zeros() as usize;

        // unsafe {
        //     self.bin[class].push(ptr as *mut usize);
        //     let mut current_ptr = ptr as usize;
        //     let mut current_class = class;
        //     loop {
        //         let buddy = current_ptr ^ (1 << current_class);
        //         let mut flag = false;
        //         for block in self.bin[current_class].iter_mut() {
        //             if block.value() as usize == buddy {
        //                 block.pop();
        //                 flag = true;
        //                 break;
        //             }
        //         }
        //         if flag {
        //             self.bin[current_class].pop();
        //             current_ptr = ::core::cmp::min(current_ptr, buddy);
        //             current_class += 1;
        //             self.bin[current_class].push(current_ptr as *mut usize);
        //         }
        //         else {
        //             break;
        //         }
        //     }
        // }
        // self.allocated -= size;

        self.bin[get_align_index_upper(layout.size())].push(ptr as *mut usize);
        let mut size = get_bin_size(get_align_index_upper(layout.size()));
        let mut loc = ptr as usize;
        let mut change = false;
        while change {
            change = false;
            let mut temp = size;
            'outer:
            for i in 0..29 {
                for p in self.bin[i].iter_mut() {
                    if (p.value() as usize).saturating_add(get_bin_size(i)) == loc {
                        temp = size;
                        size = get_bin_size(i) + size;
                        loc = p.pop() as usize;
                        change = true;
                        break 'outer;
                    }
                    else if loc.saturating_add(size) == (p.value() as usize) {
                        temp = size;
                        size = get_bin_size(i) + size;
                        self.bin[get_align_index_below(temp) - 1].pop();
                        p.pop();
                        change = true;
                        break 'outer;
                    }
                }
            }
            if change {
                self.bin[get_align_index_below(size)].push(loc as *mut usize);
            }
        }
    }  
}

// FIXME: Implement `Debug` for `Allocator`.
impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Bin Allocator")
        .finish()
    }
}