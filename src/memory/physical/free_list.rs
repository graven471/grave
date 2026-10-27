use core::hint::likely;

#[cfg(target_arch = "x86_64")]
use crate::x86_64::utils::prefetch_read_t0;

// (todo): rename it to FreeListNode
pub struct FreeBlock {
    pub next: *mut FreeBlock,
}

impl FreeBlock {
    #[allow(unused)]
    pub const fn new() -> Self {
        Self {
            next: core::ptr::null_mut(),
        }
    }

    #[inline(always)]
    pub const unsafe fn push(head: *mut *mut FreeBlock, other: *mut FreeBlock) -> bool {
        debug_assert!(
            !head.is_null() || !other.is_null(),
            "cannot push null values"
        );
        unsafe {
            (*other).next = *head;
            *head = other;
        }
        return true;
    }

    #[inline(always)]
    pub const unsafe fn pop(head: *mut *mut FreeBlock) -> *mut FreeBlock {
        debug_assert!(!head.is_null(), "head is null");
        unsafe {
            let node = *head;
            debug_assert!(!node.is_null(), "node is null");
            *head = (*node).next;
            (*node).next = core::ptr::null_mut();
            return node;
        }
    }

    #[inline]
    pub unsafe fn remove_by_address(
        head: *mut *mut FreeBlock,
        addr: *const FreeBlock,
    ) -> *mut FreeBlock {
        debug_assert!(
            !addr.is_null(),
            "address passed in remove_by_address is null"
        );

        unsafe {
            if head.is_null() || (*head).is_null() {
                return core::ptr::null_mut();
            }

            #[cfg(target_arch = "x86_64")]
            prefetch_read_t0((*head).cast());

            if core::ptr::eq(*head, addr) {
                let node = Self::pop(head);
                debug_assert!(
                    !node.is_null(),
                    "node reuturned by pop is null is remove_by_address"
                );
                return node;
            }

            let mut cur = *head;
            while likely(!cur.is_null()) {
                if core::ptr::eq((*cur).next, addr) {
                    let node_ptr = (*cur).next;
                    (*cur).next = (*node_ptr).next;
                    debug_assert!(!node_ptr.is_null(), "node_ptr is null");
                    return node_ptr;
                }
                cur = (*cur).next;
            }
            return core::ptr::null_mut();
        }
    }
}
