use core::hint::unlikely;

#[cfg(target_arch = "x86_64")]
use crate::x86_64::utils::prefetch_read_t0;

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
        if unlikely(head.is_null() || other.is_null()) {
            return false;
        }
        unsafe {
            (*other).next = *head;
            *head = other;
        }
        return true;
    }

    #[inline(always)]
    pub unsafe fn pop(head: *mut *mut FreeBlock) -> *mut FreeBlock {
        unsafe {
            // if unlikely(head.is_null()) {
            //     return None;
            // }

            let node = *head;
            // if unlikely(node.is_null()) {
            //     return None;
            // }

            // if unlikely((node as usize) % core::mem::align_of::<FreeBlock>() != 0) {
            //     kprintln!("CORRUPT FREE LIST: node_ptr={:#x}", node as usize);
            //     return None;
            // }

            *head = (*node).next;
            (*node).next = core::ptr::null_mut();
            // return Some(node);
            return node;
        }
    }

    #[inline]
    pub unsafe fn remove_by_address(
        head: *mut *mut FreeBlock,
        addr: *const FreeBlock,
    ) -> Option<*mut FreeBlock> {
        unsafe {
            if unlikely(head.is_null() || addr.is_null()) {
                return None;
            }

            #[cfg(target_arch = "x86_64")]
            prefetch_read_t0((*head).cast());

            if unlikely((*head).is_null()) {
                return None;
            }

            if core::ptr::eq(*head, addr) {
                // return Self::pop(head);
                let node = Self::pop(head);
                return Some(node);
            }

            let temp = head;

            // todo: need solid proof to use this
            // prefetch_read_t0((*temp).cast());
            // prefetch_read_t0((**temp).next.cast());

            while !(*temp).is_null() {
                if core::ptr::eq((**temp).next, addr) {
                    let node_ptr = (**temp).next;
                    (**temp).next = (*node_ptr).next;
                    return Some(node_ptr);
                }
                *temp = (**temp).next;
            }
            return None;
        }
    }
}
