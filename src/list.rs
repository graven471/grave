#![allow(unused)]
use crate::kprintln;

/*
* why a single node must point to itself ?
* The circular-list says that repeatedly following next must eventually return to the starting
* node. Since A  is the only node available, A's sucessor must be A.
* therefore:
*          A.next = A
* reciprocal-link invariant. If A's successor is A, then that successor's predecessor must be A:
*          A.next = A ==> A.next.prev = A
* because A.next is A itself, this simplifies to:
*          A.prev = A
* so both link must point to A:
*          A.next = A.prev = A
* Insertion:
* Insertion doesn't create a linked list from scratch. It changes a small set of relationships in an existing
* graph while preserving the invariants.
*
* suppose we have three nodes, A, B, and C.
* [A prev: C next: B] -> [B prev: A next: C] -> C [prev: B next: A]
* A -> B -> C -> A
*
* we want to insert a new node X between A and B
* we are replacing adjacency relationship A -> B with two relationships:
* - A -> X
* - X -> B
* but because this is a doubly linked list we must also preserve backward relationship.
*
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

macro_rules! list_head_ptr {
    ($head:expr) => {
        core::ptr::addr_of_mut!($head)
    };
}

macro_rules! list_for_each {
    ($pos:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { Cursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.next() } {
            $body
        }
    }};
}

macro_rules! list_for_each_prev {
    ($pos:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { ReverseCursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.prev() } {
            $body
        }
    }};
}

macro_rules! list_for_each_safe {
    ($pos:ident, $n:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { Cursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.next() } {
            let $n = curr.curr;
            $body
        }
    }};
}

macro_rules! list_for_each_prev_safe {
    ($pos:ident, $n:ident, $head:expr, $body:block) => {{
        let mut curr = unsafe { ReverseCursor::new(list_head_ptr!($head)) };
        while let Some($pos) = unsafe { curr.prev() } {
            let $n = curr.curr;
            $body
        }
    }};
}

macro_rules! list_entry {
    ($ptr:expr, $type:ty, $member:ident) => {{
        unsafe {
            &mut *((($ptr as *mut u8).sub(core::mem::offset_of!($type, $member))) as *mut $type)
        }
    }};
}

macro_rules! list_first_entry {
    ($head:expr, $type:ty, $member:ident) => {{ list_entry!(unsafe { (*list_head_ptr!($head)).next }, $type, $member) }};
}

macro_rules! list_last_entry {
    ($head:expr, $type:ty, $member:ident) => {{ list_entry!(unsafe { (*list_head_ptr!($head)).prev }, $type, $member) }};
}

macro_rules! list_for_each_entry {
    ($pos:ident, $head:expr, $type:ty, $member:ident, $body:block) => {{
        list_for_each!(__node, $head, {
            let $pos = list_entry!(__node, $type, $member);
            $body
        });
    }};
}

macro_rules! list_for_each_entry_reverse {
    ($pos:ident, $head:expr, $type:ty, $member:ident, $body:block) => {{
        list_for_each_prev!(__node, $head, {
            let $pos = list_entry!(__node, $type, $member);
            $body
        });
    }};
}

pub struct ListHead {
    prev: *mut ListHead,
    next: *mut ListHead,
}

impl ListHead {
    pub unsafe fn new() -> Self {
        let mut this = Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
        };
        ListHead::init(core::ptr::addr_of_mut!(this));
        return this;
    }

    unsafe fn init(node: *mut ListHead) {
        (*node).prev = node;
        (*node).next = node;
    }

    // for test
    pub const fn uninit_for_test() -> Self {
        Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
        }
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
        //
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

use core::ptr;

// A real container embedding the intrusive link.
#[repr(C)]
struct TestNode {
    value: i32,
    link: ListHead,
}

impl TestNode {
    const fn uninit(value: i32) -> Self {
        Self {
            value,
            link: ListHead {
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            },
        }
    }
}

// These functions must run only after the values have reached
// their final addresses.
unsafe fn init_head(head: *mut ListHead) {
    ListHead::init(head);
}

unsafe fn init_node(node: *mut TestNode) {
    ListHead::init(ptr::addr_of_mut!((*node).link));
}

macro_rules! check {
    ($condition:expr, $($arg:tt)*) => {{
        if !($condition) {
            kprintln!(
                "[FAIL] {}:{}: {}",
                file!(),
                line!(),
                format_args!($($arg)*)
            );
            return false;
        }
    }};
}

macro_rules! test_case {
    ($name:ident, { $($body:tt)* }) => {
        fn $name() -> bool {
            kprintln!("[TEST] {}", stringify!($name));

            let passed = (|| -> bool {
                $($body)*
            })();

            if passed {
                kprintln!("[PASS] {}", stringify!($name));
            } else {
                kprintln!("[FAIL] {}", stringify!($name));
            }

            passed
        }
    };
}

// Verify both directions of every link, including the sentinel.
unsafe fn check_ring(head: *mut ListHead, expected_len: usize) -> bool {
    if head.is_null() {
        return false;
    }

    if (*head).next.is_null() || (*head).prev.is_null() {
        return false;
    }

    let mut count = 0usize;
    let mut node = (*head).next;
    let mut previous = head;

    while node != head {
        if node.is_null() || count >= expected_len {
            return false;
        }

        if (*node).prev != previous {
            return false;
        }

        if (*previous).next != node {
            return false;
        }

        previous = node;
        node = (*node).next;
        count += 1;
    }

    if count != expected_len {
        return false;
    }

    if (*head).prev != previous || (*previous).next != head {
        return false;
    }

    // Independently verify the backward chain.
    count = 0;
    node = (*head).prev;
    let mut next = head;

    while node != head {
        if node.is_null() || count >= expected_len {
            return false;
        }

        if (*node).next != next {
            return false;
        }

        if (*next).prev != node {
            return false;
        }

        next = node;
        node = (*node).prev;
        count += 1;
    }

    count == expected_len
}

// Helpers for the assertions below.
unsafe fn is_empty(head: *mut ListHead) -> bool {
    (*head).next == head && (*head).prev == head
}

unsafe fn node_value(link: *mut ListHead) -> i32 {
    list_entry!(link, TestNode, link).value
}

test_case!(test_forward_reverse_order, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(10);
        let mut b = TestNode::uninit(20);
        let mut c = TestNode::uninit(30);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);

        let mut expected = [10, 20, 30];
        let mut i = 0;

        list_for_each_entry!(pos, head, TestNode, link, {
            check!(i < expected.len(), "forward iterator overran");
            check!(pos.value == expected[i], "forward order mismatch");
            i += 1;
        });

        check!(i == 3, "forward iterator count incorrect");

        // Directly test ReverseCursor too.
        // Expected to expose the original ReverseCursor::prev bug.
        let mut cursor = ReverseCursor::new(&head);
        let mut reverse_expected = [30, 20, 10];
        i = 0;

        while let Some(node) = cursor.prev() {
            check!(i < reverse_expected.len(), "reverse iterator overran");
            check!(
                node_value(node) == reverse_expected[i],
                "reverse order mismatch at index {}",
                i
            );
            i += 1;
        }

        check!(i == 3, "reverse iterator count incorrect");
        check!(check_ring(&mut head, 3), "iterator damaged ring");
    }
    true
});

test_case!(test_cursor_empty_singleton, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(42);

        init_head(&mut head);
        init_node(&mut a);

        let mut forward = Cursor::new(&head);
        check!(
            forward.next().is_none(),
            "empty forward cursor yielded node"
        );

        let mut reverse = ReverseCursor::new(&head);
        check!(
            reverse.prev().is_none(),
            "empty reverse cursor yielded node"
        );

        a.link.add(&mut head);

        let mut forward = Cursor::new(&head);
        let node = forward.next();
        check!(node.is_some(), "singleton forward cursor returned None");
        check!(node.unwrap() == &mut a.link, "wrong singleton node");
        check!(forward.next().is_none(), "forward cursor did not terminate");

        let mut reverse = ReverseCursor::new(&head);
        let node = reverse.prev();
        check!(node.is_some(), "singleton reverse cursor returned None");
        check!(node.unwrap() == &mut a.link, "wrong reverse singleton node");
        check!(reverse.prev().is_none(), "reverse cursor did not terminate");
    }
    true
});

test_case!(test_first_last_entry, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(11);
        let mut b = TestNode::uninit(22);
        let mut c = TestNode::uninit(33);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);

        let first = list_first_entry!(head, TestNode, link);
        let last = list_last_entry!(head, TestNode, link);

        check!(first.value == 11, "first_entry returned wrong value");
        check!(last.value == 33, "last_entry returned wrong value");

        first.value = 111;
        last.value = 333;

        check!(a.value == 111, "first_entry not mutable");
        check!(c.value == 333, "last_entry not mutable");

        check!(check_ring(&mut head, 3), "container access damaged ring");
    }
    true
});

test_case!(test_safe_forward_delete_current, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);
        let mut b = TestNode::uninit(2);
        let mut c = TestNode::uninit(3);
        let mut d = TestNode::uninit(4);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);
        init_node(&mut d);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);
        d.link.add_tail(&mut head);

        let mut visited = 0;

        list_for_each_safe!(pos, next, head, {
            visited += 1;
            (*pos).del_init();
        });

        check!(visited == 4, "safe iteration visited {} nodes", visited);
        check!(is_empty(&mut head), "safe deletion left nodes");
        check!(check_ring(&mut head, 0), "ring invalid after safe deletion");

        check!(
            a.link.next == ptr::addr_of_mut!(a.link),
            "a not reinitialized"
        );
        check!(
            b.link.next == ptr::addr_of_mut!(b.link),
            "b not reinitialized"
        );
        check!(
            c.link.next == ptr::addr_of_mut!(c.link),
            "c not reinitialized"
        );
        check!(
            d.link.next == ptr::addr_of_mut!(d.link),
            "d not reinitialized"
        );
    }
    true
});

test_case!(test_safe_reverse_delete_current, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);
        let mut b = TestNode::uninit(2);
        let mut c = TestNode::uninit(3);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);

        let mut visited = 0;

        list_for_each_prev_safe!(pos, prev, head, {
            visited += 1;
            (*pos).del_init();
        });

        check!(visited == 3, "reverse safe iteration visited {}", visited);
        check!(is_empty(&mut head), "reverse safe deletion left nodes");
        check!(
            check_ring(&mut head, 0),
            "ring invalid after reverse deletion"
        );
    }
    true
});

test_case!(test_safe_forward_delete_alternating, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);
        let mut b = TestNode::uninit(2);
        let mut c = TestNode::uninit(3);
        let mut d = TestNode::uninit(4);
        let mut e = TestNode::uninit(5);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);
        init_node(&mut d);
        init_node(&mut e);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);
        d.link.add_tail(&mut head);
        e.link.add_tail(&mut head);

        let mut visited = 0;

        list_for_each_safe!(pos, next, head, {
            let value = node_value(pos);
            visited += 1;

            if value % 2 == 1 {
                (*pos).del_init();
            }
        });

        check!(visited == 5, "alternating deletion skipped nodes");
        check!(check_ring(&mut head, 2), "wrong number of remaining nodes");

        let mut cursor = Cursor::new(&head);
        check!(
            node_value(cursor.next().unwrap()) == 2,
            "expected node 2 to remain"
        );
        check!(
            node_value(cursor.next().unwrap()) == 4,
            "expected node 4 to remain"
        );
        check!(cursor.next().is_none(), "unexpected remaining node");
    }
    true
});

test_case!(test_repeated_add_del_init, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);

        init_head(&mut head);
        init_node(&mut a);

        let mut iteration = 0;

        while iteration < 1000 {
            a.link.add(&mut head);
            check!(
                check_ring(&mut head, 1),
                "invalid after add at {}",
                iteration
            );

            a.link.del_init();
            check!(
                check_ring(&mut head, 0),
                "invalid after del at {}",
                iteration
            );

            iteration += 1;
        }
    }
    true
});

test_case!(test_repeated_head_tail_moves, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);
        let mut b = TestNode::uninit(2);
        let mut c = TestNode::uninit(3);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);
        init_node(&mut c);

        a.link.add_tail(&mut head);
        b.link.add_tail(&mut head);
        c.link.add_tail(&mut head);

        let mut i = 0;
        while i < 1000 {
            a.link.move_head(&mut head);
            check!(check_ring(&mut head, 3), "bad move_head at {}", i);

            b.link.move_tail(&mut head);
            check!(check_ring(&mut head, 3), "bad move_tail at {}", i);

            c.link.move_head(&mut head);
            check!(check_ring(&mut head, 3), "bad second move_head at {}", i);

            i += 1;
        }
    }
    true
});

test_case!(test_reinsert_after_del_init, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = TestNode::uninit(1);
        let mut b = TestNode::uninit(2);

        init_head(&mut head);
        init_node(&mut a);
        init_node(&mut b);

        a.link.add_tail(&mut head);

        let mut i = 0;
        while i < 100 {
            b.link.add_after(&mut a.link);
            check!(check_ring(&mut head, 2), "invalid after insertion");

            b.link.del_init();
            check!(check_ring(&mut head, 1), "invalid after removal");

            i += 1;
        }

        check!(head.next == &mut a.link, "original node lost");
        check!(head.prev == &mut a.link, "original tail lost");
    }
    true
});

test_case!(test_empty_list, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        init_head(&mut head);

        check!(is_empty(&mut head), "new list must be empty");
        check!(check_ring(&mut head, 0), "empty ring invalid");

        let mut forward = Cursor::new(&head);
        check!(
            forward.next().is_none(),
            "empty forward cursor yielded node"
        );

        let mut reverse = ReverseCursor::new(&head);
        check!(
            reverse.prev().is_none(),
            "empty reverse cursor yielded node"
        );

        check!(!head.is_signular(&head), "empty list must not be singular");
    }

    true
});

test_case!(test_singleton_insert_delete, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut node = ListHead::uninit_for_test();

        init_head(&mut head);
        init_head(&mut node);

        node.add(&mut head);

        check!(!is_empty(&mut head), "singleton unexpectedly empty");
        check!(
            head.next == core::ptr::addr_of_mut!(node),
            "head.next is not node"
        );
        check!(
            head.prev == core::ptr::addr_of_mut!(node),
            "head.prev is not node"
        );
        check!((*head.next).prev == &mut head, "node.prev is incorrect");
        check!((*head.prev).next == &mut head, "node.next is incorrect");
        check!(head.is_signular(&head), "singleton not detected");
        check!(check_ring(&mut head, 1), "singleton ring invalid");

        node.del_init();

        check!(is_empty(&mut head), "list not empty after deletion");
        check!(check_ring(&mut head, 0), "ring invalid after deletion");
        check!(
            node.next == core::ptr::addr_of_mut!(node),
            "node.next not self-linked after del_init"
        );
        check!(
            node.prev == core::ptr::addr_of_mut!(node),
            "node.prev not self-linked after del_init"
        );
    }

    true
});

test_case!(test_add_and_add_tail, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = ListHead::uninit_for_test();
        let mut b = ListHead::uninit_for_test();
        let mut c = ListHead::uninit_for_test();

        init_head(&mut head);
        init_head(&mut a);
        init_head(&mut b);
        init_head(&mut c);

        a.add(&mut head);
        b.add(&mut head);
        c.add_tail(&mut head);

        check!(
            head.next == core::ptr::addr_of_mut!(b),
            "add did not insert at front"
        );
        check!(
            head.prev == core::ptr::addr_of_mut!(c),
            "add_tail did not insert at back"
        );
        check!(b.next == core::ptr::addr_of_mut!(a), "expected b -> a");
        check!(a.next == core::ptr::addr_of_mut!(c), "expected a -> c");

        check!(check_ring(&mut head, 3), "ring invalid after insertion");
    }

    true
});

test_case!(test_add_before_after, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = ListHead::uninit_for_test();
        let mut b = ListHead::uninit_for_test();
        let mut c = ListHead::uninit_for_test();

        init_head(&mut head);
        init_head(&mut a);
        init_head(&mut b);
        init_head(&mut c);

        a.add_tail(&mut head);
        c.add_tail(&mut head);

        // Insert b before c: a <-> b <-> c.
        b.add_before(&mut c);

        check!(a.next == core::ptr::addr_of_mut!(b), "a.next should be b");
        check!(b.prev == core::ptr::addr_of_mut!(a), "b.prev should be a");
        check!(b.next == core::ptr::addr_of_mut!(c), "b.next should be c");
        check!(c.prev == core::ptr::addr_of_mut!(b), "c.prev should be b");
        check!(check_ring(&mut head, 3), "ring invalid after add_before");

        // Detach b, then insert it after a.
        b.del_init();
        check!(check_ring(&mut head, 2), "ring invalid after detaching b");

        b.add_after(&mut a);

        check!(
            a.next == core::ptr::addr_of_mut!(b),
            "a.next should be b after add_after"
        );
        check!(
            b.next == core::ptr::addr_of_mut!(c),
            "b.next should be c after add_after"
        );
        check!(check_ring(&mut head, 3), "ring invalid after add_after");
    }

    true
});

test_case!(test_del_and_del_init, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = ListHead::uninit_for_test();
        let mut b = ListHead::uninit_for_test();
        let mut c = ListHead::uninit_for_test();

        init_head(&mut head);
        init_head(&mut a);
        init_head(&mut b);
        init_head(&mut c);

        a.add_tail(&mut head);
        b.add_tail(&mut head);
        c.add_tail(&mut head);

        // del() unlinks b but does not reset b's own links.
        b.del();

        check!(
            a.next == core::ptr::addr_of_mut!(c),
            "a.next not repaired after del"
        );
        check!(
            c.prev == core::ptr::addr_of_mut!(a),
            "c.prev not repaired after del"
        );
        check!(check_ring(&mut head, 2), "ring invalid after del");

        // del_init() is valid for a node that is already self-linked
        // or linked into a list. Since b was unlinked with del(),
        // reinitialize it before calling del_init().
        ListHead::init(core::ptr::addr_of_mut!(b));
        b.add_tail(&mut head);
        check!(check_ring(&mut head, 3), "ring invalid after reinsertion");

        b.del_init();

        check!(
            b.next == core::ptr::addr_of_mut!(b),
            "b.next not self-linked after del_init"
        );
        check!(
            b.prev == core::ptr::addr_of_mut!(b),
            "b.prev not self-linked after del_init"
        );
        check!(check_ring(&mut head, 2), "ring invalid after del_init");
    }

    true
});

test_case!(test_move_head_tail, {
    unsafe {
        let mut head = ListHead::uninit_for_test();
        let mut a = ListHead::uninit_for_test();
        let mut b = ListHead::uninit_for_test();
        let mut c = ListHead::uninit_for_test();

        init_head(&mut head);
        init_head(&mut a);
        init_head(&mut b);
        init_head(&mut c);

        a.add_tail(&mut head);
        b.add_tail(&mut head);
        c.add_tail(&mut head);

        // Initial order: a, b, c.
        check!(check_ring(&mut head, 3), "initial ring invalid");

        c.move_head(&mut head);

        check!(
            head.next == core::ptr::addr_of_mut!(c),
            "move_head did not put c first"
        );
        check!(check_ring(&mut head, 3), "ring invalid after move_head");

        // Order: c, a, b.
        c.move_tail(&mut head);

        check!(
            head.prev == core::ptr::addr_of_mut!(c),
            "move_tail did not put c last"
        );
        check!(check_ring(&mut head, 3), "ring invalid after move_tail");

        // Move a to the tail.
        a.move_tail(&mut head);

        check!(
            head.prev == core::ptr::addr_of_mut!(a),
            "moving a to tail failed"
        );
        check!(check_ring(&mut head, 3), "ring invalid after moving a");

        // Move the current head to the head again; must remain valid.
        c.move_head(&mut head);

        check!(
            head.next == core::ptr::addr_of_mut!(c),
            "moving existing first node failed"
        );
        check!(check_ring(&mut head, 3), "final ring invalid");
    }

    true
});
pub fn run_list_tests() -> bool {
    let mut passed = 0usize;
    let mut failed = 0usize;

    macro_rules! run {
        ($test:ident) => {
            if $test() {
                passed += 1;
            } else {
                failed += 1;
            }
        };
    }

    run!(test_empty_list);
    run!(test_singleton_insert_delete);
    run!(test_add_and_add_tail);
    run!(test_add_before_after);
    run!(test_del_and_del_init);
    run!(test_move_head_tail);

    run!(test_forward_reverse_order);
    run!(test_cursor_empty_singleton);
    run!(test_first_last_entry);

    run!(test_safe_forward_delete_current);
    run!(test_safe_reverse_delete_current);
    run!(test_safe_forward_delete_alternating);

    run!(test_repeated_add_del_init);
    run!(test_repeated_head_tail_moves);
    run!(test_reinsert_after_del_init);

    kprintln!("================================");
    kprintln!("Intrusive list test results");
    kprintln!("Passed: {}", passed);
    kprintln!("Failed: {}", failed);
    kprintln!("================================");

    failed == 0
}
