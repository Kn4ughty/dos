use core::alloc::GlobalAlloc;
use core::{alloc::Layout, mem, ptr::NonNull};

use x86_64::instructions::interrupts::without_interrupts;

use super::Mutex;

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap, // todo: implement manually
}

impl Default for FixedSizeBlockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedSizeBlockAllocator {
    #[must_use]
    pub const fn new() -> Self {
        FixedSizeBlockAllocator {
            list_heads: [const { None }; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initialize the allocated with the given heap bounds
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the given heap bounds are
    /// valid and the heap is unused. Must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator
                .init(heap_start as *mut u8, heap_size);
        }
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        #[expect(
            clippy::redundant_closure_for_method_calls,
            reason = "colsure is clearer here"
        )]
        self.fallback_allocator
            .allocate_first_fit(layout)
            .map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }
}

unsafe impl GlobalAlloc for Mutex<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        without_interrupts(|| {
            let mut allocator = self.lock();

            let Some(index) = layout_to_size_index(&layout) else {
                return allocator.fallback_alloc(layout);
            };

            #[expect(clippy::single_match_else)]
            match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    core::ptr::from_mut(node) as *mut u8
                }
                None => {
                    let block_size = BLOCK_SIZES[index];
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align).unwrap();
                    allocator.fallback_alloc(layout)
                }
            }
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        without_interrupts(|| {
            let mut allocator = self.lock();

            let Some(index) = layout_to_size_index(&layout) else {
                let ptr = NonNull::new(ptr).unwrap();
                return unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                };
            };

            let new_node = ListNode {
                next: allocator.list_heads[index].take(),
            };

            assert!(
                mem::size_of::<ListNode>() <= BLOCK_SIZES[index],
                "A list node being larger than the block size would create overlap"
            );
            assert!(
                mem::align_of::<ListNode>() <= BLOCK_SIZES[index],
                "incorrect alignment would be bad"
            );

            debug_assert_eq!(
                ptr as usize % mem::align_of::<ListNode>(),
                0,
                "incorrect alignment would be unsafe"
            );

            #[expect(
                clippy::cast_ptr_alignment,
                reason = "safe: pointer came from Layout::from_size_align, and BLOCK_SIZES\
            are powers of two, >= align of listnode + checked"
            )]
            let new_node_ptr = ptr as *mut ListNode;
            unsafe {
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
        });
    }
}

/// Choose appropriate block size for given layout
///
/// Returns and index to the `BLOCK_SIZES` array
fn layout_to_size_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}
