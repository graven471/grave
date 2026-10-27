/*
* the four relationship must change
* before insertion:
*         A.next = B, B.prev = A
* after insertion:
*         A.next = X, X.prev = A
*         X.next = B, B.prev = X

* A [next: X prev: C] -> X [next: B prev: A] -> B [next: C prev: X]
* -> Continue B -> C -> A to close the cycle
*
* circular boundary:
* head.prev = tail <==> tail.next = head
*
* basic relationship:
* head -> A -> B -> C and C -> B -> A
 A.prev = C
 A.next = B
 B.prev = A
 B.next = C
 C.prev = B
 C.next = A

 note:
 here Head is actual ListHead its not textbook style where Head just references to first Node
 Head.next = first real node
 Head.prev = Tail last real node

 its like an reaction:
X.next = head          <==>  head.prev     =  X
X.prev = old_tail.prev <==>  old_tail.next =  X

if you update prev then you must update next, which also implies if you update next you mut also update prev

&self -> represents new
*/

pub struct Cursor {
    head: *const ListHead,
    curr: *mut ListHead,
}

pub struct ReverseCursor {
    head: *const ListHead,
    curr: *mut ListHead,
}

impl Cursor {
    pub unsafe fn new(head: *const ListHead) -> Self {
        debug_assert!(!(*head).next.is_null(), "head points to null");
        Self {
            head: head,
            curr: (*head).next,
        }
    }
    pub unsafe fn next(&mut self) -> Option<*mut ListHead> {
        if core::ptr::eq(self.curr, self.head) {
            return None;
        }
        let curr = self.curr;
        self.curr = (*curr).next;
        return Some(curr);
    }
}

impl ReverseCursor {
    pub unsafe fn new(head: *const ListHead) -> Self {
        debug_assert!(
            !(*head).prev.is_null(),
            "this is not possible in circular list"
        );
        Self {
            head,
            curr: (*head).prev,
        }
    }
    pub unsafe fn prev(&mut self) -> Option<*mut ListHead> {
        if core::ptr::eq(self.curr, self.head) {
            return None;
        }
        let curr = self.curr;
        self.curr = (*curr).prev;
        return Some(curr);
    }
}

pub struct ListHead {
    prev: *mut ListHead,
    next: *mut ListHead,
}

impl ListHead {
    pub fn new() -> Self {
        Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
        }
    }

    pub unsafe fn init(node: *mut ListHead) {
        (*node).prev = node;
        (*node).next = node;
    }

    pub unsafe fn is_empty(&self, head: *const Self) -> bool {
        core::ptr::eq((*head).next, head)
    }

    /// inserts new between two existing nodes: prev and next
    /// self: new node being inserted
    /// head: the node that should come immediately before self
    /// next: the node that should come immediately after self
    unsafe fn __add(&mut self, prev: *mut Self, next: *mut Self) {
        // insert self between prev and next
        // the core rule: if A.next = B then B.prev = A
        // A.next = B <==> B.prev = A
        // to insert a node between A and B:
        // X.prev = A, X.next = B
        // A.next = X, B.prev = X
        // self.prev = next <==> self.next = next
        // prev.next = self <==> next.prev = self
        self.prev = prev;
        self.next = next;
        (*prev).next = self as *mut ListHead;
        (*next).prev = self as *mut ListHead;
    }

    /// inserts new immediately after head, at the beginning of the list
    /// self: represents new
    /// head: represents the list head
    pub unsafe fn add(&mut self, head: *mut Self) {
        // so to insert X at head we want:
        // X.next    = head       X.prev = head.prev
        // head.prev = X
        // head never changes:
        // X.prev         = head,       X.next    = head.next
        // head.next.prev = X,          head.next = X

        self.__add(head, (*head).next);
    }

    pub unsafe fn add_tail(&mut self, head: *mut Self) {
        // X.next = head          <==>  head.prev     =  X
        // X.prev = old_tail      <==>  old_tail.next =  X
        self.__add((*head).prev, head);
    }

    pub unsafe fn add_before(&mut self, entry: *mut Self) {
        // insert node before entry
        // if self = A, and entry = B then i am expecting this
        // A -> B, A <- B
        // self.next = entry <==> entry.prev = self
        self.__add((*entry).prev, entry);
    }

    pub unsafe fn add_after(&mut self, entry: *mut Self) {
        // insert node after entry
        // if self = A, and entry = B then i am expecting this
        // B -> A, B <- A
        // entry.next = self <==> self.prev = entry
        self.__add(entry, (*entry).next);
    }

    pub unsafe fn del(&mut self) {
        // self.next.prev = self.prev <==> self.prev.next = self.next
        (*self.next).prev = self.prev;
        (*self.prev).next = self.next
    }

    pub unsafe fn del_init(&mut self) {
        // self.next.prev = self.prev <==> self.prev.next = self.next
        // self.prev = self <==> self.next = self
        self.del();
        ListHead::init(self);
    }

    pub unsafe fn move_head(&mut self, head: *mut Self) {
        self.del();
        self.add(head);
    }
    pub unsafe fn move_tail(&mut self, head: *mut Self) {
        self.del();
        self.add_tail(head);
    }

    pub unsafe fn is_signular(&self, head: *const Self) -> bool {
        core::ptr::eq((*head).next, (*head).prev) && !self.is_empty(head)
    }
}

#[macro_export]
macro_rules! list_head_ptr {
    ($head:expr) => {
        core::ptr::addr_of_mut!($head)
    };
}

#[macro_export]
macro_rules! list_for_each {
    ($pos:ident, $head:expr, $body:block) => {{
        let mut curr =
            unsafe { crate::collections::intrusive_list::Cursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.next() } {
            $body
        }
    }};
}

#[macro_export]
macro_rules! list_for_each_prev {
    ($pos:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { ReverseCursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.prev() } {
            $body
        }
    }};
}

#[macro_export]
macro_rules! list_for_each_safe {
    ($pos:ident, $n:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { Cursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.next() } {
            let $n = curr.curr;
            $body
        }
    }};
}

#[macro_export]
macro_rules! list_for_each_prev_safe {
    ($pos:ident, $n:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { ReverseCursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.prev() } {
            let $n = curr.curr;
            $body
        }
    }};
}

#[macro_export]
macro_rules! list_entry {
    ($ptr:expr, $type:ty, $member:ident) => {{
        unsafe {
            &mut *((($ptr as *mut u8).sub(core::mem::offset_of!($type, $member))) as *mut $type)
        }
    }};
}

#[macro_export]
macro_rules! list_first_entry {
    ($head:expr, $type:ty, $member:ident) => {{ list_entry!(unsafe { (*list_head_ptr!($head)).next }, $type, $member) }};
}

#[macro_export]
macro_rules! list_last_entry {
    ($head:expr, $type:ty, $member:ident) => {{ list_entry!(unsafe { (*list_head_ptr!($head)).prev }, $type, $member) }};
}

#[macro_export]
macro_rules! list_for_each_entry {
    ($pos:ident, $head:expr, $type:ty, $member:ident, $body:block) => {{
        list_for_each!(__node, $head, {
            let $pos = list_entry!(__node, $type, $member);
            $body
        });
    }};
}

#[macro_export]
macro_rules! list_for_each_entry_reverse {
    ($pos:ident, $head:expr, $type:ty, $member:ident, $body:block) => {{
        list_for_each_prev!(__node, $head, {
            let $pos = list_entry!(__node, $type, $member);
            $body
        });
    }};
}
